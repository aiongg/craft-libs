#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! [`IconPen`]: helpers for icons drawn in code (CadCraft's and FigCraft's tool icons, document
//! marks), on the 24-unit grid the icon sets are drawn on, at the suite's stroke width.
//!
//! Prefer an [`Icon`](super::Icon) from the set: a theme's icon pack restyles those. Code-drawn
//! icons are for what no set has (a CAD constraint glyph, a live preview of a setting).

use egui::{Color32, Painter, Pos2, Rect, Stroke};

/// A pen over `rect` with 24 units to its width, as Lucide's and Phosphor's viewboxes have:
/// `(x, y)` in units maps to `rect.min + (x, y) * rect.width() / 24`. Lines take the pen's
/// stroke width (in points, already scaled: [`IconSize::stroke`](super::IconSize::stroke) for the
/// suite's), radii and lengths are in units.
///
/// ```ignore
/// let pen = IconPen::new(&painter, rect, IconSize::Md.stroke(), color);
/// pen.circle(12.0, 12.0, 8.0);
/// pen.line(12.0, 4.0, 12.0, 20.0);
/// pen.arrow_tip(20.0, 12.0, 1.0, 0.0);
/// ```
#[derive(Clone, Copy)]
pub struct IconPen<'a> {
    painter: &'a Painter,
    rect: Rect,
    width: f32,
    color: Color32,
}

impl<'a> IconPen<'a> {
    pub fn new(painter: &'a Painter, rect: Rect, width: f32, color: Color32) -> Self {
        Self { painter, rect, width, color }
    }

    /// The same pen in another colour (CadCraft's accent strokes).
    pub fn with_color(self, color: Color32) -> Self {
        Self { color, ..self }
    }

    /// The point `(x, y)` in units.
    // cadcraft: crates/ui-egui/src/icons.rs:164 at
    pub fn at(&self, x: f32, y: f32) -> Pos2 {
        todo!()
    }

    /// Points per unit.
    // cadcraft: crates/ui-egui/src/icons.rs:168 sc
    pub fn scale(&self) -> f32 {
        todo!()
    }

    /// The pen's stroke.
    pub fn stroke(&self) -> Stroke {
        Stroke::new(self.width, self.color)
    }

    // cadcraft: crates/ui-egui/src/icons.rs:174 l
    pub fn line(&self, x1: f32, y1: f32, x2: f32, y2: f32) {
        todo!()
    }

    /// A polyline through `points`, back to the first when `closed`.
    // cadcraft: crates/ui-egui/src/icons.rs:180 poly
    pub fn polyline(&self, points: &[(f32, f32)], closed: bool) {
        todo!()
    }

    // cadcraft: crates/ui-egui/src/icons.rs:187 c
    pub fn circle(&self, x: f32, y: f32, r: f32) {
        todo!()
    }

    // cadcraft: crates/ui-egui/src/icons.rs:190 fill_c
    pub fn fill_circle(&self, x: f32, y: f32, r: f32) {
        todo!()
    }

    /// An arc from `a0` to `a1` degrees, counter-clockwise on screen (y up), in segments of at
    /// most 10° (at least 2).
    // cadcraft: crates/ui-egui/src/icons.rs:194 arc
    pub fn arc(&self, x: f32, y: f32, r: f32, a0: f32, a1: f32) {
        todo!()
    }

    // cadcraft: crates/ui-egui/src/icons.rs:204 rect
    pub fn rect(&self, x: f32, y: f32, w: f32, h: f32) {
        todo!()
    }

    // cadcraft: crates/ui-egui/src/icons.rs:207 fill_rect
    pub fn fill_rect(&self, x: f32, y: f32, w: f32, h: f32) {
        todo!()
    }

    /// A filled triangle.
    // cadcraft: crates/ui-egui/src/icons.rs:213 tri
    pub fn triangle(&self, points: [(f32, f32); 3]) {
        todo!()
    }

    /// A filled arrowhead with its tip at `(x, y)` pointing along `(dx, dy)`: 3 units long and
    /// 2.8 wide.
    // cadcraft: crates/ui-egui/src/icons.rs:216 arrow_tip
    pub fn arrow_tip(&self, x: f32, y: f32, dx: f32, dy: f32) {
        todo!()
    }

    /// A capsule (stadium) outline from `a` to `b` with radius `r`, in units.
    // figcraft: crates/ui-egui/src/icons.rs:250 capsule
    pub fn capsule(&self, a: (f32, f32), b: (f32, f32), r: f32) {
        todo!()
    }
}

/// The points of an arc around `(cx, cy)` of radius `r` from `a0` to `a1` radians (y down), in
/// `ceil(|a1 − a0| · r · 1.5)` steps clamped to 6–48: smooth at icon sizes without wasting
/// vertices.
// figcraft: crates/ui-egui/src/icons.rs:241 arc_pts
pub fn arc_points(cx: f32, cy: f32, r: f32, a0: f32, a1: f32) -> Vec<Pos2> {
    todo!()
}
