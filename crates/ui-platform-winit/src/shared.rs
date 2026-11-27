//! State the winit callbacks and the platform share. Callers copy what they need out of the
//! `RefCell` (an `Arc<Window>` clone, a value) and release the borrow before calling into winit,
//! which may deliver events synchronously.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use craft_ui_rt::{ImageKey, Mods, MonitorInfo, PlatformEvent, Pos2, RgbaImage, SizeU, Vec2, WindowCommand, WindowEvent, WindowId, WindowSpec};
use winit::monitor::MonitorHandle;
use winit::window::{CustomCursor, Window};

/// One window the app asked for. It exists from `create_window` on; `window` is `None` until the
/// event loop made the native window (and again on Android while the app is suspended).
pub(crate) struct Entry {
    pub id: WindowId,
    pub spec: WindowSpec,
    /// The `<canvas>` element to draw into (browser); a new one is appended to the page when `None`.
    pub canvas: Option<String>,
    pub window: Option<Arc<Window>>,
    pub adapter: Option<accesskit_winit::Adapter>,
    /// Commands received before the native window existed.
    pub queued: Vec<WindowCommand>,
    pub size_px: SizeU,
    pub scale: f32,
    pub surface_live: bool,
    pub mods: Mods,
    /// The lock keys, followed from the lock key presses (winit reports no lock state).
    pub caps_lock: bool,
    pub num_lock: bool,
    pub cursor: Pos2,
    /// What the last `State` and `FrameInset` events said, so only changes are sent.
    pub reported_state: Option<(bool, bool)>,
    pub reported_inset: Option<Vec2>,
}

pub(crate) fn monitor_info(handle: &MonitorHandle, primary: Option<&MonitorHandle>) -> MonitorInfo {
    let scale = handle.scale_factor();
    let (pos, size) = (handle.position(), handle.size());
    MonitorInfo {
        name: handle.name(),
        position: Pos2::new((f64::from(pos.x) / scale) as f32, (f64::from(pos.y) / scale) as f32),
        size: Vec2::new((f64::from(size.width) / scale) as f32, (f64::from(size.height) / scale) as f32),
        scale: scale as f32,
        refresh_hz: handle.refresh_rate_millihertz().map(|m| m as f32 / 1000.0),
        primary: primary == Some(handle),
    }
}

#[derive(Default)]
pub(crate) struct Shared {
    pub entries: Vec<Entry>,
    pub next_id: u64,
    pub events: Vec<PlatformEvent>,
    /// The runtime asked for a step; the loop turns it into a redraw of a live window.
    pub frame_wanted: bool,
    /// The frame time the runtime asked to be woken at (`Platform::request_frame_at`).
    pub wake_at: Option<f64>,
    /// Between `resumed` and `suspended`.
    pub live: bool,
    pub ever_opened: bool,
    pub monitors: Vec<MonitorInfo>,
    pub monitors_stale: bool,
    /// Pixels registered for `Cursor::Image` keys.
    pub cursor_images: HashMap<ImageKey, Arc<RgbaImage>>,
    pub cursor_cache: HashMap<(ImageKey, (u32, u32)), CustomCursor>,
    /// Custom cursors a window waits for: the loop creates them (winit needs the event loop).
    pub cursor_wanted: Vec<(WindowId, ImageKey, (u32, u32))>,
}

pub(crate) type Cell = Rc<RefCell<Shared>>;

impl Shared {
    pub fn entry(&self, id: WindowId) -> Option<&Entry> {
        self.entries.iter().find(|e| e.id == id)
    }

    pub fn entry_mut(&mut self, id: WindowId) -> Option<&mut Entry> {
        self.entries.iter_mut().find(|e| e.id == id)
    }

    pub fn native(&self, id: WindowId) -> Option<Arc<Window>> {
        self.entry(id)?.window.clone()
    }

    pub fn id_of(&self, native: winit::window::WindowId) -> Option<WindowId> {
        self.entries.iter().find(|e| e.window.as_ref().is_some_and(|w| w.id() == native)).map(|e| e.id)
    }
}

pub(crate) fn push(shared: &Cell, window: WindowId, event: WindowEvent) {
    let mut s = shared.borrow_mut();
    s.events.push(PlatformEvent::Window(window, event));
    s.frame_wanted = true;
}
