//! Tests of the Ruler line model (task TO-R1).

use kurbo::Point;

use super::model::{RulerLine, RulerPart};

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

// X, Y, W, H, A and L1 of a line, and A (0 to 180) and L2 with a protractor, equal PhotoCraft's `ruler_info` cases (y up for the angle, -180 to 180).
#[test]
fn reading_matches_the_engines_ruler_info() {
    // Down-right on screen is below the x axis with y up: -45 degrees.
    let r = RulerLine::new(Point::new(10.0, 20.0), Point::new(13.0, 24.0)).reading();
    assert_eq!((r.x, r.y, r.w, r.h, r.l1, r.l2), (10.0, 20.0, 3.0, 4.0, 5.0, None));
    assert!(near(r.angle, -(4.0f64).atan2(3.0).to_degrees()));
    // Up-left: 135 degrees.
    let r = RulerLine::new(Point::new(0.0, 0.0), Point::new(-1.0, -1.0)).reading();
    assert!(near(r.angle, 135.0));
    // Protractor: the angle between the arms, 0 to 180, and the second arm's length.
    let line = RulerLine { start: Point::new(0.0, 0.0), end: Point::new(10.0, 0.0), protractor: Some(Point::new(0.0, -10.0)) };
    let r = line.reading();
    assert!(near(r.angle, 90.0) && near(r.l1, 10.0) && r.l2.is_some_and(|l| near(l, 10.0)));
    let line = RulerLine { protractor: Some(Point::new(-5.0, 5.0)), ..line };
    assert!(near(line.reading().angle, 135.0));
    let line = RulerLine { start: Point::new(0.0, 0.0), end: Point::new(10.0, -1.0), protractor: Some(Point::new(10.0, 1.0)) };
    assert!(line.reading().angle >= 0.0 && line.reading().angle <= 180.0);
}

// `RulerReading::scaled` multiplies X, Y, W, H, L1 and L2 and keeps A.
#[test]
fn scaled_multiplies_lengths_and_positions_not_the_angle() {
    let line = RulerLine { start: Point::new(1.0, 2.0), end: Point::new(4.0, 6.0), protractor: Some(Point::new(1.0, 12.0)) };
    let (r, s) = (line.reading(), line.reading().scaled(2.0));
    assert_eq!((s.x, s.y, s.w, s.h, s.l1), (2.0, 4.0, 6.0, 8.0, 10.0));
    assert_eq!(s.l2, r.l2.map(|l| l * 2.0));
    assert_eq!(s.angle, r.angle);
}

// `part_at` within the reach prefers the protractor's end, then the end, then the start; outside the reach it is `None`.
#[test]
fn a_press_finds_the_protractor_then_the_end_then_the_start() {
    let line = RulerLine { start: Point::new(0.0, 0.0), end: Point::new(1.0, 0.0), protractor: Some(Point::new(0.5, 0.0)) };
    assert_eq!(line.part_at(Point::new(0.5, 0.0), 2.0), Some(RulerPart::Protractor));
    assert_eq!(line.part_at(Point::new(0.5, 0.0), 0.2), Some(RulerPart::Protractor));
    let line = RulerLine { protractor: None, ..line };
    assert_eq!(line.part_at(Point::new(0.5, 0.0), 2.0), Some(RulerPart::End));
    assert_eq!(line.part_at(Point::new(-1.5, 0.0), 2.0), Some(RulerPart::Start));
    assert_eq!(line.part_at(Point::new(50.0, 50.0), 2.0), None);
}
