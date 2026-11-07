//! Allocation and `interact_opt`.

use egui::{Rect, Ui, Vec2};

/// Takes `size` in the parent's layout and returns its rect. Registers no widget.
pub(crate) fn space(ui: &mut Ui, size: Vec2) -> Rect {
    ui.allocate_space(size).1
}

/// Moves the parent's cursor past `rect`, a cell the caller placed itself.
pub(crate) fn advance_past(ui: &mut Ui, rect: Rect) {
    ui.advance_cursor_after_rect(rect);
}
