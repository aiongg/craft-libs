//! [`crisp`]: lines on whole device pixels.

use egui::epaint::Mesh;
use egui::{Painter, Pos2, Rect, Shape, Stroke, pos2};

/// Draws the line from `a` to `b` with `stroke`. A horizontal or vertical line covers whole device
/// pixels, at least one, in a mesh without antialiasing, so a 1 point line is as sharp at 1.25 or
/// 1.5 pixels per point as at 1 (PhotoCraft's smart guides' `round() + 0.5`, for any scale). A
/// slanted line is drawn as it is, antialiased.
pub fn crisp(painter: &Painter, a: Pos2, b: Pos2, stroke: impl Into<Stroke>) {
    painter.add(crisp_shape(a, b, stroke.into(), painter.pixels_per_point()));
}

/// The shape [`crisp`] draws at `pixels_per_point`.
pub fn crisp_shape(a: Pos2, b: Pos2, stroke: Stroke, pixels_per_point: f32) -> Shape {
    let ppp = pixels_per_point;
    let horizontal = (a.y - b.y).abs() * ppp < 0.01;
    let vertical = (a.x - b.x).abs() * ppp < 0.01;
    if stroke.is_empty() || !(horizontal || vertical) || !ppp.is_finite() || ppp <= 0.0 {
        return Shape::line_segment([a, b], stroke);
    }
    let width = (stroke.width * ppp).round().max(1.0);
    // Across the line: `width` device pixels starting at the rounded position, so a 1-pixel line
    // fills the pixel row below a position on the grid.
    let across = |v: f32| {
        let start = (v * ppp).round() - (width / 2.0).floor();
        (start / ppp, (start + width) / ppp)
    };
    // Along the line: from pixel edge to pixel edge.
    let along = |u: f32, v: f32| {
        let (lo, hi) = (u.min(v), u.max(v));
        ((lo * ppp).round() / ppp, ((hi * ppp).round().max((lo * ppp).round() + 1.0)) / ppp)
    };
    let rect = if horizontal {
        let ((y0, y1), (x0, x1)) = (across(a.y), along(a.x, b.x));
        Rect::from_min_max(pos2(x0, y0), pos2(x1, y1))
    } else {
        let ((x0, x1), (y0, y1)) = (across(a.x), along(a.y, b.y));
        Rect::from_min_max(pos2(x0, y0), pos2(x1, y1))
    };
    let mut mesh = Mesh::default();
    mesh.add_colored_rect(rect, stroke.color);
    Shape::mesh(mesh)
}
