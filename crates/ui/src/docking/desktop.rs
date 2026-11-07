//! The desktop as the dock sees it: the connected monitors in the model's type, and which of the application's
//! windows lies under a desktop point. Points are desktop logical points.

use craft_behavior::Monitor;
use egui::{Context, Pos2, ViewportId};

use crate::runtime::windows::WindowHost;

/// The connected monitors in desktop points of the root window, as the model's [`Monitor`]s. Empty where the
/// platform lists none, or before the root window has reported its scale; a placement then uses the size of the
/// monitor the main window is on.
pub fn monitors(ctx: &Context) -> Vec<Monitor> {
    let host = WindowHost::of(ctx);
    let Some(scale) = host.facts(ViewportId::ROOT).map(|facts| facts.scale) else { return Vec::new() };
    host.monitors().into_iter().map(|info| Monitor::new(Some(info.name.clone()), info.points_at(scale), info.scale)).collect()
}

/// The application window under the desktop point `desktop`: the focused one where windows overlap, else the
/// smallest. `None` over the desktop or another application's window.
pub fn window_under(ctx: &Context, desktop: Pos2) -> Option<ViewportId> {
    WindowHost::of(ctx).viewport_at(desktop)
}
