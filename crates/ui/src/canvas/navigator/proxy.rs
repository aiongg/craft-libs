//! The Navigator's geometry: where the document sits in the preview box, the proxy of the view,
//! and the view a click or a Cmd/Ctrl-drag asks for. Pure functions, for tests and for apps that
//! draw a preview of their own (VectorCraft's artboards-only preview).

use egui::{Pos2, Rect, pos2, vec2};
use kurbo::{Point, Size};

use crate::canvas::{CanvasView, ZoomLadder, zoom};

/// A zoom rectangle under this many document units on a side steps one rung in instead.
const MIN_ZOOM_BOX: f64 = 2.0;

/// The document of `size` (document units) fitted inside `frame` with `inset` points on every
/// side, centred and keeping its aspect: the rect the preview picture is drawn into.
///
/// The space left after the inset is at least 1 × 1 point; an empty size counts as 1 × 1.
/// [`Navigator`](super::Navigator) passes its style's `inset` (the inset, 8).
pub fn preview_rect(frame: Rect, size: Size, inset: f32) -> Rect {
    // pc: crates/ui-egui/src/panels/navigator.rs:157 `navigator_fit` (inner = frame − 2 × 8, at least 1 × 1;
    // scale = min(inner.x / w, inner.y / h); centred on frame.center()).
    let inner = (frame.size() - vec2(2.0 * inset, 2.0 * inset)).max(vec2(1.0, 1.0));
    let (w, h) = (size.width.max(1.0) as f32, size.height.max(1.0) as f32);
    let scale = (inner.x / w).min(inner.y / h);
    Rect::from_center_size(frame.center(), vec2(w * scale, h * scale))
}

/// The proxy of `view` in the preview: the canvas rectangle `view.rect` mapped into document
/// space and from there into `preview` (the rect from [`preview_rect`] for `size`), as four
/// corners in the canvas's order (top-left, top-right, bottom-right, bottom-left on screen).
///
/// On an upright, unmirrored view this is PhotoCraft's rectangle: `view.rect`'s size over the
/// point zoom, divided by the pixel aspect horizontally ([`CanvasView::aspect`]), times the
/// preview scale, centred on the preview point of `view.center`. On a rotated or flipped view
/// it is the quad [`CanvasView::to_doc`] gives for the canvas corners, mapped the same way
/// (PhotoCraft never rotated its navigator).
///
/// The quad is not clipped: [`Navigator::show`](super::Navigator::show) clips each edge to the
/// preview box less 1 pt when it draws it, and an upright proxy is the axis-aligned rect
/// intersected with that box (PhotoCraft's `intersect(frame.shrink(1.0))`).
pub fn proxy_quad(view: &CanvasView, size: Size, preview: Rect) -> [Pos2; 4] {
    // pc: crates/ui-egui/src/panels/navigator.rs:99-104 (point zoom, aspect, scale `s`, centre `c`).
    let s = preview_scale(preview, size);
    let r = view.rect;
    [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()].map(|corner| {
        let doc = view.to_doc(corner);
        pos2(preview.min.x + doc.x as f32 * s, preview.min.y + doc.y as f32 * s)
    })
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
    let edges = [doc_rect.x0, doc_rect.y0, doc_rect.x1, doc_rect.y1];
    if !edges.iter().all(|v| v.is_finite()) {
        return false;
    }
    // Not `kurbo::Rect::intersect`, which clamps a rectangle off the document to zero size.
    let (x0, y0) = (doc_rect.x0.max(0.0), doc_rect.y0.max(0.0));
    let (x1, y1) = (doc_rect.x1.min(size.width), doc_rect.y1.min(size.height));
    let (width, height) = (x1 - x0, y1 - y0);
    if width < 0.0 || height < 0.0 {
        return false;
    }
    let aspect = if view.aspect.is_finite() && view.aspect > 0.0 { view.aspect } else { 1.0 };
    let ppp = if pixels_per_point.is_finite() && pixels_per_point > 0.0 { pixels_per_point } else { 1.0 };
    let canvas = view.rect.size();
    let device = if width < MIN_ZOOM_BOX || height < MIN_ZOOM_BOX || canvas.x <= 0.0 || canvas.y <= 0.0 {
        ZoomLadder::step(zoom::device_zoom(view, ppp), 1, size)
    } else {
        // Document columns shown across the canvas are `canvas.x / (point_zoom × aspect)`.
        let point_zoom = (f64::from(canvas.x) / (width * aspect)).min(f64::from(canvas.y) / height);
        point_zoom * f64::from(ppp)
    };
    if !device.is_finite() {
        return false;
    }
    let before = *view;
    view.zoom = CanvasView::points_per_unit(ZoomLadder::clamp(device, size), ppp);
    view.center = Point::new(x0 + width / 2.0, y0 + height / 2.0);
    *view != before
}

/// The document point under `at` in a preview drawn into `preview` for a document of `size`,
/// clamped to the document `(0, 0)..=size`: the view centre a click or a drag at `at` asks for.
pub fn view_center_for(preview: Rect, size: Size, at: Pos2) -> Point {
    // pc: crates/ui-egui/src/panels/navigator.rs:61-62 (`(pp − rect.min) / s`, clamped to the document).
    let s = f64::from(preview_scale(preview, size));
    Point::new((f64::from(at.x - preview.min.x) / s).clamp(0.0, size.width), (f64::from(at.y - preview.min.y) / s).clamp(0.0, size.height))
}

/// Preview points per document unit.
fn preview_scale(preview: Rect, size: Size) -> f32 {
    preview.width() / size.width.max(1.0) as f32
}
