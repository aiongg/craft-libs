//! Tests of the pin session (task TR-T6), on a fake solver that records its calls.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// `rest_of` finds the rest position under a deformed point from the top of the draw order and
// None off the mesh.
#[test]
#[ignore = "skeleton"]
fn a_click_on_the_mesh_places_a_pin_at_the_rest_position_under_it() {
    todo!()
}

// A Begin whose value adds a pin, Updates, Commit: one undo step, one rebind then solves.
#[test]
#[ignore = "skeleton"]
fn placing_a_pin_and_dragging_it_on_is_one_step() {
    todo!()
}

// A drag of an existing pin is one step; Cancel puts the pins back and records nothing.
#[test]
#[ignore = "skeleton"]
fn dragging_a_pin_is_one_step_and_esc_puts_it_back() {
    todo!()
}

// A Set that drops a pin is one step and rebinds; the selection loses the deleted index.
#[test]
#[ignore = "skeleton"]
fn alt_click_deletes_a_pin_as_one_step() {
    todo!()
}

// A density the solver refuses restores the previous density and expansion and returns its text.
#[test]
#[ignore = "skeleton"]
fn a_refused_remesh_restores_density_and_expansion_and_reports_the_text() {
    todo!()
}

// The same pins reached by two pointer paths call `solve` with the same pins (no warm start).
#[test]
#[ignore = "skeleton"]
fn solving_starts_from_the_rest_mesh_whatever_the_pointer_history() {
    todo!()
}

// Undo and redo restore the pins, rebind, and keep selected indices that still exist.
#[test]
#[ignore = "skeleton"]
fn undo_and_redo_restore_the_pins_and_keep_the_selection_when_it_still_exists() {
    todo!()
}

// Untouched pins (and a turn of 360°) give no result; a moved pin gives the pins.
#[test]
#[ignore = "skeleton"]
fn an_untouched_session_has_no_result() {
    todo!()
}

// Arrow nudges within the pause are one step.
#[test]
#[ignore = "skeleton"]
fn pin_nudges_within_the_pause_fold_into_one_step() {
    todo!()
}
