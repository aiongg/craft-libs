//! An aliased tip's cell on the pixel grid.

use kurbo::Rect;

use crate::canvas::CanvasView;

/// The screen rect of `cell` (document units) through `view`, every edge rounded to a multiple
/// of `1 / pixels_per_point`, so the cell's outline lies on physical pixels at any zoom. A
/// `pixels_per_point` that is not finite and positive counts as 1. Under a turned view the rect
/// is the axis-aligned box of the mapped corners.
pub(super) fn pixel_cell(view: &CanvasView, cell: Rect, pixels_per_point: f32) -> egui::Rect {
    // pc: crates/ui-egui/src/canvas/cursors.rs:19 `pencil_cursor_rect`.
    let ppp = if pixels_per_point.is_finite() && pixels_per_point > 0.0 { pixels_per_point } else { 1.0 };
    let snap = |v: f32| (v * ppp).round() / ppp;
    let corners = view.quad(cell);
    let bounds = corners.iter().skip(1).fold(egui::Rect::from_min_max(corners[0], corners[0]), |r, p| r.union(egui::Rect::from_min_max(*p, *p)));
    egui::Rect::from_min_max(egui::pos2(snap(bounds.min.x), snap(bounds.min.y)), egui::pos2(snap(bounds.max.x), snap(bounds.max.y)))
}
