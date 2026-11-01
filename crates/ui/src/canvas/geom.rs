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
    let n = points.len();
    let mut winding = 0;
    for i in 0..n {
        let (a, b) = (points[i], points[(i + 1) % n]);
        if (a.y <= p.y) != (b.y <= p.y) && a.x + (b.x - a.x) * (p.y - a.y) / (b.y - a.y) < p.x {
            winding += if b.y > a.y { 1 } else { -1 };
        }
    }
    winding != 0
}

/// The part of segment `a`–`b` inside `rect` (Liang–Barsky), or `None` when none is.
// vectorcraft: crates/ui-egui/src/canvas/grid.rs:90 clip_segment
pub fn clip_segment(a: Point, b: Point, rect: Rect) -> Option<(Point, Point)> {
    let d = b - a;
    let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
    // Each edge as (the run toward it, the room before it).
    for (run, room) in [(-d.x, a.x - rect.x0), (d.x, rect.x1 - a.x), (-d.y, a.y - rect.y0), (d.y, rect.y1 - a.y)] {
        if run == 0.0 {
            if room < 0.0 {
                return None;
            }
        } else if run < 0.0 {
            t0 = t0.max(room / run);
        } else {
            t1 = t1.min(room / run);
        }
    }
    (t0 <= t1).then(|| (a + d * t0, a + d * t1))
}

/// `v`'s angle from the x axis in degrees, −180 to 180, positive towards +y (clockwise on a
/// y-down screen).
// filmcraft: crates/ui-egui/src/panels/transform_handles.rs:298 angle_deg
pub fn angle_deg(v: Vec2) -> f64 {
    v.y.atan2(v.x).to_degrees()
}

/// A 3 × 3 projective matrix, rows of (x, y, w) coefficients applied to (x, y, 1).
pub type Projective = [[f64; 3]; 3];

/// The closed polygon `points` through `m`, cut at `w = NEAR_W` before the perspective divide, so
/// only the part in front of the camera is drawn; empty when none of it is. Points that come out
/// non-finite are dropped.
// effectcraft: crates/ui-egui/src/panels/viewer/geometry.rs:51 project_clipped
pub fn project_clipped(m: &Projective, points: &[Point]) -> Vec<Point> {
    let homogeneous: Vec<[f64; 3]> = points
        .iter()
        .map(|p| {
            let row = |i: usize| m[i][0] * p.x + m[i][1] * p.y + m[i][2];
            [row(0), row(1), row(2)]
        })
        .collect();
    let mut out = Vec::new();
    for (i, a) in homogeneous.iter().enumerate() {
        let b = &homogeneous[(i + 1) % homogeneous.len()];
        let (a_in, b_in) = (a[2] >= NEAR_W, b[2] >= NEAR_W);
        if a_in {
            out.push(*a);
        }
        if a_in != b_in {
            let t = (NEAR_W - a[2]) / (b[2] - a[2]);
            out.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, NEAR_W]);
        }
    }
    out.iter().map(|q| Point::new(q[0] / q[2], q[1] / q[2])).filter(|q| q.x.is_finite() && q.y.is_finite()).collect()
}
