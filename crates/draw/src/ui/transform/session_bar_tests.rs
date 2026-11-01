//! Tests of the session's options bar (task TR-T9).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Box phase: fields, interpolation, Show Reference Point, cancel, commit, switch in Tab order;
// Content-Aware subset: position and size only, no switch; Warp phase: the warp group, cancel,
// commit, Reset warp, the switch pressed.
#[test]
#[ignore = "skeleton"]
fn the_bar_shows_fields_interpolation_check_and_buttons_in_phase_order() {
    todo!()
}

// The buttons' accessible names are "Commit transform" and "Cancel transform" in the Box phase and
// "Commit warp" and "Cancel warp" in the Warp phase; tooltips add Enter and Esc.
#[test]
#[ignore = "skeleton"]
fn commit_and_cancel_buttons_carry_the_phase_names_and_keys() {
    todo!()
}

// The check reports ReferencePoint and writes `show_reference_point` into the settings; the
// interpolation select reports Interpolation and writes the setting.
#[test]
#[ignore = "skeleton"]
fn the_check_and_the_interpolation_report_edits_and_write_the_settings() {
    todo!()
}

// The switch reports WarpRequested in the Box phase and LeaveWarp in the Warp phase; Reset warp
// sets `reset_warp`.
#[test]
#[ignore = "skeleton"]
fn the_switch_asks_for_a_warp_and_reset_asks_for_an_identity_warp() {
    todo!()
}
