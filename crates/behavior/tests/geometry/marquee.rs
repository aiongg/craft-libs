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
fn modifiers_held_at_the_press_apply_only_after_a_release_and_press() {
    // shift held at the press squares only after a release and press
    {
        let origin = Point::new(0.0, 0.0);
        let mut gesture = MarqueeGesture::press(pos2(0.0, 0.0), origin, Mods::SHIFT);
        gesture.follow(Some(pos2(100.0, 50.0)), Mods::SHIFT);
        assert_eq!(corners(&mut gesture, Point::new(100.0, 50.0)), (origin, Point::new(100.0, 50.0)));
        gesture.follow(None, Mods::NONE);
        gesture.follow(None, Mods::SHIFT);
        assert_eq!(corners(&mut gesture, Point::new(100.0, 50.0)), (origin, Point::new(100.0, 100.0)));
    }
    // alt held at the press centres only after a release and press
    {
        let origin = Point::new(0.0, 0.0);
        let mut gesture = MarqueeGesture::press(pos2(0.0, 0.0), origin, Mods::ALT);
        gesture.follow(Some(pos2(100.0, 50.0)), Mods::ALT);
        assert_eq!(corners(&mut gesture, Point::new(100.0, 50.0)), (origin, Point::new(100.0, 50.0)));
        gesture.follow(None, Mods::NONE);
        gesture.follow(None, Mods::ALT);
        assert_eq!(corners(&mut gesture, Point::new(100.0, 50.0)), (Point::new(-100.0, -50.0), Point::new(100.0, 50.0)));
    }
}

#[test]
fn shift_pressed_after_the_press_squares_at_once() {
    let origin = Point::new(0.0, 0.0);
    let mut gesture = MarqueeGesture::press(pos2(0.0, 0.0), origin, Mods::NONE);
    gesture.follow(Some(pos2(100.0, 50.0)), Mods::SHIFT);
    assert_eq!(corners(&mut gesture, Point::new(100.0, 50.0)), (origin, Point::new(100.0, 100.0)));
}

#[test]
fn the_gesture_begins_once() {
    let mut gesture = MarqueeGesture::press(pos2(0.0, 0.0), Point::ZERO, Mods::NONE);
    assert!(!gesture.has_begun());
    assert!(gesture.begin());
    assert!(gesture.has_begun());
    assert!(!gesture.begin());
}

#[test]
fn constraint_settings_load_and_round_trip() {
    // Saved tool settings from the legacy crate keep loading: camelCase variant names.
    {
        let legacy = r#"[ "free", {"ratio": [1, 1]}, {"size": [64, 48]} ]"#;
        let loaded: Vec<Constraint> = serde_json::from_str(legacy).unwrap();
        assert_eq!(loaded, [Constraint::Free, Constraint::Ratio(1.0, 1.0), Constraint::Size(64.0, 48.0)]);
        assert_eq!(serde_json::to_string(&loaded).unwrap(), r#"["free",{"ratio":[1.0,1.0]},{"size":[64.0,48.0]}]"#);
    }
    // every marquee style survives a save and load
    {
        for style in [Constraint::Free, Constraint::Ratio(3.0, 2.0), Constraint::Size(64.0, 48.0)] {
            let saved = serde_json::to_string(&style).unwrap();
            assert_eq!(serde_json::from_str::<Constraint>(&saved).unwrap(), style, "{saved}");
        }
    }
}
