//! Whether a window fits its monitor.
//!
//! egui doesn't expose the monitor's work area (the screen less the taskbar, Dock or panel), only
//! the monitor's size. [`TASKBAR`] is the room kept free at the monitor's bottom edge.

use egui::{Rect, Vec2, ViewportInfo};

/// Room kept free at the monitor's bottom edge for a taskbar or Dock egui can't see (the Windows
/// taskbar is 40–48 pt tall; a bottom Dock about as much).
// pc: crates/ui-egui/src/work_area.rs:17 `TASKBAR`.
pub const TASKBAR: f32 = 48.0;

/// Whether a window of outer size `outer` fits `monitor`: `outer.x <= monitor.x + 0.5` and
/// `outer.y <= monitor.y - TASKBAR + 0.5`.
// pc: crates/ui-egui/src/work_area.rs:100 `fits`.
pub fn fits(outer: Vec2, monitor: Vec2) -> bool {
    outer.x <= monitor.x + 0.5 && outer.y <= monitor.y - TASKBAR + 0.5
}

/// How the window is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowState {
    /// Neither maximized nor full screen.
    Normal,
    /// Maximized.
    Maximized,
    /// Full screen.
    FullScreen,
}

impl WindowState {
    /// The state `info` reports.
    pub fn of(info: &ViewportInfo) -> Self {
        let _ = info;
        todo!("tp.T12")
    }
}

/// The part of `content` that is visible on a monitor of size `monitor` for a window in `state` with inner rect
/// `inner`; the whole of `content` when the monitor or the position is unknown.
pub fn visible_part(content: Rect, monitor: Option<Vec2>, inner: Option<Rect>, state: WindowState) -> Rect {
    let _ = (content, monitor, inner, state);
    todo!("tp.T12")
}
