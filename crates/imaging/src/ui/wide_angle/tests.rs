//! Tests of the Adaptive Wide Angle workspace (task WS-WA1).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A drag over 6 points adds a constraint, shorter adds none; Shift at the release makes it
// Horizontal or Vertical by the larger delta.
#[test]
#[ignore = "skeleton"]
fn wide_angle_drag_adds_a_constraint_over_six_points_and_shift_aligns() {
    todo!()
}

// A right-click within 10 points of a curve opens its menu; Horizontal changes it, Delete removes
// it; a right-click elsewhere opens nothing.
#[test]
#[ignore = "skeleton"]
fn wide_angle_right_click_menu_changes_orientation_or_deletes() {
    todo!()
}

// Preview on shows the corrected source with the curves mapped; off, or while dragging, the
// source with the curves as arcs.
#[test]
#[ignore = "skeleton"]
fn wide_angle_preview_toggle_shows_source_or_corrected() {
    todo!()
}

// ⌘Z steps the session's history, not the document's; Enter is OK.
#[test]
#[ignore = "skeleton"]
fn wide_angle_command_z_steps_the_workspace_and_enter_is_ok() {
    todo!()
}

// Constraints are keyboard parts: Page picks, arrows move by a pixel (Shift ten), Delete removes.
#[test]
#[ignore = "skeleton"]
fn wide_angle_constraints_are_keyboard_parts() {
    todo!()
}
