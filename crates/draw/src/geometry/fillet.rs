//! Rounded corners of a polygon: where a vertex's circular rounding sits ([`fillet`]).
//!
//! The corner widgets (`craft_draw::ui::corner`) place, cap and drag their radius with it, and an
//! app's engine rounds its polygons and stars with it, so the widget always sits on the arc the
//! engine draws.

#![expect(clippy::todo, unused_variables, reason = "skeleton: removed when the body lands (task PE-A5)")]

use kurbo::{Point, Vec2};

/// A vertex rounded by a circular arc tangent to both its edges: see [`fillet`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fillet {
    /// The arc's centre.
    pub centre: Point,
    /// The arc's tangent point on the edge towards the previous vertex.
    pub from: Point,
    /// The arc's tangent point on the edge towards the next vertex.
    pub to: Point,
    /// The radius used: the one asked for, capped at what the edges allow.
    pub radius: f64,
    /// The unit direction from the vertex towards the arc's centre.
    pub bisector: Vec2,
    /// Half the vertex's inner angle, in radians, in 1e-6..=π/2 − 1e-6.
    pub half_angle: f64,
}

impl Fillet {
    /// How far the arc's centre moves per unit of radius: `bisector / sin(half_angle)`. A right
    /// angle's is `(±1, ±1)`. The corner widgets project a drag on it.
    pub fn step(&self) -> Vec2 {
        // pc: crates/ui-egui/src/tools/corner_radius.rs:75-79 (`bisector / sin`, `None` below 1e-9;
        // `half_angle` is never that small here).
        todo!()
    }
}

/// Where vertex `vertex` between `prev` and `next` is rounded with radius `radius`: the radius is
/// capped so both tangent points stay within half of each edge (`radius.min(min(l1, l2) / 2 *
/// tan(half))`, so neighbouring roundings never overlap); the centre is `bisector * radius /
/// sin(half)` from the vertex; the tangent points `radius / tan(half)` along each edge. A
/// rectangle's cap is `min(w, h) / 2`.
///
/// `None` for `radius` NaN or `<= 0`, an edge shorter than 1e-9, a half angle outside
/// 1e-6..=π/2 − 1e-6 (a straight or folded vertex), or a bisector shorter than 1e-9. Pass a very
/// large radius (PhotoCraft passes 1e12) to get the largest one the vertex takes.
pub fn fillet(prev: Point, vertex: Point, next: Point, radius: f64) -> Option<Fillet> {
    // pc: crates/vector/src/shapes.rs:128-165 `fillet` (constants as there).
    todo!()
}
