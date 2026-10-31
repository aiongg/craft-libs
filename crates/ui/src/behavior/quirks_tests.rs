//! The input quirks.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// On macOS a Control press becomes secondary and its release too, even when Control is released
// first; other platforms and Control+Command stay primary.
#[test]
#[ignore = "skeleton"]
fn control_click_is_secondary_on_macos_only() {
    todo!()
}

// A second primary press within half a second while held is dropped, as is a release while up.
#[test]
#[ignore = "skeleton"]
fn touch_twins_are_dropped() {
    todo!()
}

// Dragging up and right in a single-line field selects right of the anchor; a multi-line field
// keeps its vertical drag.
#[test]
#[ignore = "skeleton"]
fn single_line_drag_ignores_vertical_drift() {
    todo!()
}
