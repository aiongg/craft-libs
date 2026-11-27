//! The rt window's platform: one window the size of egui's content rect at egui's scale, its
//! events queued by the bridge before each step, and its requests kept for the bridge to apply to
//! egui after the step (key focus, IME, cursor, clipboard, access) or passed on at once
//! (repaints, wakes).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use craft_ui_rt::{
    Clipboard, Cursor, ImePurpose, ImeRequest, Platform, PlatformEvent, Rect, SizeU, SurfaceHandle, SurfaceHost, Vec2, Waker, WindowCommand,
    WindowId, WindowSpec,
};

/// The rt window's IME session, re-sent to egui on every pass while it lasts (egui forgets it at
/// the end of each pass).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ImeSession {
    pub purpose: ImePurpose,
    pub cursor: Rect,
    /// New at each start, so egui sees a new IME context.
    pub generation: u64,
}

/// Clipboard writes, applied with `Context::copy_text` after the step. Reads need nothing: egui
/// delivers pasted text as its `Paste` event, which the bridge forwards.
#[derive(Default)]
pub(crate) struct EguiClipboard {
    pub copies: Vec<String>,
}

impl Clipboard for EguiClipboard {
    fn set_text(&mut self, text: &str) {
        self.copies.push(text.to_owned());
    }

    fn request_text(&mut self, _: WindowId) {}
}

pub(crate) struct EguiHost {
    ctx: egui::Context,
    window: Option<WindowId>,
    next_window: u64,
    /// The window's logical size (egui's content rect) and scale (egui's points per pixel).
    pub size: Vec2,
    pub scale: f32,
    /// Events for the next step, filled by the bridge.
    pub queue: Vec<PlatformEvent>,
    woken: Arc<AtomicBool>,
    /// The step's frame time (egui's input time), the base of `request_frame_at`.
    pub now: f64,
    pub clipboard: EguiClipboard,
    pub ime: Option<ImeSession>,
    ime_starts: u64,
    pub cursor: Cursor,
    /// The window asked for key focus ([`WindowCommand::Focus`]).
    pub focus_request: bool,
    /// Access updates since the bridge last took them.
    pub access: Vec<accesskit::TreeUpdate>,
    /// Repaints the window asked egui for, for the trials.
    pub repaints: u32,
}

impl EguiHost {
    pub fn new(ctx: egui::Context) -> EguiHost {
        EguiHost {
            ctx,
            window: None,
            next_window: 1,
            size: Vec2::ZERO,
            scale: 1.0,
            queue: Vec::new(),
            woken: Arc::new(AtomicBool::new(false)),
            now: 0.0,
            clipboard: EguiClipboard::default(),
            ime: None,
            ime_starts: 0,
            cursor: Cursor::Default,
            focus_request: false,
            access: Vec::new(),
            repaints: 0,
        }
    }
}

impl SurfaceHost for EguiHost {
    fn windows(&self) -> Vec<WindowId> {
        self.window.into_iter().collect()
    }

    /// The meshes go to egui's painter, so there is no surface to hand over.
    fn surface(&self, window: WindowId) -> Option<SurfaceHandle> {
        (self.window == Some(window)).then_some(SurfaceHandle::Offscreen)
    }

    fn size_px(&self, _: WindowId) -> SizeU {
        let px = |v: f32| (v * self.scale).round().max(0.0) as u32;
        SizeU { width: px(self.size.x), height: px(self.size.y) }
    }

    fn scale(&self, _: WindowId) -> f32 {
        self.scale
    }
}

impl Platform for EguiHost {
    fn surfaces(&mut self) -> &mut dyn SurfaceHost {
        self
    }

    fn poll(&mut self, out: &mut Vec<PlatformEvent>) {
        if self.woken.swap(false, Ordering::AcqRel) {
            out.push(PlatformEvent::Wake);
        }
        out.append(&mut self.queue);
    }

    /// The first window is the embedded one; the windows its nodes would open have no place in
    /// the host and are never shown.
    fn create_window(&mut self, _: &WindowSpec) -> WindowId {
        let id = WindowId(self.next_window);
        self.next_window += 1;
        self.window.get_or_insert(id);
        id
    }

    fn window(&mut self, window: WindowId, cmd: WindowCommand) {
        if self.window == Some(window) && cmd == WindowCommand::Focus {
            self.focus_request = true;
        }
    }

    fn ime(&mut self, _: WindowId, req: ImeRequest) {
        match req {
            ImeRequest::Start { purpose, cursor } => {
                self.ime_starts += 1;
                self.ime = Some(ImeSession { purpose, cursor, generation: self.ime_starts });
            }
            ImeRequest::Cursor(cursor) => {
                if let Some(session) = &mut self.ime {
                    session.cursor = cursor;
                }
            }
            ImeRequest::Stop => self.ime = None,
        }
    }

    fn set_cursor(&mut self, _: WindowId, cursor: Cursor) {
        self.cursor = cursor;
    }

    fn clipboard(&mut self) -> &mut dyn Clipboard {
        &mut self.clipboard
    }

    fn access(&mut self, _: WindowId, update: accesskit::TreeUpdate) {
        self.access.push(update);
    }

    fn request_frame(&mut self) {
        self.repaints += 1;
        self.ctx.request_repaint();
    }

    fn request_frame_at(&mut self, time: f64) {
        self.repaints += 1;
        self.ctx.request_repaint_after(Duration::from_secs_f64((time - self.now).max(0.0)));
    }

    fn waker(&self) -> Waker {
        let (ctx, woken) = (self.ctx.clone(), self.woken.clone());
        Waker::new(move || {
            woken.store(true, Ordering::Release);
            ctx.request_repaint();
        })
    }
}
