//! Tests of the point picker row and its canvas tool (task AD-1).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A click arms Black (`Some(Some(Black))`), a second click disarms (`Some(None)`), Esc on the focused armed button disarms and keeps the press.
#[test]
#[ignore = "skeleton"]
fn tone_picker_row_reports_arm_and_disarm() {
    todo!()
}

// A picker disabled by the host (Gray in a grey document) takes no click; `unavailable` disables all three and shows the note.
#[test]
#[ignore = "skeleton"]
fn tone_picker_row_disables_a_picker_the_host_disables() {
    todo!()
}

// A press on the tool host's canvas samples at the document point and `take` yields a `TonePick` for the armed picker once; off the image nothing.
#[test]
#[ignore = "skeleton"]
fn tone_pick_tool_reports_the_sample_for_the_armed_picker() {
    todo!()
}
