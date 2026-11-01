//! Tests of the dynamic control row (task BR-BS5).
#![cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// The Fade steps field shows only while the current entry is `fade`; a typed 30 reports Set(30).
#[test]
#[ignore = "skeleton"]
fn dynamic_row_shows_fade_steps_only_for_fade() {
    todo!()
}

// The curve toggle shows only for `curve` entries and is `selected` while the curve is open or
// shaped; a click reports `curve_toggled`.
#[test]
#[ignore = "skeleton"]
fn curve_toggle_lights_when_the_curve_is_not_linear() {
    todo!()
}

// Picking another entry in the select reports `control`.
#[test]
#[ignore = "skeleton"]
fn picking_a_control_reports_it() {
    todo!()
}
