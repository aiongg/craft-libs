//! Tests of the move gesture.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// The locked direction holds until the pointer is half a step plus 8° past it (PhotoCraft's
// `move_mods/tests.rs` `shift_axis_switches_once_the_pointer_is_clearly_nearer_the_other_one`).
#[test]
#[ignore = "skeleton"]
fn lock_direction_keeps_the_axis_until_eight_degrees_past_half() {
    todo!()
}

// `Four` locks to the axes only; `Eight` also to the diagonals; lengths are projections, never negative.
#[test]
#[ignore = "skeleton"]
fn four_and_eight_directions() {
    todo!()
}

// With ⌥ at the press `copy` is false before 2 screen points of travel, true after, and stays true.
#[test]
#[ignore = "skeleton"]
fn alt_copies_once_after_two_points_of_travel() {
    todo!()
}

// A press and release without travel commits with `changed` false; ⌥ without travel does not copy.
#[test]
#[ignore = "skeleton"]
fn a_click_without_travel_commits_unchanged() {
    todo!()
}

// ⌥ pressed after the press never turns `copy` on.
#[test]
#[ignore = "skeleton"]
fn alt_after_the_press_does_not_copy() {
    todo!()
}

// The release's value equals the last update's under ⇧.
#[test]
#[ignore = "skeleton"]
fn end_applies_the_lock_of_the_last_move() {
    todo!()
}

// `cancel` reports `Cancel` with the Escape reason, so the host restores its mark.
#[test]
#[ignore = "skeleton"]
fn esc_cancels_and_reports_cancel() {
    todo!()
}

// Inside 3 screen points the origin comes back; then the nearest 45° ray holds until ⇧ is up.
#[test]
#[ignore = "skeleton"]
fn stroke_lock_waits_three_points_then_locks_to_45_degrees() {
    todo!()
}

// `snap_ray` projects onto the nearest π/4 ray and leaves a point on the origin or non-finite alone.
#[test]
#[ignore = "skeleton"]
fn snap_ray_projects_onto_the_nearest_ray() {
    todo!()
}
