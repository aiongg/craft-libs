//! The desktop as the dock sees it: which of the application's windows lies under a desktop point. Points are
//! desktop logical points.

use egui::{Context, Pos2, ViewportId};

use crate::runtime::windows::WindowHost;

/// The application window under the desktop point `desktop`: the focused one where windows overlap, else the
/// smallest. `None` over the desktop or another application's window.
pub fn window_under(ctx: &Context, desktop: Pos2) -> Option<ViewportId> {
    WindowHost::of(ctx).viewport_at(desktop)
}
