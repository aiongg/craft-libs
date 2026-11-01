//! Tests of the picker's persisted view (task BR-BP3).

use super::{BrushCardParts, BrushCardsState, BrushPickerSettings};

fn settings(parts: BrushCardParts, scale: f32) -> BrushPickerSettings {
    BrushPickerSettings { parts, scale }
}

// `restored` clamps the scale to 0.15..=2.0, turns a non-finite scale into 1, and turns every part
// on when none is; a JSON value missing fields loads the defaults (serde `default`).
#[test]
fn settings_restore_clamps_the_scale_and_re_enables_parts() {
    let all = BrushCardParts::ALL;
    assert_eq!(settings(all, 9.0).restored().scale, 2.0);
    assert_eq!(settings(all, 0.0).restored().scale, 0.15);
    assert_eq!(settings(all, 0.5).restored().scale, 0.5);
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(settings(all, bad).restored().scale, 1.0, "{bad}");
    }
    let none = BrushCardParts { name: false, stroke: false, tip: false };
    assert_eq!(settings(none, 1.0).restored().parts, BrushCardParts::ALL);
    let tip = BrushCardParts { name: false, stroke: false, tip: true };
    assert_eq!(settings(tip, 1.0).restored().parts, tip);

    let Ok(empty) = serde_json::from_str::<BrushPickerSettings>("{}") else { panic!("an empty object loads") };
    assert_eq!(empty, BrushPickerSettings::default());
    let Ok(partial) = serde_json::from_str::<BrushPickerSettings>(r#"{"scale":0.5}"#) else { panic!("a partial object loads") };
    assert_eq!((partial.parts, partial.scale), (BrushCardParts::ALL, 0.5));
}

// `BrushCardsState::restore` then `settings` gives the restored value back.
#[test]
fn cards_state_round_trips_its_settings() {
    let mut state = BrushCardsState::<String>::new();
    let saved = settings(BrushCardParts { name: true, stroke: false, tip: true }, 0.8);
    state.restore(&saved);
    assert_eq!(state.settings(), saved);

    state.restore(&settings(BrushCardParts { name: false, stroke: false, tip: false }, 7.0));
    assert_eq!(state.settings(), settings(BrushCardParts::ALL, 2.0));
}
