#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! Painter helpers that several apps wrote for themselves: vertical gradients, hatching, filled
//! polygons of any shape, text that fits a box, and ink that reads on a fill.

use craft_theme::TypeRole;
use egui::{Align2, Color32, FontId, Mesh, Painter, Pos2, Rect};

use crate::theme::UiTokens;

/// A rect shaded from `top` to `bottom` (one mesh, colours blended in gamma space as egui's
/// `lerp_to_gamma`).
// effectcraft: crates/ui-egui/src/theme.rs:115 gradient_rect; soundcraft: crates/ui-egui/src/widgets.rs:24 vgradient
pub fn paint_vertical_gradient(painter: &Painter, rect: Rect, top: Color32, bottom: Color32) {
    todo!()
}

/// A rounded rect shaded from `top` to `bottom`, with egui's anti-aliased edges: the rect is
/// tessellated filled white, then each vertex takes the gradient's colour at its height times its
/// own alpha (the feathered edge).
// soundcraft: crates/ui-egui/src/widgets.rs:31 rounded_vgradient
pub fn paint_rounded_vertical_gradient(painter: &Painter, rect: Rect, radius: f32, top: Color32, bottom: Color32) {
    todo!()
}

/// Diagonal hatching over `rect`, clipped to it: 1-point lines from bottom left to top right,
/// `spacing` apart along the bottom edge (FilmCraft's transitions: 4 points, light grey at 110 of
/// 255).
// filmcraft: crates/ui-egui/src/panels/timeline/clips.rs:305 paint_hatch
pub fn paint_hatch(painter: &Painter, rect: Rect, spacing: f32, color: Color32) {
    todo!()
}

/// Fills a simple polygon of any shape: a convex one through egui's convex polygon (anti-aliased
/// edges), any other through [`triangulate`]; nothing for fewer than three points or a polygon
/// that can't be split.
// gridcraft: crates/ui-egui/src/chartview.rs:152 is_convex, :173 area_mesh; figcraft: crates/ui-egui/src/icons.rs:256 triangulate
pub fn fill_polygon(painter: &Painter, points: &[Pos2], color: Color32) {
    todo!()
}

/// Whether `points` turn one way only (collinear runs, within 0.001, don't count).
// gridcraft: crates/ui-egui/src/chartview.rs:152 is_convex
pub fn is_convex(points: &[Pos2]) -> bool {
    todo!()
}

/// The triangles of a simple polygon by ear clipping, as indices into `points` (three per
/// triangle); either winding; empty when it has fewer than three points or can't be split.
// figcraft: crates/ui-egui/src/icons.rs:256 triangulate
pub fn triangulate(points: &[Pos2]) -> Vec<u32> {
    todo!()
}

/// A mesh of the polygon filled in `color`, from [`triangulate`] (empty when it can't be split).
pub fn polygon_mesh(points: &[Pos2], color: Color32) -> Mesh {
    todo!()
}

/// The font that makes `text` fit `room` points wide: `None` when `font` already fits, else the
/// same family at `size · room / natural · 0.97` (glyph advances don't scale exactly; at least
/// 1 point). For text inside a box whose size the content sets (a cell, a chart label), not for
/// interface text, which keeps its type role.
// gridcraft: crates/ui-egui/src/grid/cell_paint.rs:219 shrink_font
pub fn fit_font(painter: &Painter, text: &str, font: &FontId, room: f32) -> Option<FontId> {
    todo!()
}

/// Black or white, whichever reads on `fill`: white below a luma (0.299 R + 0.587 G + 0.114 B)
/// of 150, else black. For document colours (a cell fill, a label colour); interface colours
/// take their `text.on_*` token.
// gridcraft: crates/ui-egui/src/grid/cell_paint.rs:297 ink_on
pub fn ink_on(fill: Color32) -> Color32 {
    todo!()
}

/// One line of text in `role` at `pos`, aligned by `align`, in `color`; returns its rect. For
/// painters that place small labels (a meter's dB marks, a ruler's numbers) where a widget can't
/// go; the size comes from the role, never a number (FilmCraft's `small_text` took a size).
// filmcraft: crates/ui-egui/src/panels/mixer.rs:184 small_text
pub fn paint_role_text(painter: &Painter, tokens: &UiTokens, pos: Pos2, align: Align2, text: &str, role: TypeRole, color: Color32) -> Rect {
    todo!()
}
