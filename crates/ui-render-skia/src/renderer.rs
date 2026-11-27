use std::collections::{HashMap, HashSet};

use craft_theme::Rgba;
use craft_ui_rt::{
    Clip, FrameTree, ImageStore, Media, NodeId, Part, ReadBack, ReadBackId, RectU, RenderStats, Renderer, SizeU, SurfaceHandle, SurfaceKey, WindowId,
    damage_pixels,
};
use emath::{Rect, Vec2};
use skia_safe::{AlphaType, ClipOp, Color4f, ColorType, IRect, ImageInfo, Matrix, Picture, RRect, Region, Surface, Vector, surfaces};

use crate::images::Images;
use crate::picture::{Recorded, record};
use crate::surface::{Ingested, SurfaceProducer, SurfaceSpec, Surfaces};
use crate::text::SkiaText;
use crate::window::{NativeWindow, SoftPresenter};

#[derive(Clone, Debug)]
pub struct SkiaOptions {
    /// What a frame starts from, straight alpha.
    pub clear: Rgba,
    /// Draw every pixel every frame, ignoring the damage. For checking that damage-only frames
    /// give the same bytes.
    pub full_redraw: bool,
    /// GPU backend: bracket each frame's submission with GPU timestamps and wait for them, which
    /// fills [`DrawnFrame::gpu_ms`]. For benches; the CPU backend ignores it.
    pub gpu_timing: bool,
}

impl Default for SkiaOptions {
    fn default() -> Self {
        SkiaOptions { clear: Rgba([0, 0, 0, 0]), full_redraw: false, gpu_timing: false }
    }
}

/// What the last `render` drew, beyond [`RenderStats`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DrawnFrame {
    /// Pixels inside the damage rects, each counted once.
    pub pixels: u64,
    /// Playing the pictures back into the damage, in milliseconds of CPU: rasterising on the CPU
    /// backend, turning the draws into GPU operations on the GPU backend.
    pub raster_ms: f64,
    /// Handing the canvas to the window (softbuffer copy and present, or the GPU submission and
    /// present), when the target is a window; the GPU flush and submit offscreen.
    pub submit_ms: f64,
    /// GPU time of the frame's submission, with `gpu_timing`.
    pub gpu_ms: Option<f64>,
}

/// A node part's recorded picture and what it was recorded from.
struct Kept {
    version: u64,
    opacity_bits: u32,
    scale_bits: u32,
    rec: Recorded,
}

/// Where the pixels are drawn.
enum Engine {
    /// A CPU raster surface.
    Cpu,
    /// Ganesh on the platform's GPU API.
    #[cfg(feature = "gpu")]
    Gpu(craft_ui_render_skia_gpu::Gpu),
}

/// The window the canvas is shown in.
enum Presenter {
    Soft(SoftPresenter),
    #[cfg(feature = "gpu")]
    Gpu(craft_ui_render_skia_gpu::Presenter, NativeWindow),
}

impl Presenter {
    fn is_for(&self, native: &NativeWindow) -> bool {
        let mine = match self {
            Presenter::Soft(p) => &p.window,
            #[cfg(feature = "gpu")]
            Presenter::Gpu(_, w) => w,
        };
        std::ptr::addr_eq(std::sync::Arc::as_ptr(&mine.0), std::sync::Arc::as_ptr(&native.0))
    }
}

struct Target {
    size_px: SizeU,
    scale: f32,
    /// The frame's pixels, kept from frame to frame.
    canvas: Surface,
    presenter: Option<Presenter>,
}

/// Skia with kept pictures on a kept CPU raster or GPU surface per window. See the crate docs.
pub struct SkiaRenderer {
    options: SkiaOptions,
    text: SkiaText,
    images: Images,
    surfaces: Surfaces,
    /// Pictures by (node, part): node ids are unique across windows.
    kept: HashMap<(NodeId, Part), Kept>,
    stash: HashMap<WindowId, Presenter>,
    reads: HashMap<ReadBackId, ReadBack>,
    next_read: u64,
    drawn: DrawnFrame,
    ingested: Ingested,
    // Last: the GPU surfaces and the windows' swapchains above go before the device.
    targets: HashMap<WindowId, Target>,
    engine: Engine,
}

impl SkiaRenderer {
    /// The CPU raster backend.
    pub fn new(options: SkiaOptions) -> SkiaRenderer {
        Self::with_engine(options, Engine::Cpu)
    }

    /// The GPU backend: Metal on macOS, Vulkan elsewhere. `first` is the window the device must
    /// be able to present to; `None` (or `Offscreen`) picks any device, for headless runs.
    #[cfg(feature = "gpu")]
    pub fn new_gpu(options: SkiaOptions, first: Option<&SurfaceHandle>) -> Result<SkiaRenderer, String> {
        let window = match first {
            Some(SurfaceHandle::Native(any)) => {
                let native = any.downcast_ref::<NativeWindow>().ok_or("the native surface is not a NativeWindow")?;
                Some(native.window_ref().ok_or("the window has no raw handles")?)
            }
            _ => None,
        };
        let gpu = craft_ui_render_skia_gpu::Gpu::new(window.as_ref())?;
        Ok(Self::with_engine(options, Engine::Gpu(gpu)))
    }

    fn with_engine(options: SkiaOptions, engine: Engine) -> SkiaRenderer {
        SkiaRenderer {
            options,
            text: SkiaText::new(),
            images: Images::default(),
            surfaces: Surfaces::default(),
            kept: HashMap::new(),
            stash: HashMap::new(),
            reads: HashMap::new(),
            next_read: 0,
            drawn: DrawnFrame::default(),
            ingested: Ingested::default(),
            targets: HashMap::new(),
            engine,
        }
    }

    /// Replaces the text engine (an app that registers its faces up front, or tests that want no
    /// system fonts).
    pub fn with_text(mut self, text: SkiaText) -> SkiaRenderer {
        self.text = text;
        self
    }

    pub fn last_frame(&self) -> DrawnFrame {
        self.drawn
    }

    /// What the last surface ingest filled (pixels and time), for measurements.
    pub fn last_ingest(&self) -> Ingested {
        self.ingested
    }

    /// Registers the producer of surface `key`; the next frame fills every tile.
    pub fn register_surface(&mut self, key: SurfaceKey, spec: SurfaceSpec, producer: Box<dyn SurfaceProducer>) {
        self.surfaces.register(key, spec, producer);
        self.drop_pictures_of(&HashSet::from([Media::Surface(key)]));
    }

    pub fn remove_surface(&mut self, key: SurfaceKey) {
        if self.surfaces.remove(key) {
            self.drop_pictures_of(&HashSet::from([Media::Surface(key)]));
        }
    }

    /// `rect` (surface pixels; the whole surface when `None`) of surface `key` changed. Call it
    /// together with `Runtime::invalidate_surface`, which damages the item.
    pub fn invalidate_surface(&mut self, key: SurfaceKey, rect: Option<RectU>) {
        self.surfaces.invalidate(key, rect);
    }

    fn drop_pictures_of(&mut self, changed: &HashSet<Media>) {
        if !changed.is_empty() {
            self.kept.retain(|_, k| k.rec.uses.is_disjoint(changed));
        }
    }

    /// Ingests pending surface changes: the pictures that show them are dropped first, so Skia
    /// writes the tiles in place.
    fn ingest_surfaces(&mut self, mut changed: HashSet<Media>) {
        changed.extend(self.surfaces.pending());
        self.drop_pictures_of(&changed);
        if self.surfaces.has_pending() {
            self.ingested = self.surfaces.ingest(&mut changed);
        }
    }

    fn presenter(&mut self, window: WindowId, native: &NativeWindow, size: SizeU) -> Option<Presenter> {
        let kept = self.stash.remove(&window).filter(|p| p.is_for(native));
        let mut p = match kept {
            Some(p) => p,
            None => match &mut self.engine {
                Engine::Cpu => Presenter::Soft(SoftPresenter::new(native.clone())?),
                #[cfg(feature = "gpu")]
                Engine::Gpu(gpu) => Presenter::Gpu(gpu.presenter(native.window_ref()?)?, native.clone()),
            },
        };
        match &mut p {
            Presenter::Soft(s) => s.resize(size).then_some(p),
            #[cfg(feature = "gpu")]
            Presenter::Gpu(g, _) => {
                g.resize(size);
                Some(p)
            }
        }
    }

    fn read(target: &mut Target, rect: Option<RectU>) -> ReadBack {
        let whole = RectU::of_size(target.size_px);
        let r = rect.map_or(whole, |r| RectU::new(r.x0.min(whole.x1), r.y0.min(whole.y1), r.x1.min(whole.x1), r.y1.min(whole.y1)));
        if r.is_empty() {
            return ReadBack::Failed;
        }
        let mut out = craft_ui_rt::RgbaImage::new(SizeU::new(r.width(), r.height()));
        let info = ImageInfo::new((r.width() as i32, r.height() as i32), ColorType::RGBA8888, AlphaType::Unpremul, None);
        if target.canvas.read_pixels(&info, &mut out.pixels, r.width() as usize * 4, (r.x0 as i32, r.y0 as i32)) {
            ReadBack::Ready(out)
        } else {
            log::error!("skia read-back failed");
            ReadBack::Failed
        }
    }
}

fn irect(r: RectU) -> IRect {
    IRect::new(r.x0 as i32, r.y0 as i32, r.x1 as i32, r.y1 as i32)
}

/// `rect` (logical) in pixels with each edge rounded to the nearest pixel, as a scissor rounds.
fn pixel_clip(rect: Rect, scale: f32) -> skia_safe::Rect {
    let px = |v: f32| (v * scale).round();
    skia_safe::Rect::new(px(rect.min.x), px(rect.min.y), px(rect.max.x), px(rect.max.y))
}

fn clip_rounded(canvas: &skia_safe::Canvas, clip: &Clip, scale: f32) {
    let r = clip.radii;
    let g = |v: f32| Vector::new(v * scale, v * scale);
    let rect = skia_safe::Rect::new(clip.rect.min.x * scale, clip.rect.min.y * scale, clip.rect.max.x * scale, clip.rect.max.y * scale);
    canvas.clip_rrect(RRect::new_rect_radii(rect, &[g(r.top_left), g(r.top_right), g(r.bottom_right), g(r.bottom_left)]), ClipOp::Intersect, true);
}

/// One picture to play back: where it goes and what clips it, in window coordinates.
struct Play {
    picture: Picture,
    origin: Vec2,
    clip: Rect,
    rounded: Vec<Clip>,
}

fn clear_color(c: Rgba) -> Color4f {
    let [r, g, b, a] = c.0;
    Color4f::new(f32::from(r) / 255.0, f32::from(g) / 255.0, f32::from(b) / 255.0, f32::from(a) / 255.0)
}

impl Renderer for SkiaRenderer {
    type Text = SkiaText;

    fn text(&mut self) -> &mut SkiaText {
        &mut self.text
    }

    /// Pictures are recorded in logical points and played back under the scale, so a new scale
    /// keeps them.
    fn set_target(&mut self, window: WindowId, target: SurfaceHandle, size_px: SizeU, scale: f32) {
        if let Some(old) = self.targets.remove(&window)
            && let Some(p) = old.presenter
        {
            self.stash.insert(window, p);
        }
        if size_px.width == 0 || size_px.height == 0 {
            return;
        }
        let canvas = match &mut self.engine {
            Engine::Cpu => surfaces::raster_n32_premul((size_px.width as i32, size_px.height as i32)),
            #[cfg(feature = "gpu")]
            Engine::Gpu(gpu) => gpu.surface(size_px),
        };
        let Some(canvas) = canvas else {
            log::error!("skia could not make a {}x{} surface", size_px.width, size_px.height);
            return;
        };
        let presenter = match &target {
            SurfaceHandle::Offscreen => None,
            SurfaceHandle::Native(any) => match any.downcast_ref::<NativeWindow>() {
                Some(native) => self.presenter(window, native, size_px),
                None => {
                    log::error!("the native surface is not a NativeWindow");
                    None
                }
            },
        };
        self.targets.insert(window, Target { size_px, scale, canvas, presenter });
    }

    fn lose_target(&mut self, window: WindowId) {
        self.targets.remove(&window);
        self.stash.remove(&window);
    }

    fn sync_images(&mut self, images: &ImageStore) -> u32 {
        self.surfaces.begin_frame();
        let mut changed = HashSet::new();
        let built = self.images.sync(images, &mut changed);
        self.ingest_surfaces(changed);
        built
    }

    fn render(&mut self, window: WindowId, frame: &FrameTree<'_>, damage: &[Rect]) -> RenderStats {
        let mut stats = RenderStats::default();
        self.drawn = DrawnFrame::default();
        for &key in &frame.changes().surfaces {
            self.surfaces.invalidate_unless_marked(key);
        }
        if self.surfaces.has_pending() {
            self.ingest_surfaces(HashSet::new());
        }
        for id in &frame.changes().removed {
            self.kept.remove(&(*id, Part::Content));
            self.kept.remove(&(*id, Part::Overlay));
        }
        let Some(target) = self.targets.get_mut(&window) else {
            return stats;
        };
        let all = [Rect::from_min_size(emath::Pos2::ZERO, frame.viewport())];
        let whole = self.options.full_redraw;
        let damage = damage_pixels(if whole { &all[..] } else { damage }, target.scale, target.size_px);
        if damage.is_empty() {
            return stats;
        }
        let logical: Vec<Rect> = damage.iter().map(|r| r.to_logical(target.scale)).collect();

        let t = web_time::Instant::now();
        let mut plays: Vec<Play> = Vec::new();
        let scale = target.scale;
        let (kept, images, surfaces) = (&mut self.kept, &self.images, &self.surfaces);
        frame.walk(|d| {
            stats.draws += 1;
            if !logical.iter().any(|r| r.intersects(d.ink)) {
                return;
            }
            let bits = d.opacity.to_bits();
            let stale = |k: &Kept| k.version != d.version || k.opacity_bits != bits || (k.rec.scale_sensitive && k.scale_bits != scale.to_bits());
            if kept.get(&(d.node, d.part)).is_none_or(stale) {
                // The whole list's ink, not the clipped `d.ink`: the picture outlives this
                // frame's clip, and Skia culls playback by the recorded bounds.
                let bounds = d.list.ink().unwrap_or(Rect::NOTHING);
                let rec = record(d.list, d.opacity, scale, images, surfaces, bounds);
                kept.insert((d.node, d.part), Kept { version: d.version, opacity_bits: bits, scale_bits: scale.to_bits(), rec });
                stats.prepared += 1;
            } else {
                stats.reused += 1;
            }
            if let Some(p) = kept.get(&(d.node, d.part)).and_then(|k| k.rec.picture.clone()) {
                plays.push(Play { picture: p, origin: d.origin, clip: d.clip, rounded: d.rounded.to_vec() });
            }
        });
        stats.prepare_ms = t.elapsed().as_secs_f64() * 1000.0;

        let t = web_time::Instant::now();
        let mut region = Region::new();
        let rects: Vec<IRect> = damage.iter().copied().map(irect).collect();
        region.set_rects(&rects);
        let canvas = target.canvas.canvas();
        canvas.save();
        canvas.reset_matrix();
        // Every rect is whole pixels and the clip is hard, so a pixel inside the damage gets the
        // bytes a full redraw gives it.
        canvas.clip_region(&region, ClipOp::Intersect);
        canvas.clear(clear_color(self.options.clear));
        for p in &plays {
            canvas.save();
            canvas.clip_rect(pixel_clip(p.clip, scale), ClipOp::Intersect, false);
            for c in &p.rounded {
                clip_rounded(canvas, c, scale);
            }
            let mut m = Matrix::scale((scale, scale));
            m.pre_translate((p.origin.x, p.origin.y));
            canvas.draw_picture(&p.picture, Some(&m), None);
            canvas.restore();
        }
        canvas.restore();
        let raster_ms = t.elapsed().as_secs_f64() * 1000.0;
        let t = web_time::Instant::now();
        self.drawn = DrawnFrame { pixels: damage.iter().map(|r| r.area()).sum(), raster_ms, submit_ms: 0.0, gpu_ms: None };
        match &mut self.engine {
            Engine::Cpu => {
                if let Some(Presenter::Soft(p)) = target.presenter.as_mut() {
                    p.present(&mut target.canvas, target.size_px, &damage);
                }
            }
            #[cfg(feature = "gpu")]
            Engine::Gpu(gpu) => {
                let presenter = match target.presenter.as_mut() {
                    Some(Presenter::Gpu(p, _)) => Some(p),
                    _ => None,
                };
                let sent = gpu.submit(&mut target.canvas, presenter, self.options.gpu_timing);
                self.drawn.gpu_ms = sent.gpu_ms;
                self.drawn.submit_ms = sent.submit_ms;
                return stats;
            }
        }
        self.drawn.submit_ms = t.elapsed().as_secs_f64() * 1000.0;
        stats
    }

    /// The pixels are read at the request (both backends read synchronously), so the first poll
    /// returns them.
    fn request_read_back(&mut self, window: WindowId, rect: Option<RectU>) -> ReadBackId {
        let id = ReadBackId(self.next_read);
        self.next_read += 1;
        let result = match self.targets.get_mut(&window) {
            Some(target) => Self::read(target, rect),
            None => ReadBack::Failed,
        };
        self.reads.insert(id, result);
        id
    }

    fn poll_read_back(&mut self, id: ReadBackId) -> ReadBack {
        self.reads.remove(&id).unwrap_or(ReadBack::Failed)
    }
}

impl Drop for SkiaRenderer {
    fn drop(&mut self) {
        // The pictures and the surfaces' images go before the GPU device (field order does the
        // rest).
        self.kept.clear();
    }
}
