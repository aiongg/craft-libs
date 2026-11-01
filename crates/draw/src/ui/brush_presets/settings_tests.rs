//! Tests of the picker's persisted view (task BR-BP3).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// `restored` clamps the scale to 0.15..=2.0, turns a non-finite scale into 1, and turns every part
// on when none is; a JSON value missing fields loads the defaults (serde `default`).
#[test]
#[ignore = "skeleton"]
fn settings_restore_clamps_the_scale_and_re_enables_parts() {
    todo!()
}

// `BrushCardsState::restore` then `settings` gives the restored value back.
#[test]
#[ignore = "skeleton"]
fn cards_state_round_trips_its_settings() {
    todo!()
}
