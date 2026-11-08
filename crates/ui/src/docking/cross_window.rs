//! Drags that cross windows. The drag session maps the desktop pointer into every window and each window's dock
//! resolves the drop from its own spots, so a panel, group or window dropped in another window docks there with no
//! second path. These functions are the dock's two window-level decisions: which window a drop lands in, and the
//! focus the receiving window takes.

use crate::runtime::drag;
use crate::runtime::windows::{WindowAction, WindowHost};
use egui::{Context, Pos2, ViewportId};

/// The window a drop at the desktop point `at` lands in: the topmost window there other than `dragged`. The window a
/// drag moves (a floating window's own drag) is always under the pointer, so it never counts as the target.
pub fn drag_target(ctx: &Context, at: Pos2, dragged: Option<ViewportId>) -> Option<ViewportId> {
    WindowHost::of(ctx).viewports_at(at).into_iter().find(|viewport| Some(*viewport) != dragged)
}

/// Raises the running window when the drop it takes comes from a drag that started in another window, so the user
/// continues in the window the panel landed in. Call it where the dock applies the dropped action. False for a drop
/// within the window the drag started in.
pub fn accept_cross_window(ctx: &Context) -> bool {
    let host = WindowHost::of(ctx);
    let viewport = host.current();
    drag::cross_window_pointer(ctx, viewport).is_some() && host.act(viewport, WindowAction::Focus)
}
