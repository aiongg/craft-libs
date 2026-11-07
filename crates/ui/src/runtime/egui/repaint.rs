//! `request_repaint_after`, the one timed wake, and the immediate repaint request every other caller goes through.

use egui::Context;

/// Asks for one more pass after this one.
pub(crate) fn request(ctx: &Context) {
    ctx.request_repaint();
}
