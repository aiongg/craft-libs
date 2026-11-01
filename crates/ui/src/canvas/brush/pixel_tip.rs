//! An aliased tip's cell on the pixel grid.
#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use kurbo::Rect;

use crate::canvas::CanvasView;

/// The screen rect of `cell` (document units) through `view`, every edge rounded to a multiple
/// of `1 / pixels_per_point`, so the cell's outline lies on physical pixels at any zoom. A
/// `pixels_per_point` that is not finite and positive counts as 1. Under a turned view the rect
/// is the axis-aligned box of the mapped corners.
pub(super) fn pixel_cell(view: &CanvasView, cell: Rect, pixels_per_point: f32) -> egui::Rect {
    // pc: crates/ui-egui/src/canvas/cursors.rs:19 `pencil_cursor_rect`.
    todo!()
}
