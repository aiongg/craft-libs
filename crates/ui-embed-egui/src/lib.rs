//! An rt window drawn inside an egui app (a PhotoCraft panel on craft-ui-rt in egui PhotoCraft).
//!
//! The rt window covers egui's whole content rect, so pointer positions and kept meshes are in
//! egui's screen coordinates with no translation, and pixel snapping is egui's. Its root places
//! the app's content at the rect [`EmbeddedWindow::show`] is given. Each egui pass that shows the
//! window runs one [`craft_ui_rt::Driver`] step through two bridges:
//! - the platform ([`host`]): egui's events while the panel is hovered, holds a press or has
//!   egui's key focus; repaints, wakes, cursor, clipboard, IME and access updates back to egui;
//! - the renderer ([`painter`]): render-epaint's kept meshes, moved to window coordinates once per
//!   rt frame and added to egui's painter on every pass; images as egui textures; text through
//!   egui's own fonts and glyph atlas ([`fonts`]).
//!
//! A pass with nothing dirty runs no build, layout, paint or tessellation: the kept meshes are
//! added again. An idle egui app runs no pass and the window asks for none.

mod access;
mod focus;
mod fonts;
mod host;
mod input;
mod painter;

use std::rc::Rc;

use craft_ui_rt::{BuildCx, Constraints, Driver, Element, LayoutCx, Rect, Runtime, State, Tokens, Vec2, Waker, WindowId, WindowSpec};

pub use fonts::EguiFonts;

use crate::host::EguiHost;
use crate::painter::EguiPainter;

/// What one [`EmbeddedWindow::show`] did.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EmbedStats {
    /// The driver steps, the atlas resync's included.
    pub step_ms: f64,
    /// Adding the kept meshes to egui's painter (and the access nodes, when AccessKit is on).
    pub emit_ms: f64,
    /// An rt frame ran in this pass.
    pub ran: bool,
    /// rt frames this pass: 2 when the glyph atlas moved and the meshes were made again.
    pub frames: u32,
    /// egui events forwarded to the rt window.
    pub events: u32,
    pub built: u32,
    pub laid_out: u32,
    pub painted: u32,
    /// Display lists tessellated this pass, and those reused from the kept meshes.
    pub prepared: u32,
    pub reused: u32,
    /// Meshes added to egui's painter, and their vertices.
    pub meshes: u32,
    pub vertices: u32,
}

/// What [`EmbeddedWindow::show`] returns.
pub struct EmbedResponse {
    /// The panel's egui response (hover, press capture, key focus).
    pub response: egui::Response,
    pub stats: EmbedStats,
}

/// One rt window inside an egui app.
pub struct EmbeddedWindow {
    driver: Driver<EguiHost, EguiPainter>,
    window: WindowId,
    id: egui::Id,
    /// Where the root places the content, in window (egui screen) coordinates.
    rect: State<Rect>,
    focus: focus::FocusMirror,
    pointer: input::PointerGate,
    access: access::AccessMirror,
}

impl EmbeddedWindow {
    /// An rt window over `ctx` with `tokens`, whose egui widget id is `id`. It shows nothing until
    /// [`EmbeddedWindow::set_root`].
    pub fn new(ctx: &egui::Context, id: egui::Id, tokens: Tokens) -> EmbeddedWindow {
        let mut runtime = Runtime::new(tokens);
        let rect = runtime.store_mut().create(Rect::NOTHING);
        let host = EguiHost::new(ctx.clone());
        let painter = EguiPainter::new(ctx.clone());
        let mut driver = Driver::new(runtime, host, painter);
        let spec = WindowSpec { embedded: true, ..WindowSpec::new("embedded") };
        let window = driver.open_window(&spec, Placed { rect, content: Rc::new(|_: &mut BuildCx<'_>| {}) });
        EmbeddedWindow {
            driver,
            window,
            id,
            rect,
            focus: focus::FocusMirror::default(),
            pointer: input::PointerGate::default(),
            access: access::AccessMirror::default(),
        }
    }

    /// The window's content: `content` builds it (its reads are tracked as in any build) and the
    /// root lays each child out at the rect `show` is given.
    pub fn set_root(&mut self, content: impl Fn(&mut BuildCx<'_>) + 'static) {
        self.driver.runtime.set_root(self.window, Placed { rect: self.rect, content: Rc::new(content) });
    }

    /// Runs the window for this egui pass in `rect` (screen points) and adds its meshes to `ui`'s
    /// painter at the call position, clipped to `rect` and the ui's clip.
    pub fn show(&mut self, ui: &mut egui::Ui, rect: Rect) -> EmbedResponse {
        let _ = (ui, rect);
        todo!("T2: bridge body")
    }

    pub fn runtime(&self) -> &Runtime {
        &self.driver.runtime
    }

    /// The runtime: its cells (the document the content reads), images (thumbnails) and tokens.
    pub fn runtime_mut(&mut self) -> &mut Runtime {
        &mut self.driver.runtime
    }

    /// A waker for background jobs: it asks egui for a pass, and the next step delivers
    /// [`craft_ui_rt::PlatformEvent::Wake`].
    pub fn waker(&self) -> Waker {
        self.driver.waker()
    }

    pub fn window(&self) -> WindowId {
        self.window
    }

    /// The panel's egui widget id: egui key focus on it is the rt window's key focus.
    pub fn id(&self) -> egui::Id {
        self.id
    }
}

/// The window root: lays every child out at the rect cell, in window coordinates.
struct Placed {
    rect: State<Rect>,
    content: Rc<dyn Fn(&mut BuildCx<'_>)>,
}

impl Element for Placed {
    type Output = ();
    type Local = ();

    fn props_key(&self) -> u64 {
        // Each `set_root` is new content.
        Rc::as_ptr(&self.content).cast::<()>() as usize as u64
    }

    fn init(&self, _: &mut BuildCx<'_>) {}

    fn build(&self, _: &mut (), cx: &mut BuildCx<'_>) {
        (self.content)(cx);
    }

    fn layout(&self, _: &mut (), cx: &mut LayoutCx<'_>, c: Constraints) -> Vec2 {
        let rect = self.rect.get(cx);
        let size = if rect.is_positive() { rect.size() } else { Vec2::ZERO };
        let at = if rect.is_positive() { rect.min.to_vec2() } else { Vec2::ZERO };
        for i in 0..cx.child_count() {
            let child = cx.child_at(i);
            cx.layout_child(child, Constraints::tight(size));
            cx.place_child(child, at);
        }
        c.max
    }

    fn relayout_boundary(&self) -> bool {
        true
    }
}
