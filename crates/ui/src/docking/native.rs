//! Native dock windows: a floating dock group that shows in an OS window of its own.
//!
//! A native group is a [`FloatingGroup`](super::FloatingGroup) mounted in a viewport. The container does the window
//! work: its title row moves the window, its grips resize it, its limits are the window's, and a close is a
//! `close_requested` report. What is left is policy, in these functions: whether a window of the dock model gets an
//! OS window, what kind of window it is, where it opens on the connected monitors, and what the dock model records
//! when the OS moves or resizes it. `DockArea` and `FloatingGroup` call them; rects are desktop logical points.

use super::{DockConfig, FloatingGroupOutput, PanelSpec};
use crate::runtime::drag;
use crate::runtime::env::ShellCx;
use crate::runtime::windows::{WindowAction, WindowHost, WindowSpec};
use craft_behavior::dock::{Action, Floating, NativeSpace, restore_rect};
use craft_behavior::placement::monitor_of;
use craft_behavior::{GesturePhase, Monitor};
use craft_host::target::WindowKind;
use egui::{Context, Rect, Vec2, ViewportId};

/// How far the OS-reported rect of a window may differ from the one the dock model keeps before the difference is a
/// move or resize: the OS rounds to pixels, and a model that records its own rounding would re-send it for ever.
const SETTLED: f32 = 0.5;

/// How a floating window of the dock model shows.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub(crate) struct Opening {
    /// The rect the group is placed at: on the desktop for an OS window, in the app window otherwise.
    pub rect: Rect,
    /// The OS window to open, or `None` where the group shows in the app window.
    pub spec: Option<WindowSpec>,
}

/// Whether the groups of panels with `specs` may open OS windows: the dock's `config` turns them on, the platform
/// opens them and every panel allows it.
pub(crate) fn allowed(ctx: &Context, shell: Option<&ShellCx>, config: &DockConfig, specs: &[PanelSpec]) -> bool {
    config.native_windows && WindowHost::of(ctx).viewports_available(shell) && specs.iter().all(|spec| spec.allow_native)
}

/// The window a group of panels with `specs` opens: a document window when any panel is a document, else a tool
/// window (a palette level, no taskbar entry).
pub fn spec_for(specs: &[PanelSpec]) -> WindowSpec {
    if specs.iter().any(|spec| spec.window_kind == WindowKind::Document) { WindowSpec::document() } else { WindowSpec::tool() }
}

/// Where the saved native rect of `window` opens against `space`: its saved monitor when it is connected, else the
/// main window's monitor at the same offset; a window whose bar cannot be grabbed is moved onto the nearest monitor.
/// `None` for a window that is not native.
pub fn restore<P>(window: &Floating<P>, space: NativeSpace<'_>) -> Option<Rect> {
    window.native.map(|saved| restore_rect(saved, window.monitor.as_ref(), space))
}

/// The space a native window is placed against: `monitors` (in the points of the root window, see
/// [`monitors`](super::desktop::monitors)), the main window's desktop rect, and for a platform that lists no monitors
/// the monitor size the window recorded. `bar` is the height of the group's title row, which must stay reachable.
pub fn space_of<'a, P>(ctx: &Context, window: &Floating<P>, monitors: &'a [Monitor], bar: f32) -> NativeSpace<'a> {
    let mut space = NativeSpace::new(monitors, bar);
    space.main = WindowHost::of(ctx).facts(ViewportId::ROOT).map(|facts| facts.outer.unwrap_or(facts.inner));
    space.monitor_size = window.monitor_size.map(Vec2::from);
    space
}

/// How `window` shows: in an OS window at its restored rect when it is native and `config`, the platform and its
/// panels allow one, else in the app window at its own rect. `specs` are its panels' specs.
pub(crate) fn opening<P>(
    ctx: &Context,
    shell: Option<&ShellCx>,
    config: &DockConfig,
    window: &Floating<P>,
    specs: &[PanelSpec],
    bar: f32,
) -> Opening {
    if allowed(ctx, shell, config, specs) {
        let monitors = super::desktop::monitors(ctx);
        if let Some(rect) = restore(window, space_of(ctx, window, &monitors, bar)) {
            return Opening { rect, spec: Some(spec_for(specs)) };
        }
    }
    Opening { rect: window.rect, spec: None }
}

/// The actions that record where the OS put the native window `window`, at the desktop rect `rect` (a window's
/// `WindowFacts::outer`, else its `inner`): that rect and the monitor holding it. Empty while the model already
/// holds what the OS reports, so a window at rest costs no action.
pub(crate) fn reported<P: Clone>(window: &Floating<P>, rect: Rect, monitors: &[Monitor]) -> Vec<Action<P>> {
    let Some(panel) = window.panels().next() else { return Vec::new() };
    let mut actions = Vec::new();
    if window.native.is_none_or(|saved| !same(saved, rect)) {
        actions.push(Action::set_native(panel.clone(), Some(rect)));
    }
    let on = monitor_of(monitors, rect).cloned();
    if on.is_some() && window.monitor != on {
        actions.push(Action::set_monitor(panel.clone(), on, None));
    }
    actions
}

fn same(a: Rect, b: Rect) -> bool {
    a.min.distance(b.min) <= SETTLED && a.max.distance(b.max) <= SETTLED
}

/// Raises the window of the native group that `output` reports on when the user began moving or resizing it this
/// pass, so a palette comes to the front of the windows it overlaps. False when nothing was raised.
pub(crate) fn raise(ctx: &Context, output: &FloatingGroupOutput) -> bool {
    let began = output.placed.is_some_and(|change| change.phase == GesturePhase::Begin);
    let host = WindowHost::of(ctx);
    began && host.viewport_of(output.response.id).is_some_and(|viewport| host.act(viewport, WindowAction::Focus))
}

/// Fades the native window of the group that `output` reports on while its drag highlights a drop target, and makes
/// it opaque again otherwise: over empty space, for a move that cannot dock and while Ctrl or Cmd keeps the drop
/// floating (the target then takes no drop and shows no mark). Every way a drag ends brings the opacity back, because
/// this runs in each pass of the group. The group runs in its parent's pass, so the window is the group's own, not the
/// running viewport.
pub(crate) fn fade(ctx: &Context, output: &FloatingGroupOutput) {
    let host = WindowHost::of(ctx);
    let Some(viewport) = host.viewport_of(output.response.id) else { return };
    let highlighted = drag::moves(ctx, viewport) && drag::mark(ctx).is_some();
    host.act(viewport, WindowAction::SetOpacity(if highlighted { craft_theme::Opacity::MOVING } else { 1.0 }));
}

/// Tells the window host that the native window of the group `id` is dragged by its title row from the app (the group
/// docks its window), so the Windows hit test leaves that row to the app.
pub(crate) fn keeps_app_drag(ctx: &Context, id: egui::Id) {
    if let Some(viewport) = WindowHost::of(ctx).viewport_of(id) {
        crate::runtime::windows::keeps_app_drag(ctx, viewport);
    }
}
