//! Tests of the Free Transform session (task TR-T2).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A new session has no result and commits as CloseUntouched; a moved box gives Apply with the
// frame, the box and the interpolation; a mode or interpolation change alone stays untouched.
#[test]
#[ignore = "skeleton"]
fn an_untouched_session_has_no_result_and_a_moved_one_has() {
    todo!()
}

// Box Begin, Update, then Cancel restores the box taken at Begin and requests nothing; a Cancel
// edit with no drag running requests Discard.
#[test]
#[ignore = "skeleton"]
fn esc_during_a_drag_restores_the_box_and_a_second_esc_discards() {
    todo!()
}

// A Fields typing run (Begin, Updates, Commit) sets the box and the V skew from each report, which
// the fields built from the box at the run's start (gaps 112 to 114), and is one undo step.
#[test]
#[ignore = "skeleton"]
fn a_field_typing_run_builds_every_report_from_the_box_at_its_start() {
    todo!()
}

// Rotate 90° Clockwise and Flip Horizontal turn and mirror the box about the reference point and
// request nothing; in the Warp phase a preset changes nothing.
#[test]
#[ignore = "skeleton"]
fn presets_turn_the_box_about_the_reference_point_without_applying() {
    todo!()
}

// WarpRequested answers NeedWarp with the current frame and box; Warp(w) enters the Warp phase;
// one undo goes back to the Box phase with the box as it was.
#[test]
#[ignore = "skeleton"]
fn entering_warp_asks_for_a_warp_and_carries_the_box() {
    todo!()
}

// LeaveWarp drops the warp and returns to the Box phase with the box unchanged.
#[test]
#[ignore = "skeleton"]
fn leaving_warp_drops_the_warp_and_keeps_the_box() {
    todo!()
}

// A move, a pivot move, a warp: undo restores the box, the pivot and the warp of each step
// together; redo goes forward; a new edit after undo drops the redo.
#[test]
#[ignore = "skeleton"]
fn undo_and_redo_restore_the_box_pivot_and_warp_together() {
    todo!()
}

// The menu lists Free Transform, Scale, Rotate, Skew, Distort, Perspective, Warp, separator, the
// three rotations, separator, the two flips; the current mode is checked; Distort and Perspective
// are disabled when the content refuses distort; every pick applies as its edit.
#[test]
#[ignore = "skeleton"]
fn the_mode_menu_lists_photoshops_order_and_the_picks_apply() {
    todo!()
}

// ReferencePoint(false) keeps the pivot where it is: a later preset and the box rules still turn
// and scale about it.
#[test]
#[ignore = "skeleton"]
fn a_hidden_reference_point_stays_where_it_is_and_still_pivots_scaling() {
    todo!()
}

// `box_rules` with `legacy` gives ScaleRule::Free, without it Proportional; the aspect link and
// the session's mode and allow carry over; the rotate step is 15.
#[test]
#[ignore = "skeleton"]
fn settings_choose_the_legacy_scale_rule_and_the_aspect_link() {
    todo!()
}

// Two nudges within the pause share one step; a drag between them breaks the run; with no pause
// every nudge is its own step.
#[test]
#[ignore = "skeleton"]
fn nudges_within_the_pause_fold_into_one_step() {
    todo!()
}

// Between a Box Begin and its Commit, undo and redo return false and change nothing.
#[test]
#[ignore = "skeleton"]
fn undo_is_refused_while_a_drag_runs() {
    todo!()
}

// `homography` maps the frame's corners onto the box's corners and its centre into the box; a box
// with three corners on a line gives None.
#[test]
#[ignore = "skeleton"]
fn the_homography_maps_the_frame_onto_the_box() {
    todo!()
}
