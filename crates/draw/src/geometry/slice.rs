//! Slice rectangles and the rules that size them: [`fixed_ratio_pixels`] (a Fixed Ratio slice
//! from a drag) and [`resized_rect`] (a handle drag). No egui. Normal and Fixed Size slices use the
//! marquee's rules (`craft_ui::canvas::{Constraint::corners, marquee_pixels, fixed_size_pixels}`).
//!
//! Slices are whole document pixels: every rectangle these return has integer edges.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use kurbo::{Rect, Vec2};

/// The handles of a rectangle in hit order, corners first, then edge middles: each is the
/// direction from the centre per axis (−1 the low edge, 0 the middle, 1 the high edge).
/// PhotoCraft's slice handles and the artboard handles use this order.
pub const HANDLES: [[i8; 2]; 8] = [[-1, -1], [1, -1], [-1, 1], [1, 1], [0, -1], [0, 1], [-1, 0], [1, 0]];

/// The Fixed Ratio slice drawn from `start` to `end` with the ratio `(w, h)`: the longer side of
/// the ratio follows the drag to within a pixel and the shorter side takes the ratio from it,
/// rounded to whole pixels (at least 1). A drag shorter than half a pixel along the longer side is
/// a click: a slice of no size. The slice sits on the pixel the press is in: drawn right or down
/// it starts at that pixel's top-left edge, drawn left or up at its far edge, so it holds that
/// pixel either way.
///
/// `Constraint::Ratio` is not used: rounding each edge of a fractional drag apart turns 2 : 1
/// into 5 × 3 (PhotoCraft test `fixed_ratio_slices_keep_their_ratio_from_a_fractional_drag`).
pub fn fixed_ratio_pixels(start: kurbo::Point, end: kurbo::Point, ratio: (f64, f64)) -> Rect {
    // pc: crates/ui-egui/src/tools/slice_ui.rs:113-148 `new_rect` ("fixedRatio": `k = |end.x -
    // start.x| / w` after `marquee_end`, `follow(long, short)` with `l = (k * long).round()` and
    // `(l * short / long).round().max(1.0)`; the corner on the press's pixel at :143-147).
    todo!()
}

/// `rect` with the edges `handle` grabs (one of [`HANDLES`]) moved by `by`, rounded to whole
/// pixels; an edge dragged past the opposite one turns the rectangle the right way round, which
/// stays at least 1 × 1.
pub fn resized_rect(rect: Rect, handle: [i8; 2], by: Vec2) -> Rect {
    // pc: crates/ui-egui/src/tools/slice_ui.rs:78-93 `resized` (`x0 + dx` for handle −1, `x1 + dx`
    // for 1; `(x0.min(x1), x0.max(x1).max(x0.min(x1) + 1))` per axis).
    todo!()
}

/// The handle of `rect` within `reach` document units of `point`, in [`HANDLES`] order (corners
/// win over edge middles), or `None`.
pub fn handle_at(rect: Rect, point: kurbo::Point, reach: f64) -> Option<[i8; 2]> {
    // pc: crates/ui-egui/src/tools/slice_ui.rs:58-74 `handle_at` (a square reach per axis around each
    // handle; corners first).
    todo!()
}
