//! The part of the window the user can see and reach, and fitting a window that is larger than
//! its monitor.
//!
//! egui doesn't expose the monitor's work area (the screen less the taskbar, Dock or panel), only
//! the monitor's size and the window's position on the desktop. So the visible part is the content
//! rect cut to the monitor, keeping [`TASKBAR`] clear at the monitor's bottom. Menus size and place
//! themselves by it, so their last items and scroll arrows stay reachable on a small display.
//! [`fit_window`] also fixes the cause at startup: a window that doesn't fit its monitor is
//! maximized, which puts it in the work area (Photoshop opens maximized on small displays).

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{Context, Rect, Vec2};

/// Room kept free at the monitor's bottom edge for a taskbar or Dock egui can't see (the Windows
/// taskbar is 40–48 pt tall; a bottom Dock about as much).
// pc: crates/ui-egui/src/work_area.rs:17 `TASKBAR`.
pub const TASKBAR: f32 = 48.0;

/// How the window sits on its monitor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowState {
    Normal,
    Maximized,
    /// Full screen covers the taskbar on purpose: all of it can be seen.
    FullScreen,
}

/// Where the window's content can be seen, in egui points. `monitor` is the monitor's size and
/// `inner` the content's rect on the desktop (both from [`egui::ViewportInfo`]).
///
/// - No monitor, a monitor size that isn't finite or under 1 pt, or [`WindowState::FullScreen`]:
///   `content`.
/// - Maximized with no position (Wayland): `content`. Maximized with a position is checked like any
///   other window (an X11 window manager that reserves no panel space maximizes over the panel).
/// - No position (Wayland): the content from its top, `monitor.x` wide and `monitor.y - TASKBAR`
///   tall.
/// - Otherwise the monitor is taken to be the tile of the desktop, `monitor` sized, that holds the
///   window's centre (`floor(c / m) * m`), less [`TASKBAR`] at its bottom, moved into window
///   coordinates and cut with `content`.
/// - A result that isn't positive, or under 160 pt (or the content's size, if smaller) in either
///   direction, means the guess went wrong: `content`.
// pc: crates/ui-egui/src/work_area.rs:24 `visible_part`, :19 `MIN_VISIBLE` (160), :82 `trusted`.
pub fn visible_part(content: Rect, monitor: Option<Vec2>, inner: Option<Rect>, state: WindowState) -> Rect {
    todo!()
}

/// [`visible_part`] for this frame's viewport.
// pc: crates/ui-egui/src/work_area.rs:91 `visible_rect` (state from `WindowState::of(viewport)`).
pub fn visible_rect(ctx: &Context) -> Rect {
    todo!()
}

/// Whether a window of outer size `outer` fits `monitor`: `outer.x <= monitor.x + 0.5` and
/// `outer.y <= monitor.y - TASKBAR + 0.5`.
// pc: crates/ui-egui/src/work_area.rs:100 `fits`.
pub fn fits(outer: Vec2, monitor: Vec2) -> bool {
    todo!()
}

/// Once per context, as soon as the monitor is known: maximizes a window bigger than its monitor
/// and returns true when it asked to. Never on macOS (it keeps new windows inside the screen's
/// visible frame itself); never for a window already maximized or full screen. On Wayland, where
/// the outer rect is unknown, the content rect stands for it.
// pc: crates/ui-egui/src/work_area.rs:110 `fit_window`.
pub fn fit_window(ctx: &Context) -> bool {
    todo!()
}
