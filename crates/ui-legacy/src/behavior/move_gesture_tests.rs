//! Tests of the move gesture.

use egui::Modifiers;
use kurbo::{Point, Vec2};

use super::move_gesture::{ConstrainStyle, Directions, MoveGesture, StrokeLock, lock_direction, snap_ray};
use crate::edit::{CancelReason, EditEvent};

const SHIFT: Modifiers = Modifiers { alt: false, shift: true, ctrl: false, mac_cmd: false, command: false };
const ALT: Modifiers = Modifiers { alt: true, shift: false, ctrl: false, mac_cmd: false, command: false };

fn style() -> ConstrainStyle {
    ConstrainStyle { copy_reach: 2.0, stroke_lock: 3.0 }
}

fn polar(deg: f64, len: f64) -> Vec2 {
    let a = deg.to_radians();
    Vec2::new(len * a.cos(), len * a.sin())
}

fn value<T: Copy>(e: Option<EditEvent<T>>) -> T {
    *e.expect("an event").value().expect("a value")
}

// The locked direction holds until the pointer is half a step plus 8 degrees past it (PhotoCraft's
// `move_mods/tests.rs` `shift_axis_switches_once_the_pointer_is_clearly_nearer_the_other_one`).
#[test]
fn lock_direction_keeps_the_axis_until_eight_degrees_past_half() {
    let axis = lock_direction(Vec2::new(10.0, 0.0), None, Directions::Eight);
    assert_eq!(axis, Vec2::new(10.0, 0.0));
    // 22.5 + 8 = 30.5 degrees is the switch.
    let kept = lock_direction(polar(30.0, 10.0), Some(axis), Directions::Eight);
    assert_eq!(kept.y, 0.0);
    assert!(kept.x > 0.0);
    let switched = lock_direction(polar(31.0, 10.0), Some(axis), Directions::Eight);
    assert!((switched.y - switched.x).abs() < 1e-9 && switched.y > 0.0);
}

// `Four` locks to the axes only; `Eight` also to the diagonals; lengths are projections, never negative.
#[test]
fn four_and_eight_directions() {
    let d = polar(40.0, 10.0);
    let four = lock_direction(d, None, Directions::Four);
    assert_eq!(four.y, 0.0);
    assert!((four.x - d.x).abs() < 1e-9);
    let eight = lock_direction(d, None, Directions::Eight);
    assert!((eight.x - eight.y).abs() < 1e-9);
    assert!((eight.hypot() - 10.0 * 5.0_f64.to_radians().cos()).abs() < 1e-9);
    assert_eq!(lock_direction(Vec2::ZERO, None, Directions::Four), Vec2::ZERO);
    let nan = Vec2::new(f64::NAN, 1.0);
    assert!(lock_direction(nan, None, Directions::Four).x.is_nan());
}

// With alt at the press `copy` is false before 2 screen points of travel, true after, and stays true.
#[test]
fn alt_copies_once_after_two_points_of_travel() {
    let (mut g, begin) = MoveGesture::begin(&style(), Point::ZERO, ALT, Directions::Four);
    assert!(matches!(begin, EditEvent::Begin { value: None, .. }));
    // 1 document unit at zoom 1.5 is 1.5 screen points.
    assert!(!value(g.drag(Point::new(1.0, 0.0), ALT, 1.5)).copy);
    assert!(value(g.drag(Point::new(2.0, 0.0), ALT, 1.5)).copy);
    assert!(value(g.drag(Point::new(0.5, 0.0), ALT, 1.5)).copy);
}

// A press and release without travel commits with `changed` false; alt without travel does not copy.
#[test]
fn a_click_without_travel_commits_unchanged() {
    let (g, _) = MoveGesture::begin(&style(), Point::new(5.0, 5.0), ALT, Directions::Four);
    match g.end(Point::new(5.0, 5.0), ALT, 1.0) {
        EditEvent::Commit { value, changed, .. } => {
            assert!(!changed);
            assert!(!value.copy);
            assert_eq!(value.delta, Vec2::ZERO);
        }
        other => panic!("{other:?}"),
    }
}

// Alt pressed after the press never turns `copy` on.
#[test]
fn alt_after_the_press_does_not_copy() {
    let (mut g, _) = MoveGesture::begin(&style(), Point::ZERO, Modifiers::NONE, Directions::Four);
    assert!(!value(g.drag(Point::new(50.0, 0.0), ALT, 1.0)).copy);
}

// The release's value equals the last update's under shift.
#[test]
fn end_applies_the_lock_of_the_last_move() {
    let (mut g, _) = MoveGesture::begin(&style(), Point::ZERO, Modifiers::NONE, Directions::Eight);
    let last = value(g.drag(Point::new(10.0, 3.0), SHIFT, 1.0));
    assert_eq!(last.delta.y, 0.0);
    let EditEvent::Commit { value: end, changed, .. } = g.end(Point::new(10.0, 3.0), SHIFT, 1.0) else { panic!() };
    assert_eq!(end, last);
    assert!(changed);
}

// `cancel` reports `Cancel` with the Escape reason, so the host restores its mark.
#[test]
fn esc_cancels_and_reports_cancel() {
    let (g, _) = MoveGesture::begin(&style(), Point::ZERO, Modifiers::NONE, Directions::Four);
    assert!(matches!(g.cancel(), EditEvent::Cancel { reason: CancelReason::Escape, .. }));
}

// Inside 3 screen points the origin comes back; then the nearest 45 degree ray holds until shift is up.
#[test]
fn stroke_lock_waits_three_points_then_locks_to_45_degrees() {
    let mut lock = StrokeLock::new(&style());
    let last = Point::new(10.0, 10.0);
    // 3 screen points at zoom 2 is 1.5 document units.
    assert_eq!(lock.constrain(last, Point::new(11.0, 10.5), true, 2.0), last);
    assert!(!lock.locked());
    let p = lock.constrain(last, Point::new(20.0, 11.0), true, 2.0);
    assert!(lock.locked());
    assert!((p.y - 10.0).abs() < 1e-9 && p.x > 19.0);
    // The direction holds although the pointer is nearer the diagonal now.
    let p = lock.constrain(Point::new(19.0, 10.0), Point::new(20.0, 18.0), true, 2.0);
    assert!((p.y - 10.0).abs() < 1e-9);
    let free = Point::new(3.0, 4.0);
    assert_eq!(lock.constrain(last, free, false, 2.0), free);
    assert!(!lock.locked());
}

// `snap_ray` projects onto the nearest pi/4 ray and leaves a point on the origin or non-finite alone.
#[test]
fn snap_ray_projects_onto_the_nearest_ray() {
    let step = std::f64::consts::FRAC_PI_4;
    let o = Point::new(1.0, 1.0);
    let p = snap_ray(o, Point::new(11.0, 3.0), step);
    assert!((p.y - 1.0).abs() < 1e-9 && (p.x - 11.0).abs() < 1e-9);
    let p = snap_ray(o, Point::new(6.0, 5.0), step);
    assert!((p.x - p.y).abs() < 1e-9);
    assert_eq!(snap_ray(o, o, step), o);
    let bad = Point::new(f64::NAN, 2.0);
    assert!(snap_ray(o, bad, step).x.is_nan());
    assert_eq!(snap_ray(o, Point::new(5.0, 5.0), 0.0), Point::new(5.0, 5.0));
}
