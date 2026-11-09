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
fn shift_pressed_after_the_press_squares_at_once() {
    let origin = Point::new(0.0, 0.0);
    let mut gesture = MarqueeGesture::press(pos2(0.0, 0.0), origin, Mods::NONE);
    gesture.follow(Some(pos2(100.0, 50.0)), Mods::SHIFT);
    assert_eq!(corners(&mut gesture, Point::new(100.0, 50.0)), (origin, Point::new(100.0, 100.0)));
}

// Saved tool settings from the legacy crate keep loading: camelCase variant names.
#[test]
fn constraint_loads_the_legacy_settings_json() {
    let legacy = r#"[ "free", {"ratio": [1, 1]}, {"size": [64, 48]} ]"#;
    let loaded: Vec<Constraint> = serde_json::from_str(legacy).unwrap();
    assert_eq!(loaded, [Constraint::Free, Constraint::Ratio(1.0, 1.0), Constraint::Size(64.0, 48.0)]);
    assert_eq!(serde_json::to_string(&loaded).unwrap(), r#"["free",{"ratio":[1.0,1.0]},{"size":[64.0,48.0]}]"#);
}
