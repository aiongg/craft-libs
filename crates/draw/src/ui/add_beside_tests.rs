//! Tests of the "+" buttons (task SH-T8), and of the Artboard tool's "+" press, which needs them.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// An object 15 points wide on screen shows no buttons; a button under a `clear_of` rect is hidden
// and the others show; `hidden(true)` hides all.
#[test]
#[ignore = "skeleton"]
fn add_beside_hides_when_small_or_under_clear_rectangles() {
    todo!()
}

// Tab focuses the group, → picks Right, Enter reports `clicked: Some(Right)`; a click with ⌥
// reports `copy`.
#[test]
#[ignore = "skeleton"]
fn add_beside_keys_and_alt_click() {
    todo!()
}

// The Artboard tool with Photoshop's rules: a press on the right "+" of the selected board
// reports `AddBeside { side: Right, copy: false }`, with ⌥ `copy: true` (PhotoCraft
// `plus_buttons_add_boards_and_shift_keeps_proportions`).
#[test]
#[ignore = "skeleton"]
fn artboard_plus_button_reports_add_beside_and_alt_copies() {
    todo!()
}
