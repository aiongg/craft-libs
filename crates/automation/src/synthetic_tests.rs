//! Behaviour tests of `synthetic`.

use egui::{Event, Key, PointerButton, RawInput};
use serde_json::json;

use super::*;

fn press_release(raw: &[Event]) -> (usize, usize) {
    let presses = raw.iter().filter(|e| matches!(e, Event::PointerButton { pressed: true, .. })).count();
    let releases = raw.iter().filter(|e| matches!(e, Event::PointerButton { pressed: false, .. })).count();
    (presses, releases)
}

// A two-click ui.click injects the move, then each press and each release on its own pass; nothing more is injected until pass_finished; MAX_CLICKS bounds count.
#[test]
fn click_steps_land_on_separate_passes() {
    let click = Click::from_params(&json!({"x": 3, "y": 4, "button": "secondary", "count": 2})).unwrap();
    assert_eq!(click, Click { at: egui::pos2(3.0, 4.0), button: PointerButton::Secondary, count: 2 });
    assert_eq!(Click::from_params(&json!({})).unwrap().count, 1);
    assert_eq!(Click::from_params(&json!({"count": 257})).unwrap_err(), "`count` must be at most 256 (got 257)");
    assert!(Click::from_params(&json!({"z": 1})).unwrap_err().starts_with("unknown field `z`"));

    let mut input = SyntheticInput::default();
    input.click(&click);
    let mut first = RawInput::default();
    assert_eq!(input.inject(&mut first, true), Injected::Step);
    assert!(matches!(first.events[..], [Event::PointerMoved(_)]));

    let mut stalled = RawInput::default();
    assert_eq!(input.inject(&mut stalled, true), Injected::Nothing);
    assert!(stalled.events.is_empty() && !input.is_empty());

    let mut seen = Vec::new();
    for _ in 0..4 {
        input.pass_finished();
        let mut raw = RawInput::default();
        assert_eq!(input.inject(&mut raw, true), Injected::Step);
        assert_eq!(raw.events.len(), 1);
        seen.push(press_release(&raw.events));
    }
    assert_eq!(seen, [(1, 0), (0, 1), (1, 0), (0, 1)]);
    assert!(!input.is_empty());
    input.pass_finished();
    assert!(input.is_empty());
    assert_eq!(input.inject(&mut RawInput::default(), true), Injected::Nothing);

    let key = KeyPress::from_params(&json!({"key": "Enter", "shift": true})).unwrap();
    assert_eq!((key.key, key.modifiers.shift), (Key::Enter, true));
    assert_eq!(KeyPress::from_params(&json!({})).unwrap_err(), "missing `key`");
    assert_eq!(KeyPress::from_params(&json!({"key": "Nope"})).unwrap_err(), "unknown key `Nope`");
}

// A deferred pass gets its step appended again once; a changed target withdraws the step from the input and clears the queue.
#[test]
fn in_flight_steps_are_reinjected_or_withdrawn() {
    let mut input = SyntheticInput::default();
    input.key(&KeyPress { key: Key::A, modifiers: Default::default() });
    input.type_text("x");
    let mut raw = RawInput::default();
    assert_eq!(input.inject(&mut raw, true), Injected::Step);
    // A key press is physical, so the shortcut dispatcher's position fallback can read it.
    assert!(matches!(raw.events[..], [Event::Key { key: Key::A, physical_key: Some(Key::A), pressed: true, .. }]));

    input.pass_deferred();
    let mut again = RawInput::default();
    assert_eq!(input.inject(&mut again, true), Injected::Again);
    assert_eq!(again.events, raw.events);
    assert_eq!(input.inject(&mut RawInput::default(), true), Injected::Nothing);

    // The hook sees eframe's preserved events around the step; only the step's run is removed.
    let mut preserved = RawInput { events: vec![Event::Text("keep".into())], ..Default::default() };
    preserved.events.extend(raw.events.clone());
    assert_eq!(input.inject(&mut preserved, false), Injected::Withdrawn);
    assert_eq!(preserved.events, vec![Event::Text("keep".into())]);
    assert!(input.is_empty());
    assert!(!input.cancel());
}
