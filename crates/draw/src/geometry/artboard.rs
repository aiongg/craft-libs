//! Artboard rectangles and the rules that size them: [`drawn_rect`], [`moved_rect`],
//! [`resized_rect`] and [`resized_keep_ratio`]. No egui.
//!
//! Artboards are whole document pixels: every rectangle these return has integer edges and is at
//! least 1 × 1. A handle is one of [`HANDLES`](super::slice::HANDLES) (−1, 0 or 1 per axis).

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use kurbo::{Point, Rect, Vec2};

/// The board a drag from `a` to `b` draws: both corners rounded to whole pixels, ordered.
pub fn drawn_rect(a: Point, b: Point) -> Rect {
    // pc: crates/ui-egui/src/tools/artboard_tool.rs:113-115 `drawn`.
    todo!()
}

/// `rect` moved by `by` rounded to whole pixels (a move keeps the size).
pub fn moved_rect(rect: Rect, by: Vec2) -> Rect {
    // pc: crates/ui-egui/src/tools/artboard_tool.rs:118-120 `offset` (each axis rounded), :158-161.
    todo!()
}

/// `rect` with handle `handle` dragged by `by` (rounded to whole pixels). An edge dragged past the
/// opposite one flips the board, which never gets narrower than one pixel. `about_centre` (⌥)
/// moves the opposite edge by the same amount the other way, so the centre stays.
pub fn resized_rect(rect: Rect, handle: [i8; 2], by: Vec2, about_centre: bool) -> Rect {
    // pc: crates/ui-egui/src/tools/artboard_tool.rs:125-139 `resized` (per axis: −1 moves the low
    // edge, 1 the high one; `(a, b.max(a + 1))`).
    // new: `about_centre` (⌥, Photoshop's artboard resize; PhotoCraft has none).
    todo!()
}

/// `rect` with corner `handle` dragged by `by`, keeping its proportions (⇧ on a corner): the
/// opposite corner stays (the centre with `about_centre`), and the board scales by the larger of
/// the two axes' factors, each side rounded to whole pixels and at least 1. An edge handle (a 0 in
/// `handle`) resizes as [`resized_rect`] does.
pub fn resized_keep_ratio(rect: Rect, handle: [i8; 2], by: Vec2, about_centre: bool) -> Rect {
    // pc: crates/ui-egui/src/tools/artboard_tool.rs:141-153 `resized_keep` (`k = (dw.abs() /
    // w).max(dh.abs() / h)`, sides `(w * k).round().max(1.0)`, spanning from the fixed corner toward
    // the drag), :160 (only for a corner).
    // new: `about_centre`.
    todo!()
}
