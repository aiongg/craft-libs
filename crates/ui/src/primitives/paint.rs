//! Painter helpers that several apps wrote for themselves: vertical gradients, hatching, filled
//! polygons of any shape, text that fits a box, and ink that reads on a fill.

use craft_theme::{Rgba, TypeRole, contrast_ratio};
use egui::epaint::{RectShape, Tessellator};
use egui::text::LayoutJob;
use egui::{Align2, Color32, FontId, Mesh, Painter, Pos2, Rect, Shape, Stroke};

use crate::theme::{UiTokens, role_format};

/// A rect shaded from `top` to `bottom` (one mesh, colours blended in gamma space as egui's
/// `lerp_to_gamma`).
// effectcraft: crates/ui-egui/src/theme.rs:115 gradient_rect; soundcraft: crates/ui-egui/src/widgets.rs:24 vgradient
pub fn paint_vertical_gradient(painter: &Painter, rect: Rect, top: Color32, bottom: Color32) {
    let mut mesh = Mesh::default();
    for (pos, color) in [(rect.left_top(), top), (rect.right_top(), top), (rect.left_bottom(), bottom), (rect.right_bottom(), bottom)] {
        mesh.colored_vertex(pos, color);
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 2, 3);
    painter.add(mesh);
}

/// A rounded rect shaded from `top` to `bottom`, with egui's anti-aliased edges: the rect is
/// tessellated filled white, then each vertex takes the gradient's colour at its height times its
/// own alpha (the feathered edge).
// soundcraft: crates/ui-egui/src/widgets.rs:31 rounded_vgradient
pub fn paint_rounded_vertical_gradient(painter: &Painter, rect: Rect, radius: f32, top: Color32, bottom: Color32) {
    let options = painter.ctx().tessellation_options(|options| *options);
    let mut tessellator = Tessellator::new(painter.pixels_per_point(), options, [1, 1], Vec::new());
    let mut mesh = Mesh::default();
    tessellator.tessellate_rect(&RectShape::filled(rect, radius, Color32::WHITE), &mut mesh);
    for vertex in &mut mesh.vertices {
        let t = if rect.height() > 0.0 { ((vertex.pos.y - rect.min.y) / rect.height()).clamp(0.0, 1.0) } else { 0.0 };
        vertex.color = top.lerp_to_gamma(bottom, t).gamma_multiply(f32::from(vertex.color.a()) / 255.0);
    }
    painter.add(mesh);
}

/// Diagonal hatching over `rect`, clipped to it: 1-point lines from bottom left to top right,
/// `spacing` apart along the bottom edge (FilmCraft's transitions: 4 points, light grey at 110 of
/// 255).
// filmcraft: crates/ui-egui/src/panels/timeline/clips.rs:305 paint_hatch
pub fn paint_hatch(painter: &Painter, rect: Rect, spacing: f32, color: Color32) {
    if spacing <= 0.0 {
        return;
    }
    let clipped = painter.with_clip_rect(rect.intersect(painter.clip_rect()));
    let stroke = Stroke::new(1.0, color);
    let mut x = rect.min.x - rect.height();
    while x < rect.max.x {
        clipped.line_segment([Pos2::new(x, rect.max.y), Pos2::new(x + rect.height(), rect.min.y)], stroke);
        x += spacing;
    }
}

/// Fills a simple polygon of any shape: a convex one through egui's convex polygon (anti-aliased
/// edges), any other through [`triangulate`]; nothing for fewer than three points or a polygon
/// that can't be split.
// gridcraft: crates/ui-egui/src/chartview.rs:152 is_convex, :173 area_mesh; figcraft: crates/ui-egui/src/icons.rs:256 triangulate
pub fn fill_polygon(painter: &Painter, points: &[Pos2], color: Color32) {
    if points.len() < 3 {
        return;
    }
    if is_convex(points) {
        painter.add(Shape::convex_polygon(points.to_vec(), color, Stroke::NONE));
    } else {
        let mesh = polygon_mesh(points, color);
        if !mesh.is_empty() {
            painter.add(mesh);
        }
    }
}

/// Whether `points` turn one way only (collinear runs, within 0.001, don't count).
// gridcraft: crates/ui-egui/src/chartview.rs:152 is_convex
pub fn is_convex(points: &[Pos2]) -> bool {
    let n = points.len();
    let mut sign = 0.0_f32;
    for i in 0..n {
        let (a, b, c) = (points[i], points[(i + 1) % n], points[(i + 2) % n]);
        let cross = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x);
        if cross.abs() > 1e-3 {
            if sign == 0.0 {
                sign = cross.signum();
            } else if cross.signum() != sign {
                return false;
            }
        }
    }
    true
}

/// The triangles of a simple polygon by ear clipping, as indices into `points` (three per
/// triangle); either winding; empty when it has fewer than three points or can't be split.
// figcraft: crates/ui-egui/src/icons.rs:256 triangulate
pub fn triangulate(points: &[Pos2]) -> Vec<u32> {
    let n = points.len();
    if n < 3 {
        return Vec::new();
    }
    let area: f32 = (0..n).map(|i| points[i].x * points[(i + 1) % n].y - points[(i + 1) % n].x * points[i].y).sum();
    let mut indices: Vec<usize> = if area >= 0.0 { (0..n).collect() } else { (0..n).rev().collect() };
    let cross = |a: Pos2, b: Pos2, c: Pos2| (b - a).x * (c - b).y - (b - a).y * (c - b).x;
    let mut triangles = Vec::with_capacity((n - 2) * 3);
    while indices.len() > 3 {
        let m = indices.len();
        let ear = (0..m).find(|&i| {
            let (ia, ib, ic) = (indices[(i + m - 1) % m], indices[i], indices[(i + 1) % m]);
            let (a, b, c) = (points[ia], points[ib], points[ic]);
            cross(a, b, c) > 0.0
                && !indices.iter().any(|&k| {
                    k != ia && k != ib && k != ic && cross(a, b, points[k]) >= 0.0 && cross(b, c, points[k]) >= 0.0 && cross(c, a, points[k]) >= 0.0
                })
        });
        let Some(i) = ear else { return Vec::new() };
        triangles.extend([indices[(i + m - 1) % m] as u32, indices[i] as u32, indices[(i + 1) % m] as u32]);
        indices.remove(i);
    }
    triangles.extend(indices.iter().map(|&i| i as u32));
    triangles
}

/// A mesh of the polygon filled in `color`, from [`triangulate`] (empty when it can't be split).
pub fn polygon_mesh(points: &[Pos2], color: Color32) -> Mesh {
    let mut mesh = Mesh::default();
    let triangles = triangulate(points);
    if triangles.is_empty() {
        return mesh;
    }
    for &point in points {
        mesh.colored_vertex(point, color);
    }
    mesh.indices = triangles;
    mesh
}

/// The font that makes `text` fit `room` points wide: `None` when `font` already fits, else the
/// same family at `size · room / natural · 0.97` (glyph advances don't scale exactly; at least
/// 1 point). For text inside a box whose size the content sets (a cell, a chart label), not for
/// interface text, which keeps its type role.
// gridcraft: crates/ui-egui/src/grid/cell_paint.rs:219 shrink_font
pub fn fit_font(painter: &Painter, text: &str, font: &FontId, room: f32) -> Option<FontId> {
    let natural = painter.layout_no_wrap(text.to_string(), font.clone(), Color32::PLACEHOLDER).size().x;
    let room = room.max(1.0);
    if natural.is_nan() || natural <= room {
        return None;
    }
    Some(FontId::new((font.size * room / natural * 0.97).max(1.0), font.family.clone()))
}

/// Black or white, whichever has the higher WCAG contrast on `fill`
/// ([`craft_theme::contrast_ratio`]); a tie is white. For document colours (a cell fill, a label
/// colour); interface colours take their `text.on_*` token. GridCraft's luma rule (white below
/// 0.299 R + 0.587 G + 0.114 B = 150) disagrees on mid tones: on green `#00C800` it picks
/// white at 2.3:1 where black has 9.3:1. The suite takes the contrast rule craft-update's
/// prompt already uses (its private `ink_on` becomes a call to this one).
// gridcraft: crates/ui-egui/src/grid/cell_paint.rs:296 ink_on; craft-update: crates/update/src/dialog.rs:436 ink_on
pub fn ink_on(fill: Color32) -> Color32 {
    let to_rgba = |c: Color32| Rgba::rgb(c.r(), c.g(), c.b());
    let fill = to_rgba(fill);
    if contrast_ratio(fill, to_rgba(Color32::WHITE)) >= contrast_ratio(fill, to_rgba(Color32::BLACK)) { Color32::WHITE } else { Color32::BLACK }
}

/// One line of text in `role` at `pos`, aligned by `align`, in `color`; returns its rect. For
/// painters that place small labels (a meter's dB marks, a ruler's numbers) where a widget can't
/// go; the size comes from the role, never a number (FilmCraft's `small_text` took a size).
// filmcraft: crates/ui-egui/src/panels/mixer.rs:184 small_text
pub fn paint_role_text(painter: &Painter, tokens: &UiTokens, pos: Pos2, align: Align2, text: &str, role: TypeRole, color: Color32) -> Rect {
    let format = role_format(tokens, role, color);
    let galley = painter.layout_job(LayoutJob::single_section(text.to_string(), format));
    let rect = align.anchor_size(pos, galley.size());
    painter.galley(rect.min, galley, color);
    rect
}
