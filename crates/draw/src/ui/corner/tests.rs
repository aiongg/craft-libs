//! Tests of the corner widgets (task PE-B4).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Dragging a rectangle's widget reports Begin, Update and Commit with every selected target at the
// same radius; with ⌥ only the dragged corner of the rectangle changes and a star's all.
#[test]
#[ignore = "skeleton"]
fn corner_drag_reports_every_target_and_alt_one_corner() {
    todo!()
}

// A press and release without movement reports no event and is claimed.
#[test]
#[ignore = "skeleton"]
fn corner_click_without_movement_changes_nothing() {
    todo!()
}

// With `kinds(true)` an ⌥-click reports `cycle_kind` with the next kind; a double-click reports
// `activate`.
#[test]
#[ignore = "skeleton"]
fn corner_alt_click_cycles_the_kind_and_double_click_activates() {
    todo!()
}

// With focus: Page Down picks the next widget, the arrows report Sets of ±1 (⇧ 10, ⌥ one corner),
// Enter reports `activate`.
#[test]
#[ignore = "skeleton"]
fn corner_keyboard_steps_the_radius() {
    todo!()
}

// Esc during a drag reports Cancel and the rest of the press changes nothing.
#[test]
#[ignore = "skeleton"]
fn corner_esc_cancels_the_drag() {
    todo!()
}

// The group is named "Corner radius"; each widget is an object "Corner i of n" valued "radius 12
// px"; the readout shows "R: 25.0 px" while dragging.
#[test]
#[ignore = "skeleton"]
fn corner_accesskit_group_and_values() {
    todo!()
}
