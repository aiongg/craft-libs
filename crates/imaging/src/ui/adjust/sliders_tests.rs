//! Tests of the slider tables (task AD-3).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A double-click on Exposure's slider reports one `Set` with every value and Exposure at its default 0.
#[test]
#[ignore = "skeleton"]
fn adjust_sliders_double_click_returns_to_default() {
    todo!()
}

// The wheel over an unfocused slider steps it with `AdjustContext::dialog` and does nothing with `properties`.
#[test]
#[ignore = "skeleton"]
fn adjust_sliders_wheel_steps_only_in_a_dialog_context() {
    todo!()
}

// `AdjustField::round`: Exposure keeps two places, Offset four, a 0..10 range 0.01, wider ranges whole numbers (PhotoCraft's `row_value` test).
#[test]
#[ignore = "skeleton"]
fn adjust_sliders_round_to_the_fields_decimals() {
    todo!()
}

// A field with `AdjustTrack::Colors` draws its colour track with dynamic tracks on and the plain track with them off.
#[test]
#[ignore = "skeleton"]
fn dynamic_tracks_off_draws_plain_tracks() {
    todo!()
}
