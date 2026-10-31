#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! Plane geometry the canvas overlays share, in document or screen units: point in polygon,
//! segment clipping, a vector's angle, and a polygon carried through a projective map with the
//! part behind the camera cut off. Pure maths beside [`quad`](super::quad), on kurbo types.
//!
//! Curve flattening is kurbo's ([`kurbo::flatten`]); the
//! projective map of a box is [`quad::Homography`](super::quad::Homography).

use kurbo::{Point, Rect, Vec2};

/// The least homogeneous depth `w` a projected point keeps: points at or behind a perspective
/// camera would flip through infinity and draw long lines across the view (EffectCraft issue
/// 572). 0.5, EffectCraft's `NEAR_W`.
pub const NEAR_W: f64 = 0.5;

/// Whether `p` is inside the polygon `points` (closed implicitly) by the nonzero winding rule,
/// so a self-crossing outline is filled where it winds (FilmCraft's mask hit test).
// filmcraft: crates/ui-egui/src/panels/masks.rs:219 inside
pub fn point_in_polygon(p: Point, points: &[Point]) -> bool {
    todo!()
}

/// The part of segment `a`–`b` inside `rect` (Liang–Barsky), or `None` when none is.
// vectorcraft: crates/ui-egui/src/canvas/grid.rs:90 clip_segment
pub fn clip_segment(a: Point, b: Point, rect: Rect) -> Option<(Point, Point)> {
    todo!()
}

/// `v`'s angle from the x axis in degrees, −180 to 180, positive towards +y (clockwise on a
/// y-down screen).
// filmcraft: crates/ui-egui/src/panels/transform_handles.rs:298 angle_deg
pub fn angle_deg(v: Vec2) -> f64 {
    todo!()
}

/// A 3 × 3 projective matrix, rows of (x, y, w) coefficients applied to (x, y, 1).
pub type Projective = [[f64; 3]; 3];

/// The closed polygon `points` through `m`, cut at `w = NEAR_W` before the perspective divide, so
/// only the part in front of the camera is drawn; empty when none of it is. Points that come out
/// non-finite are dropped.
// effectcraft: crates/ui-egui/src/panels/viewer/geometry.rs:51 project_clipped
pub fn project_clipped(m: &Projective, points: &[Point]) -> Vec<Point> {
    todo!()
}
