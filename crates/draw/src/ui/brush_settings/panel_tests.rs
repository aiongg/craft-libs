//! Tests of the Brush Settings panel frame (task BR-BS6).
#![cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// The body of a section whose box is clear runs in a disabled `Ui`; an on section's runs enabled.
#[test]
#[ignore = "skeleton"]
fn an_off_section_greys_its_pane() {
    todo!()
}

// `body` gets the selected pane's key, or the first pane section's when `selected` is None or a
// switch; its result comes back in `body`.
#[test]
#[ignore = "skeleton"]
fn the_body_draws_the_selected_pane() {
    todo!()
}

// New and Reset report their actions; Update is disabled without `can_update` and reports
// `UpdatePreset` with it.
#[test]
#[ignore = "skeleton"]
fn the_strip_buttons_report_their_actions() {
    todo!()
}

// The list's events come through the panel's output in order.
#[test]
#[ignore = "skeleton"]
fn section_events_pass_through_in_order() {
    todo!()
}
