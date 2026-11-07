//! The marquee gesture: square and centre count only after a release and a new press.

use craft_behavior::geometry::{Constraint, MarqueeGesture};
use craft_keymap::Mods;
use emath::pos2;
use kurbo::Point;

fn corners(gesture: &mut MarqueeGesture, to: Point) -> (Point, Point) {
    let rect = gesture.size_to(to, false, Constraint::Free);
    (rect.a, rect.b)
}

#[test]
fn shift_held_at_the_press_squares_only_after_a_release_and_press() {
    let origin = Point::new(0.0, 0.0);
    let mut gesture = MarqueeGesture::press(pos2(0.0, 0.0), origin, Mods::SHIFT);
    gesture.follow(Some(pos2(100.0, 50.0)), Mods::SHIFT);
    assert_eq!(corners(&mut gesture, Point::new(100.0, 50.0)), (origin, Point::new(100.0, 50.0)));
    gesture.follow(None, Mods::NONE);
    gesture.follow(None, Mods::SHIFT);
    assert_eq!(corners(&mut gesture, Point::new(100.0, 50.0)), (origin, Point::new(100.0, 100.0)));
}

#[test]
fn alt_held_at_the_press_centres_only_after_a_release_and_press() {
    let origin = Point::new(0.0, 0.0);
    let mut gesture = MarqueeGesture::press(pos2(0.0, 0.0), origin, Mods::ALT);
    gesture.follow(Some(pos2(100.0, 50.0)), Mods::ALT);
    assert_eq!(corners(&mut gesture, Point::new(100.0, 50.0)), (origin, Point::new(100.0, 50.0)));
    gesture.follow(None, Mods::NONE);
    gesture.follow(None, Mods::ALT);
    assert_eq!(corners(&mut gesture, Point::new(100.0, 50.0)), (Point::new(-100.0, -50.0), Point::new(100.0, 50.0)));
}

#[test]
fn shift_pressed_after_the_press_squares_at_once() {
    let origin = Point::new(0.0, 0.0);
    let mut gesture = MarqueeGesture::press(pos2(0.0, 0.0), origin, Mods::NONE);
    gesture.follow(Some(pos2(100.0, 50.0)), Mods::SHIFT);
    assert_eq!(corners(&mut gesture, Point::new(100.0, 50.0)), (origin, Point::new(100.0, 100.0)));
}

#[test]
fn the_gesture_begins_past_the_slop() {
    let mut gesture = MarqueeGesture::press(pos2(0.0, 0.0), Point::ZERO, Mods::NONE);
    gesture.follow(Some(pos2(2.0, 0.0)), Mods::NONE);
    assert!(!gesture.begin_past(4.0, Constraint::Free));
    gesture.follow(Some(pos2(9.0, 0.0)), Mods::NONE);
    assert!(gesture.begin_past(4.0, Constraint::Free));
    assert!(!gesture.begin_past(4.0, Constraint::Free));
}
