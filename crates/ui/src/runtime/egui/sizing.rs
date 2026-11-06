//! Sizing passes and `request_discard`.

use egui::{Context, Layout, Rect, Ui, UiBuilder, Vec2};

/// Asks egui to discard the running pass and run it again.
pub(crate) fn request_discard(ctx: &Context, reason: &'static str) {
    ctx.request_discard(reason);
}

/// Runs `f` in an invisible sizing pass `width` wide, from the parent's cursor, and returns what it took.
/// The parent allocates nothing.
pub(crate) fn run_sized<R>(ui: &mut Ui, width: f32, layout: Layout, f: impl FnOnce(&mut Ui) -> R) -> (R, Vec2) {
    let room = Rect::from_min_size(ui.cursor().min, Vec2::new(width, f32::INFINITY));
    let mut child = ui.new_child(UiBuilder::new().max_rect(room).layout(layout).sizing_pass().invisible());
    let inner = f(&mut child);
    (inner, child.min_rect().size())
}
