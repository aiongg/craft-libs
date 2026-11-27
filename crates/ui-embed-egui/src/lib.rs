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

use craft_ui_rt::{
    BuildCx, Constraints, Cursor, CursorIcon, Driver, Element, ImePurpose, LayoutCx, PlatformEvent, Rect, Runtime, State, StepReport, Tokens, Vec2,
    Waker, WindowEvent, WindowId, WindowSpec,
};
use web_time::Instant;

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

impl EmbedStats {
    fn add(&mut self, report: &StepReport) {
        if report.ran {
            self.ran = true;
            self.frames += 1;
        }
        self.built += report.frame.built;
        self.laid_out += report.frame.laid_out;
        self.painted += report.frame.painted;
        self.prepared += report.render.prepared;
        self.reused += report.render.reused;
    }
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
        let ctx = ui.ctx().clone();
        let response = ui.interact(rect, self.id, egui::Sense::click_and_drag());
        let (window, id) = (self.window, self.id);
        let mut stats = EmbedStats::default();

        let (content, scale, time) = (ctx.content_rect(), ctx.pixels_per_point(), ui.input(|i| i.time));
        // The window spans the screen from its origin to the content rect's far corner, so its
        // coordinates are egui's.
        let size = content.max.to_vec2();
        let host = &mut self.driver.platform;
        host.now = time;
        if scale != host.scale {
            host.scale = scale;
            host.queue.push(PlatformEvent::Window(window, WindowEvent::Scale(scale)));
        }
        if size != host.size {
            host.size = size;
            host.queue.push(PlatformEvent::Window(window, WindowEvent::Resize(size)));
        }
        let runtime = &mut self.driver.runtime;
        if *runtime.store().peek(self.rect) != rect {
            self.rect.set(runtime.store_mut(), rect);
        }

        let focused = self.focus.before(&ctx, id, window, &mut self.driver.runtime, &mut self.driver.platform.queue);
        let hovered = response.hovered();
        let gate = input::Gate { window, hovered, focused, platform: craft_keymap::egui::platform(&ctx) };
        let (pointer, queue, access) = (&mut self.pointer, &mut self.driver.platform.queue, &self.access);
        let (sent, scrolling) = ui.input(|i| {
            let mut sent = input::forward(i, &gate, pointer, queue);
            for event in &i.events {
                if let egui::Event::AccessKitActionRequest(request) = event
                    && let Some(request) = access.action(request)
                {
                    queue.push(PlatformEvent::Window(window, WindowEvent::Access(request)));
                    sent.events += 1;
                }
            }
            (sent, i.smooth_scroll_delta != Vec2::ZERO)
        });
        stats.events = sent.events;
        if hovered && scrolling {
            // The panel takes the wheel: no parent scrolls by it.
            ctx.input_mut(|i| i.smooth_scroll_delta = Vec2::ZERO);
        }

        let t = Instant::now();
        let report = self.driver.step(time);
        stats.add(&report);
        if !self.driver.renderer.is_current() {
            // egui's own text grew or cleared the glyph atlas since the meshes were made.
            self.driver.runtime.invalidate_pixels();
            let report = self.driver.step(time);
            stats.add(&report);
        }
        stats.step_ms = t.elapsed().as_secs_f64() * 1000.0;

        let unhandled = self.driver.runtime.take_unhandled_keys(window);
        let claimed = input::claimed(&sent.keys, &unhandled);
        if !claimed.is_empty() {
            ctx.input_mut(|i| {
                for &index in claimed.iter().rev() {
                    i.events.remove(index);
                }
            });
        }
        self.focus.after(&ctx, id, window, &mut self.driver.runtime, &mut self.driver.platform, &unhandled);
        self.apply_outputs(&ctx, rect, focused, hovered);

        let t = Instant::now();
        let (meshes, vertices) = self.driver.renderer.emit(&ui.painter().with_clip_rect(rect));
        self.access.emit(&ctx, id);
        stats.meshes = meshes;
        stats.vertices = vertices;
        stats.emit_ms = t.elapsed().as_secs_f64() * 1000.0;
        EmbedResponse { response, stats }
    }

    /// The window's requests of this step, applied to egui: IME, cursor, clipboard, access.
    fn apply_outputs(&mut self, ctx: &egui::Context, rect: Rect, focused: bool, hovered: bool) {
        let host = &mut self.driver.platform;
        if focused && let Some(session) = host.ime {
            let purpose = match session.purpose {
                ImePurpose::Normal => egui::IMEPurpose::Normal,
                ImePurpose::Password => egui::IMEPurpose::Password,
            };
            ctx.output_mut(|o| o.ime = Some(egui::output::IMEOutput::new(self.id, session.generation, purpose, rect, session.cursor)));
        }
        if hovered || self.pointer.captured() {
            ctx.set_cursor_icon(cursor_icon(host.cursor));
        }
        for text in host.clipboard.copies.drain(..) {
            ctx.copy_text(text);
        }
        for update in host.access.drain(..) {
            self.access.apply(update);
        }
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

/// The egui cursor for an rt cursor. An image cursor (an rt-drawn tool cursor) has no egui
/// equivalent and shows the default.
fn cursor_icon(cursor: Cursor) -> egui::CursorIcon {
    use egui::CursorIcon as E;
    let Cursor::Icon(icon) = cursor else {
        return if cursor == Cursor::Hidden { E::None } else { E::Default };
    };
    match icon {
        CursorIcon::Default => E::Default,
        CursorIcon::None => E::None,
        CursorIcon::ContextMenu => E::ContextMenu,
        CursorIcon::Help => E::Help,
        CursorIcon::PointingHand => E::PointingHand,
        CursorIcon::Progress => E::Progress,
        CursorIcon::Wait => E::Wait,
        CursorIcon::Cell => E::Cell,
        CursorIcon::Crosshair => E::Crosshair,
        CursorIcon::Text => E::Text,
        CursorIcon::VerticalText => E::VerticalText,
        CursorIcon::Alias => E::Alias,
        CursorIcon::Copy => E::Copy,
        CursorIcon::Move => E::Move,
        CursorIcon::NoDrop => E::NoDrop,
        CursorIcon::NotAllowed => E::NotAllowed,
        CursorIcon::Grab => E::Grab,
        CursorIcon::Grabbing => E::Grabbing,
        CursorIcon::AllScroll => E::AllScroll,
        CursorIcon::ResizeHorizontal => E::ResizeHorizontal,
        CursorIcon::ResizeNeSw => E::ResizeNeSw,
        CursorIcon::ResizeNwSe => E::ResizeNwSe,
        CursorIcon::ResizeVertical => E::ResizeVertical,
        CursorIcon::ResizeColumn => E::ResizeColumn,
        CursorIcon::ResizeEast => E::ResizeEast,
        CursorIcon::ResizeSouthEast => E::ResizeSouthEast,
        CursorIcon::ResizeSouth => E::ResizeSouth,
        CursorIcon::ResizeSouthWest => E::ResizeSouthWest,
        CursorIcon::ResizeWest => E::ResizeWest,
        CursorIcon::ResizeNorthWest => E::ResizeNorthWest,
        CursorIcon::ResizeNorth => E::ResizeNorth,
        CursorIcon::ResizeNorthEast => E::ResizeNorthEast,
        CursorIcon::ZoomIn => E::ZoomIn,
        CursorIcon::ZoomOut => E::ZoomOut,
        _ => E::Default,
    }
}
