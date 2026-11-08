//! The move gesture, the stroke lock and ray snapping.

use craft_behavior::geometry::{ConstrainStyle, Directions, MoveGesture, StrokeLock, lock_direction, snap_ray};
use craft_keymap::Mods;
use kurbo::{Point, Vec2};

fn style() -> ConstrainStyle {
    ConstrainStyle::new(2.0, 3.0)
}

fn polar(deg: f64, len: f64) -> Vec2 {
    let a = deg.to_radians();
    Vec2::new(len * a.cos(), len * a.sin())
}

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
}

#[test]
fn alt_copies_once_after_the_reach_and_stays_on() {
    let mut g = MoveGesture::press(&style(), Point::ZERO, Mods::ALT, Directions::Four);
    // 1 document unit at zoom 1.5 is 1.5 screen points.
    assert!(!g.follow(Point::new(1.0, 0.0), Mods::ALT, 1.5).unwrap().copy);
    assert!(g.follow(Point::new(2.0, 0.0), Mods::ALT, 1.5).unwrap().copy);
    assert!(g.follow(Point::new(0.5, 0.0), Mods::ALT, 1.5).unwrap().copy);
}

#[test]
fn a_click_without_travel_is_unchanged() {
    let g = MoveGesture::press(&style(), Point::new(5.0, 5.0), Mods::ALT, Directions::Four);
    let value = g.release(Point::new(5.0, 5.0), Mods::ALT, 1.0);
    assert!(!value.changed());
    assert!(!value.copy);
}

#[test]
fn alt_after_the_press_does_not_copy() {
    let mut g = MoveGesture::press(&style(), Point::ZERO, Mods::NONE, Directions::Four);
    assert!(!g.follow(Point::new(50.0, 0.0), Mods::ALT, 1.0).unwrap().copy);
}

#[test]
fn release_applies_the_lock_of_the_last_move() {
    let mut g = MoveGesture::press(&style(), Point::ZERO, Mods::NONE, Directions::Eight);
    let last = g.follow(Point::new(10.0, 3.0), Mods::SHIFT, 1.0).unwrap();
    assert_eq!(last.delta.y, 0.0);
    assert_eq!(g.value(), last);
    let end = g.release(Point::new(10.0, 3.0), Mods::SHIFT, 1.0);
    assert_eq!(end, last);
    assert!(end.changed());
}

#[test]
fn stroke_lock_waits_then_locks_to_45_degrees() {
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

#[test]
fn snap_ray_projects_onto_the_nearest_ray() {
    let step = std::f64::consts::FRAC_PI_4;
    let o = Point::new(1.0, 1.0);
    let p = snap_ray(o, Point::new(11.0, 3.0), step);
    assert!((p.y - 1.0).abs() < 1e-9 && (p.x - 11.0).abs() < 1e-9);
    let p = snap_ray(o, Point::new(6.0, 5.0), step);
    assert!((p.x - p.y).abs() < 1e-9);
    assert_eq!(snap_ray(o, o, step), o);
    assert_eq!(snap_ray(o, Point::new(5.0, 5.0), 0.0), Point::new(5.0, 5.0));
}
