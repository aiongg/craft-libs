//! Fills of paths that are not one convex contour. epaint's tessellator fills convex polygons only,
//! so concave, self-intersecting and multi-contour paths are cut into trapezoids here: the plane
//! is split into horizontal bands at every vertex and every edge crossing, and inside a band the
//! edges do not cross, so sorted by x they give the winding number of each span between them.
//! The rule (non-zero or even-odd) then says which spans are inside.
//!
//! The trapezoids have hard edges. [`fringe`] adds the anti-aliasing: a one-pixel ramp outside each
//! contour edge that has the shape on one side only.

use craft_ui_rt::FillRule;
use emath::{Pos2, pos2};

/// One non-horizontal contour edge, `a` above `b` (smaller y first).
#[derive(Clone, Copy)]
struct Edge {
    a: (f64, f64),
    b: (f64, f64),
    /// +1 when the contour runs downwards along it, -1 upwards.
    dir: i32,
}

impl Edge {
    fn x_at(&self, y: f64) -> f64 {
        if y <= self.a.1 {
            self.a.0
        } else if y >= self.b.1 {
            self.b.0
        } else {
            self.a.0 + (self.b.0 - self.a.0) * (y - self.a.1) / (self.b.1 - self.a.1)
        }
    }
}

fn edges(contours: &[Vec<Pos2>]) -> Vec<Edge> {
    let mut out = Vec::new();
    for c in contours {
        let n = c.len();
        if n < 3 {
            continue;
        }
        for i in 0..n {
            let (p, q) = (c[i], c[(i + 1) % n]);
            if !(p.x.is_finite() && p.y.is_finite() && q.x.is_finite() && q.y.is_finite()) || p.y == q.y {
                continue;
            }
            let (p, q) = ((f64::from(p.x), f64::from(p.y)), (f64::from(q.x), f64::from(q.y)));
            out.push(if p.1 < q.1 { Edge { a: p, b: q, dir: 1 } } else { Edge { a: q, b: p, dir: -1 } });
        }
    }
    out
}

/// The y where two edges cross in both their interiors, if they do.
fn crossing(e: &Edge, f: &Edge) -> Option<f64> {
    let (d1x, d1y) = (e.b.0 - e.a.0, e.b.1 - e.a.1);
    let (d2x, d2y) = (f.b.0 - f.a.0, f.b.1 - f.a.1);
    let denom = d1x * d2y - d1y * d2x;
    if denom.abs() < 1e-12 {
        return None;
    }
    let (wx, wy) = (f.a.0 - e.a.0, f.a.1 - e.a.1);
    let t = (wx * d2y - wy * d2x) / denom;
    let u = (wx * d1y - wy * d1x) / denom;
    (t > 1e-9 && t < 1.0 - 1e-9 && u > 1e-9 && u < 1.0 - 1e-9).then_some(e.a.1 + t * d1y)
}

fn inside(winding: i32, rule: FillRule) -> bool {
    match rule {
        FillRule::NonZero => winding != 0,
        FillRule::EvenOdd => winding & 1 != 0,
    }
}

/// Triangles covering the inside of `contours` (each closed implicitly) by `rule`.
pub(crate) fn triangulate(contours: &[Vec<Pos2>], rule: FillRule) -> Vec<[Pos2; 3]> {
    let edges = edges(contours);
    if edges.len() < 3 {
        return Vec::new();
    }
    let mut ys: Vec<f64> = edges.iter().flat_map(|e| [e.a.1, e.b.1]).collect();
    for (i, e) in edges.iter().enumerate() {
        for f in &edges[i + 1..] {
            ys.extend(crossing(e, f));
        }
    }
    ys.sort_by(f64::total_cmp);
    ys.dedup_by(|a, b| (*a - *b).abs() < 1e-7);

    let mut out = Vec::new();
    let mut active: Vec<&Edge> = Vec::new();
    for band in ys.windows(2) {
        let (y0, y1) = (band[0], band[1]);
        let ym = (y0 + y1) / 2.0;
        active.clear();
        active.extend(edges.iter().filter(|e| e.a.1 <= ym && e.b.1 >= ym));
        active.sort_by(|e, f| e.x_at(ym).total_cmp(&f.x_at(ym)));
        let mut winding = 0;
        for pair in active.windows(2) {
            winding += pair[0].dir;
            if !inside(winding, rule) {
                continue;
            }
            let (l, r) = (pair[0], pair[1]);
            let p = |x: f64, y: f64| pos2(x as f32, y as f32);
            let (tl, tr, br, bl) = (p(l.x_at(y0), y0), p(r.x_at(y0), y0), p(r.x_at(y1), y1), p(l.x_at(y1), y1));
            out.push([tl, tr, br]);
            out.push([tl, br, bl]);
        }
    }
    out
}

fn winding_at(edges: &[Edge], x: f64, y: f64) -> i32 {
    edges.iter().filter(|e| e.a.1 <= y && y < e.b.1 && e.x_at(y) < x).map(|e| e.dir).sum()
}

/// Quads along the contour edges that have the shape on exactly one side: the full colour at the
/// edge and none `width` further out, on the outside. Corners are not mitred.
pub(crate) fn fringe(contours: &[Vec<Pos2>], rule: FillRule, width: f32) -> Vec<[Pos2; 4]> {
    let edges = edges(contours);
    let probe = f64::from(width) * 0.25;
    let mut out = Vec::new();
    for c in contours {
        let n = c.len();
        if n < 3 {
            continue;
        }
        for i in 0..n {
            let (p, q) = (c[i], c[(i + 1) % n]);
            let d = q - p;
            let len = d.length();
            if !len.is_finite() || len < 1e-6 {
                continue;
            }
            let normal = pos2(-d.y / len, d.x / len);
            let mid = (p.to_vec2() + q.to_vec2()) / 2.0;
            let sample = |sign: f32| {
                let s = (mid + normal.to_vec2() * (sign * probe as f32)).to_pos2();
                inside(winding_at(&edges, f64::from(s.x), f64::from(s.y)), rule)
            };
            let (left, right) = (sample(1.0), sample(-1.0));
            if left == right {
                continue;
            }
            // Out is the side without the shape.
            let out_dir = if left { -normal.to_vec2() } else { normal.to_vec2() };
            out.push([p, q, q + out_dir * width, p + out_dir * width]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(tris: &[[Pos2; 3]]) -> f32 {
        tris.iter().map(|[a, b, c]| ((b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y)).abs() / 2.0).sum()
    }

    fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<Pos2> {
        vec![pos2(x0, y0), pos2(x1, y0), pos2(x1, y1), pos2(x0, y1)]
    }

    #[test]
    fn an_l_shape_covers_its_area_and_not_its_notch() {
        // 10x10 square without the top-right 5x5.
        let l = vec![pos2(0.0, 0.0), pos2(5.0, 0.0), pos2(5.0, 5.0), pos2(10.0, 5.0), pos2(10.0, 10.0), pos2(0.0, 10.0)];
        let tris = triangulate(&[l], FillRule::NonZero);
        assert!((area(&tris) - 75.0).abs() < 1e-3, "area {}", area(&tris));
        let notch = pos2(7.5, 2.5);
        assert!(!tris.iter().any(|[a, b, c]| contains(*a, *b, *c, notch)));
    }

    fn contains(a: Pos2, b: Pos2, c: Pos2, p: Pos2) -> bool {
        let s = |u: Pos2, v: Pos2| (v.x - u.x) * (p.y - u.y) - (v.y - u.y) * (p.x - u.x);
        let (x, y, z) = (s(a, b), s(b, c), s(c, a));
        (x >= 0.0 && y >= 0.0 && z >= 0.0) || (x <= 0.0 && y <= 0.0 && z <= 0.0)
    }

    #[test]
    fn a_hole_is_left_out_by_both_rules_when_wound_against_the_outer() {
        let outer = rect(0.0, 0.0, 10.0, 10.0);
        let mut hole = rect(3.0, 3.0, 7.0, 7.0);
        hole.reverse();
        for rule in [FillRule::NonZero, FillRule::EvenOdd] {
            let tris = triangulate(&[outer.clone(), hole.clone()], rule);
            assert!((area(&tris) - 84.0).abs() < 1e-3, "{rule:?}: {}", area(&tris));
        }
    }

    #[test]
    fn a_same_direction_inner_contour_is_filled_by_non_zero_and_a_hole_by_even_odd() {
        let (outer, inner) = (rect(0.0, 0.0, 10.0, 10.0), rect(3.0, 3.0, 7.0, 7.0));
        assert!((area(&triangulate(&[outer.clone(), inner.clone()], FillRule::NonZero)) - 100.0).abs() < 1e-3);
        assert!((area(&triangulate(&[outer, inner], FillRule::EvenOdd)) - 84.0).abs() < 1e-3);
    }

    #[test]
    fn a_five_pointed_star_differs_by_rule() {
        // The pentagram: non-zero fills the centre, even-odd leaves the pentagon open.
        let star: Vec<Pos2> = (0..5)
            .map(|i| {
                let a = (i * 2) as f32 * std::f32::consts::TAU / 5.0 - std::f32::consts::FRAC_PI_2;
                pos2(10.0 * a.cos(), 10.0 * a.sin())
            })
            .collect();
        let nonzero = area(&triangulate(std::slice::from_ref(&star), FillRule::NonZero));
        let evenodd = area(&triangulate(std::slice::from_ref(&star), FillRule::EvenOdd));
        assert!(nonzero > evenodd + 10.0, "{nonzero} vs {evenodd}");
        let centre = triangulate(std::slice::from_ref(&star), FillRule::EvenOdd);
        assert!(!centre.iter().any(|[a, b, c]| contains(*a, *b, *c, pos2(0.0, 0.0))));
    }

    #[test]
    fn the_fringe_runs_along_the_outside_of_boundary_edges_only() {
        let outer = rect(0.0, 0.0, 10.0, 10.0);
        let inner = rect(3.0, 3.0, 7.0, 7.0);
        // Same direction under non-zero: the inner contour's edges have the shape on both sides.
        assert_eq!(fringe(&[outer.clone(), inner.clone()], FillRule::NonZero, 1.0).len(), 4);
        // Under even-odd the inner edges bound a hole, so they get a ramp as well.
        assert_eq!(fringe(&[outer, inner], FillRule::EvenOdd, 1.0).len(), 8);
        let quad = &fringe(&[rect(0.0, 0.0, 10.0, 10.0)], FillRule::NonZero, 1.0)[0];
        assert!(quad.iter().all(|p| p.x >= -1.0 && p.x <= 11.0 && p.y >= -1.0 && p.y <= 11.0));
        assert!(quad.iter().any(|p| p.y < 0.0 || p.x > 10.0 || p.y > 10.0 || p.x < 0.0));
    }
}
