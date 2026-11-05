//! Tests of the Curvature Pen's handles.

use kurbo::Point;

use super::curvature::curvature_handles;

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

// An inner anchor's handles are collinear, parallel to the neighbours' chord, and a third of the
// distance to each neighbour long (uneven spacing).
#[test]
fn curvature_handles_are_collinear_and_one_third_of_the_neighbour_distance() {
    let a = [p(0.0, 0.0), p(30.0, 40.0), p(120.0, 40.0), p(150.0, 0.0)];
    let h = curvature_handles(&a, &[], false);
    assert_eq!(h.len(), 4);
    let (i, o) = h[1];
    let chord = a[2] - a[0];
    let cross = |u: kurbo::Vec2, v: kurbo::Vec2| u.x * v.y - u.y * v.x;
    assert!(cross(o - a[1], chord).abs() < 1e-9);
    assert!(cross(i - a[1], chord).abs() < 1e-9);
    assert!((a[1].distance(i) - a[1].distance(a[0]) / 3.0).abs() < 1e-9);
    assert!((a[1].distance(o) - a[1].distance(a[2]) / 3.0).abs() < 1e-9);
    assert!((a[1] - i).dot(o - a[1]) > 0.0);
}

// An open path's first and last anchors aim at their neighbour's nearer handle, a third of the
// distance long; their outer handles stay on the anchor.
#[test]
fn curvature_open_ends_aim_at_the_neighbours_nearer_handle() {
    let a = [p(0.0, 0.0), p(30.0, 40.0), p(120.0, 40.0), p(150.0, 0.0)];
    let h = curvature_handles(&a, &[], false);
    assert_eq!(h[0].0, a[0]);
    assert_eq!(h[3].1, a[3]);
    let dir = (h[1].0 - a[0]).normalize();
    let got = h[0].1 - a[0];
    assert!((got.normalize() - dir).hypot() < 1e-9);
    assert!((got.hypot() - a[0].distance(a[1]) / 3.0).abs() < 1e-9);
    let dir = (h[2].1 - a[3]).normalize();
    let got = h[3].0 - a[3];
    assert!((got.normalize() - dir).hypot() < 1e-9);
    assert!((got.hypot() - a[3].distance(a[2]) / 3.0).abs() < 1e-9);
}

// `corners[i]` keeps anchor i's handles on the anchor; one or two anchors give straight segments,
// open or closed.
#[test]
fn curvature_corners_retract_and_two_anchors_are_straight() {
    let a = [p(0.0, 0.0), p(30.0, 40.0), p(120.0, 40.0), p(150.0, 0.0)];
    let h = curvature_handles(&a, &[false, true], true);
    assert_eq!(h[1], (a[1], a[1]));
    assert_ne!(h[2], (a[2], a[2]));
    for closed in [false, true] {
        for n in 1..=2 {
            let h = curvature_handles(&a[..n], &[], closed);
            assert!(h.iter().zip(&a).all(|(h, a)| *h == (*a, *a)));
        }
    }
}

// Two or three coincident anchors give handles on the anchor and no NaN.
#[test]
fn curvature_coincident_points_give_retracted_handles() {
    for n in [2, 3] {
        let a = vec![p(5.0, 5.0); n];
        for closed in [false, true] {
            let h = curvature_handles(&a, &[], closed);
            assert!(h.iter().all(|&(i, o)| i == a[0] && o == a[0]));
        }
    }
}
