#![allow(dead_code)]

use std::rc::Rc;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use craft_theme::{Appearance, Contrast, Mode, TextSize, builtins, resolve};
use craft_ui_render_epaint::{EpaintOptions, EpaintRenderer, FontDefinitions};
use craft_ui_rt::{
    BuildCx, Constraints, Element, LayoutCx, PaintCx, ReadBack, Renderer, RgbaImage, Runtime, Show, SizeU, SurfaceHandle, Tokens, Vec2, WindowId,
};
use epaint::text::{FontData, FontFamily};

pub const WINDOW: WindowId = WindowId(1);

pub fn tokens() -> Tokens {
    let appearance = Appearance {
        mode: Mode::Dark,
        contrast: Contrast::Standard,
        size: craft_theme::Size::Compact,
        text_size: TextSize::Small,
        reduced_motion: false,
    };
    resolve(&builtins()[0], appearance, None).tokens
}

/// Ubuntu Light for the proportional family and Hack for the monospace one: epaint's bundled
/// faces stand in for craft's font stack, which lives above this crate.
pub fn fonts() -> FontDefinitions {
    let mut d = FontDefinitions::empty();
    d.font_data.insert("ubuntu".into(), Arc::new(FontData::from_static(epaint_default_fonts::UBUNTU_LIGHT)));
    d.font_data.insert("hack".into(), Arc::new(FontData::from_static(epaint_default_fonts::HACK_REGULAR)));
    d.families.insert(FontFamily::Proportional, vec!["ubuntu".into()]);
    d.families.insert(FontFamily::Monospace, vec!["hack".into()]);
    d
}

/// Held for a whole test that drives a wgpu device: lavapipe crashes when several threads drive
/// devices at once, and the test binary runs tests in parallel.
pub struct Gpu(#[allow(dead_code)] MutexGuard<'static, ()>);

pub fn gpu() -> Gpu {
    static GPU: Mutex<()> = Mutex::new(());
    Gpu(GPU.lock().unwrap_or_else(PoisonError::into_inner))
}

pub fn renderer(gpu: &Gpu) -> EpaintRenderer {
    renderer_with(gpu, EpaintOptions::predictable())
}

pub fn renderer_with(_: &Gpu, options: EpaintOptions) -> EpaintRenderer {
    EpaintRenderer::create_blocking(options, fonts(), None).expect("a wgpu adapter (lavapipe will do)")
}

type PaintFn = Rc<dyn Fn(&mut PaintCx<'_>)>;

pub fn paint(f: impl Fn(&mut PaintCx<'_>) + 'static) -> PaintFn {
    Rc::new(f)
}

/// A node of a fixed size that paints with a closure; reads inside the closure are tracked, so
/// the node repaints when they change.
pub struct Painted {
    pub id: u64,
    pub size: Vec2,
    pub paint: PaintFn,
}

impl Element for Painted {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        self.id
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn layout(&self, _: &mut (), _: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        c.constrain(self.size)
    }

    fn paint(&self, _: &(), cx: &mut PaintCx<'_>) {
        (self.paint)(cx);
    }
}

/// The window's root: places each child of `kids` at its offset.
pub struct Stage {
    pub kids: Vec<(Vec2, Painted)>,
}

impl Element for Stage {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        0
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn build(&self, _: &mut (), cx: &mut BuildCx<'_>) {
        for (_, kid) in &self.kids {
            Painted { id: kid.id, size: kid.size, paint: kid.paint.clone() }.show(cx);
        }
    }

    fn layout(&self, _: &mut (), cx: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        for (i, (at, kid)) in self.kids.iter().enumerate() {
            let child = cx.child_at(i);
            cx.layout_child(child, Constraints::tight(kid.size));
            cx.place_child(child, *at);
        }
        c.max
    }
}

/// A runtime with one offscreen window drawn by one renderer.
pub struct Scene {
    pub rt: Runtime,
    pub r: EpaintRenderer,
    pub size: SizeU,
    /// The damage the last frame handed to the renderer, in logical points.
    pub damage: Vec<craft_ui_rt::Rect>,
}

impl Scene {
    /// `make` builds the root; it runs with the runtime (to create state cells) and the
    /// renderer (to lay out text).
    pub fn new<E: Element>(mut r: EpaintRenderer, size: SizeU, scale: f32, make: impl FnOnce(&mut Runtime, &mut EpaintRenderer) -> E) -> Scene {
        let mut rt = Runtime::new(tokens());
        rt.add_window(WINDOW, size.to_logical(scale), scale);
        let root = make(&mut rt, &mut r);
        rt.set_root(WINDOW, root);
        r.set_target(WINDOW, SurfaceHandle::Offscreen, size, scale);
        Scene { rt, r, size, damage: Vec::new() }
    }

    /// One frame as the driver runs it. Returns what the renderer reported for the window.
    pub fn frame(&mut self) -> craft_ui_rt::RenderStats {
        self.rt.frame(self.r.text());
        self.r.sync_images(self.rt.images());
        let mut stats = craft_ui_rt::RenderStats::default();
        self.damage = self.rt.damage(WINDOW).to_vec();
        if self.rt.has_damage(WINDOW) {
            stats = self.r.render(WINDOW, &self.rt.frame_tree(WINDOW), self.rt.damage(WINDOW));
        }
        self.rt.end_frame();
        stats
    }

    pub fn pixels(&mut self) -> RgbaImage {
        let id = self.r.request_read_back(WINDOW, None);
        for _ in 0..10_000 {
            match self.r.poll_read_back(id) {
                ReadBack::Ready(image) => return image,
                ReadBack::Pending => std::thread::sleep(std::time::Duration::from_millis(1)),
                ReadBack::Failed => panic!("read-back failed"),
            }
        }
        panic!("read-back never finished");
    }
}

pub fn px(image: &RgbaImage, x: u32, y: u32) -> [u8; 4] {
    let at = (y as usize * image.size.width as usize + x as usize) * 4;
    image.pixels[at..at + 4].try_into().expect("four bytes")
}

pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> craft_theme::Rgba {
    craft_theme::Rgba([r, g, b, a])
}
