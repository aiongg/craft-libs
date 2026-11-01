//! The Navigator's geometry: where the document sits in the preview box, the proxy of the view,
//! and the view a click or a Cmd/Ctrl-drag asks for. Pure functions, for tests and for apps that
//! draw a preview of their own (VectorCraft's artboards-only preview).
#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{Pos2, Rect};
use kurbo::{Point, Size};

use crate::canvas::CanvasView;

/// The document of `size` (document units) fitted inside `frame` with `inset` points on every
/// side, centred and keeping its aspect: the rect the preview picture is drawn into.
///
/// The space left after the inset is at least 1 × 1 point; an empty size counts as 1 × 1.
/// [`Navigator`](super::Navigator) passes its style's `inset` (`navigator.inset`, 8).
pub fn preview_rect(frame: Rect, size: Size, inset: f32) -> Rect {
    // pc: crates/ui-egui/src/panels/navigator.rs:157 `navigator_fit` (inner = frame − 2 × 8, at least 1 × 1;
    // scale = min(inner.x / w, inner.y / h); centred on frame.center()).
    todo!()
}

/// The proxy of `view` in the preview: the canvas rectangle `view.rect` mapped into document
/// space and from there into `preview` (the rect from [`preview_rect`] for `size`), as four
/// corners in the canvas's order (top-left, top-right, bottom-right, bottom-left on screen).
///
/// On an upright, unmirrored view this is PhotoCraft's rectangle: `view.rect`'s size over the
/// point zoom, divided by the pixel aspect horizontally ([`CanvasView::aspect`]), times the
/// preview scale, centred on the preview point of `view.center`. On a rotated or flipped view
/// it is the quad [`CanvasView::to_doc`] gives for the canvas corners, mapped the same way
/// (lead call L3; PhotoCraft never rotated its navigator).
///
/// The quad is not clipped: [`Navigator::show`](super::Navigator::show) clips each edge to the
/// preview box less 1 pt when it draws it, and an upright proxy is the axis-aligned rect
/// intersected with that box (PhotoCraft's `intersect(frame.shrink(1.0))`).
pub fn proxy_quad(view: &CanvasView, size: Size, preview: Rect) -> [Pos2; 4] {
    // pc: crates/ui-egui/src/panels/navigator.rs:99-104 (point zoom, aspect, scale `s`, centre `c`).
    todo!()
}

/// Zooms `view` so the document rectangle `doc_rect` (document units) fills the canvas
/// `view.rect`, centred on it, as a Navigator Cmd/Ctrl-drag does. Returns whether the view
/// changed.
///
/// Rules, in order (PhotoCraft's `navigator_zoom_to`):
/// - Any edge of `doc_rect` that is not finite: nothing happens (checked before clipping, since
///   `min`/`max` would turn a NaN edge into a document edge).
/// - `doc_rect` is intersected with the document `(0, 0)..size`; a negative width or height
///   after that (a rectangle off the document) does nothing.
/// - Under 2 document units on a side, or an empty canvas: one ladder rung in
///   ([`ZoomLadder::step`](crate::canvas::ZoomLadder::step) with `dir` 1) on the rectangle's
///   centre.
/// - Otherwise the point zoom is `min(canvas.w / (r.w × aspect), canvas.h / r.h)`, converted to the
///   ladder's device zoom with `pixels_per_point` (the inverse of
///   [`CanvasView::points_per_unit`]), clamped with
///   [`ZoomLadder::clamp`](crate::canvas::ZoomLadder::clamp), and a zoom that is not finite does
///   nothing.
/// - The centre becomes the clipped rectangle's centre. No animation: the caller reports
///   `animate_zoom` false.
pub fn zoom_to_document_rect(view: &mut CanvasView, doc_rect: kurbo::Rect, size: Size, pixels_per_point: f32) -> bool {
    // pc: crates/ui-egui/src/panels/navigator.rs:174-199 `navigator_zoom_to`.
    todo!()
}

/// The document point under `at` in a preview drawn into `preview` for a document of `size`,
/// clamped to the document `(0, 0)..=size`: the view centre a click or a drag at `at` asks for.
pub fn view_center_for(preview: Rect, size: Size, at: Pos2) -> Point {
    // pc: crates/ui-egui/src/panels/navigator.rs:61-62 (`(pp − rect.min) / s`, clamped to the document).
    todo!()
}
