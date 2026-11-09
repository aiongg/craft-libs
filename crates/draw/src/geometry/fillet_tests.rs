//! Tests of rounded corners.

use kurbo::Point;

use super::fillet::fillet;

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

// The tangent points lie on the two edges at the same distance from the vertex, and the arc ends on the circle round the
// centre; a radius over the limit is capped, and a smaller one is kept.
#[test]
fn fillet_matches_photocraft() {
    let (a, b, c) = (Point::new(0.0, 0.0), Point::new(100.0, 0.0), Point::new(0.0, 60.0));
    for (p, q) in [(b, c), (Point::new(10.0, 0.0), Point::new(5.0, 75f64.sqrt()))] {
        let Some(f) = fillet(p, a, q, 1e12) else { panic!("a rounded vertex") };
        assert!((f.from - a).hypot() > 0.0 && near((f.from - a).hypot(), (f.to - a).hypot()), "equal tangent lengths");
        let on = |t: Point, end: Point| ((t - a).cross(end - a)).abs() < 1e-6;
        assert!(on(f.from, p) && on(f.to, q), "the tangent points lie on the edges");
        assert!(near((f.from - f.centre).hypot(), f.radius) && near((f.to - f.centre).hypot(), f.radius));
        let Some(small) = fillet(p, a, q, f.radius / 2.0) else { panic!("a small radius rounds") };
        assert!(near(small.radius, f.radius / 2.0), "a radius under the cap is kept");
    }
}

// `None` for a zero radius, NaN, a zero-length edge, a straight and a folded vertex.
#[test]
fn fillet_refuses_degenerate_vertices() {
    let (a, b, c) = (Point::new(0.0, 0.0), Point::new(10.0, 0.0), Point::new(0.0, 10.0));
    assert!(fillet(b, a, c, 0.0).is_none());
    assert!(fillet(b, a, c, f64::NAN).is_none());
    assert!(fillet(a, a, c, 5.0).is_none());
    assert!(fillet(b, a, Point::new(-10.0, 0.0), 5.0).is_none());
    assert!(fillet(b, a, Point::new(20.0, 0.0), 5.0).is_none());
}
