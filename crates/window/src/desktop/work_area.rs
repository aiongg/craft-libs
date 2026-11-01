//! The part of the window the user can see and reach, and fitting a window that is larger than
//! its monitor.
//!
//! egui doesn't expose the monitor's work area (the screen less the taskbar, Dock or panel), only
//! the monitor's size and the window's position on the desktop. So the visible part is the content
//! rect cut to the monitor, keeping [`TASKBAR`] clear at the monitor's bottom. Menus size and place
//! themselves by it, so their last items and scroll arrows stay reachable on a small display.
//! [`fit_window`] also fixes the cause at startup: a window that doesn't fit its monitor is
//! maximized, which puts it in the work area (Photoshop opens maximized on small displays).

use egui::{Context, Id, Rect, Vec2, pos2, vec2};

/// Room kept free at the monitor's bottom edge for a taskbar or Dock egui can't see (the Windows
/// taskbar is 40–48 pt tall; a bottom Dock about as much).
// pc: crates/ui-egui/src/work_area.rs:17 `TASKBAR`.
pub const TASKBAR: f32 = 48.0;

/// Below this, the guess about where the monitor is went wrong: use the whole window.
const MIN_VISIBLE: f32 = 160.0;

/// How the window sits on its monitor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowMode {
    Normal,
    Maximized,
    /// Full screen covers the taskbar on purpose: all of it can be seen.
    FullScreen,
}

/// Where the window's content can be seen, in egui points. `monitor` is the monitor's size and
/// `inner` the content's rect on the desktop (both from [`egui::ViewportInfo`]).
///
/// - No monitor, a monitor size that isn't finite or under 1 pt, or [`WindowMode::FullScreen`]:
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
pub fn visible_part(content: Rect, monitor: Option<Vec2>, inner: Option<Rect>, state: WindowMode) -> Rect {
    let Some(m) = monitor else { return content };
    if state == WindowMode::FullScreen || !m.x.is_finite() || !m.y.is_finite() || m.x < 1.0 || m.y < 1.0 {
        return content;
    }
    if state == WindowMode::Maximized && inner.is_none() {
        return content;
    }
    let Some(inner) = inner else {
        let room = vec2(m.x, m.y - TASKBAR);
        return trusted(content, Rect::from_min_size(content.min, room));
    };
    if !inner.min.is_finite() || !inner.max.is_finite() {
        return content;
    }
    // Monitors are assumed to tile the desktop by their size: only the size is known.
    let c = inner.center();
    let origin = pos2((c.x / m.x).floor() * m.x, (c.y / m.y).floor() * m.y);
    let usable = Rect::from_min_max(origin, pos2(origin.x + m.x, origin.y + m.y - TASKBAR));
    trusted(content, usable.translate(content.min - inner.min))
}

impl WindowMode {
    fn of(viewport: &egui::ViewportInfo) -> Self {
        if viewport.fullscreen == Some(true) {
            Self::FullScreen
        } else if viewport.maximized == Some(true) {
            Self::Maximized
        } else {
            Self::Normal
        }
    }
}

/// `content` cut to `usable`, unless that leaves too little to be a sensible guess.
fn trusted(content: Rect, usable: Rect) -> Rect {
    let visible = content.intersect(usable);
    if !visible.is_positive() || visible.height() < MIN_VISIBLE.min(content.height()) || visible.width() < MIN_VISIBLE.min(content.width()) {
        return content;
    }
    visible
}

/// [`visible_part`] for this frame's viewport.
// pc: crates/ui-egui/src/work_area.rs:91 `visible_rect` (state from `WindowMode::of(viewport)`).
pub fn visible_rect(ctx: &Context) -> Rect {
    let content = ctx.content_rect();
    ctx.input(|i| {
        let v = i.viewport();
        visible_part(content, v.monitor_size, v.inner_rect, WindowMode::of(v))
    })
}

/// Whether a window of outer size `outer` fits `monitor`: `outer.x <= monitor.x + 0.5` and
/// `outer.y <= monitor.y - TASKBAR + 0.5`.
// pc: crates/ui-egui/src/work_area.rs:100 `fits`.
pub fn fits(outer: Vec2, monitor: Vec2) -> bool {
    outer.x <= monitor.x + 0.5 && outer.y <= monitor.y - TASKBAR + 0.5
}

fn fitted_id() -> Id {
    Id::unique("craft-window-fitted")
}

/// Once per context, as soon as the monitor is known: maximizes a window bigger than its monitor
/// and returns true when it asked to. Never on macOS (it keeps new windows inside the screen's
/// visible frame itself); never for a window already maximized or full screen. On Wayland, where
/// the outer rect is unknown, the content rect stands for it.
// pc: crates/ui-egui/src/work_area.rs:110 `fit_window`.
pub fn fit_window(ctx: &Context) -> bool {
    if ctx.data(|d| d.get_temp::<bool>(fitted_id())).is_some() {
        return false;
    }
    let content = ctx.content_rect();
    let (monitor, outer, maximized, fullscreen) = ctx.input(|i| {
        let v = i.viewport();
        (v.monitor_size, v.outer_rect, v.maximized, v.fullscreen)
    });
    let outer = outer.unwrap_or(content);
    let Some(m) = monitor else { return false };
    if cfg!(target_os = "macos") {
        return false;
    }
    ctx.data_mut(|d| d.insert_temp(fitted_id(), true));
    if maximized == Some(true) || fullscreen == Some(true) || !m.is_finite() || !outer.is_finite() || fits(outer.size(), m) {
        return false;
    }
    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
    true
}
