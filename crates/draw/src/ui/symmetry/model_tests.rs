//! Tests of the symmetry model (task TO-S1).

use super::{MAX_COORDINATE, Symmetry, SymmetryMode};
use kurbo::{Point, Size};

fn sym(mode: SymmetryMode, rotation: f64) -> Symmetry {
    let Some(s) = Symmetry::new(mode, Point::new(20.0, 30.0), rotation) else {
        panic!("a valid symmetry");
    };
    s
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

// `axes`: Vertical and Dual start at 90°, Horizontal at 0°, Diagonal at 45° (its direction's x equals y), plus the rotation clockwise; Dual has two.
#[test]
fn axes_start_at_90_0_45_degrees_plus_rotation_and_dual_adds_the_perpendicular() {
    let v = sym(SymmetryMode::Vertical, 0.0).axes();
    assert!(v.len() == 1 && near(v[0].x, 0.0) && near(v[0].y, 1.0));
    let h = sym(SymmetryMode::Horizontal, 0.0).axes();
    assert!(near(h[0].x, 1.0) && near(h[0].y, 0.0));
    let d = sym(SymmetryMode::Diagonal, 0.0).axes();
    assert!(near(d[0].x, d[0].y));
    let turned = sym(SymmetryMode::Horizontal, 90.0).axes();
    assert!(near(turned[0].x, 0.0) && near(turned[0].y, 1.0));
    let dual = sym(SymmetryMode::Dual, 0.0).axes();
    assert_eq!(dual.len(), 2);
    assert!(near(dual[0].dot(dual[1]), 0.0));
}

// PhotoCraft's cases: every mode on 100 x 100 and 101 x 53 gives segments inside the page; a horizontal axis at y -5 gives none.
#[test]
fn segments_clip_to_the_page_and_vanish_when_the_centre_line_misses_it() {
    for size in [Size::new(100.0, 100.0), Size::new(101.0, 53.0)] {
        for mode in SymmetryMode::ALL {
            let Some(s) = Symmetry::new(mode, Point::new(size.width / 2.0, size.height / 2.0), 0.0) else {
                panic!("a valid symmetry");
            };
            let segments = s.segments(size);
            assert_eq!(segments.len(), if mode == SymmetryMode::Dual { 2 } else { 1 });
            for line in segments {
                for p in [line.p0, line.p1] {
                    assert!(p.x >= -1e-9 && p.x <= size.width + 1e-9 && p.y >= -1e-9 && p.y <= size.height + 1e-9);
                }
            }
        }
    }
    let Some(off) = Symmetry::new(SymmetryMode::Horizontal, Point::new(50.0, -5.0), 0.0) else {
        panic!("a valid symmetry");
    };
    assert!(off.segments(Size::new(100.0, 100.0)).is_empty());
}

// `Symmetry::new` refuses NaN, infinity and a centre past 1,000,000 and wraps 370 to 10 and -90 to 270.
#[test]
fn new_rejects_non_finite_and_wraps_rotation() {
    let o = Point::ORIGIN;
    assert!(Symmetry::new(SymmetryMode::Dual, Point::new(f64::NAN, 0.0), 0.0).is_none());
    assert!(Symmetry::new(SymmetryMode::Dual, o, f64::INFINITY).is_none());
    assert!(Symmetry::new(SymmetryMode::Dual, Point::new(0.0, MAX_COORDINATE + 1.0), 0.0).is_none());
    assert!(near(sym(SymmetryMode::Dual, 370.0).rotation, 10.0));
    assert!(near(sym(SymmetryMode::Dual, -90.0).rotation, 270.0));
}

// At 0.35 of the shorter side along the first axis, at least 1 unit from the centre.
#[test]
fn the_rotation_handle_sits_at_a_share_of_the_shorter_side() {
    let s = sym(SymmetryMode::Horizontal, 0.0);
    let h = s.rotation_handle(Size::new(200.0, 100.0), 0.35);
    assert!(near(h.x, 55.0) && near(h.y, 30.0));
    let tiny = s.rotation_handle(Size::new(2.0, 2.0), 0.35);
    assert!(near(tiny.x, 21.0));
}
