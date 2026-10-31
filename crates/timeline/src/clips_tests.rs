//! Clip edges, trims and snapping.

#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// edge_zone is 7 pt, at most a third of the width, None for zero width.
#[test]
#[ignore = "skeleton"]
fn edge_zone_is_a_third_of_narrow_clips() {
    todo!()
}

// edge_at prefers the edge whose clip the pointer is inside.
#[test]
#[ignore = "skeleton"]
fn the_clip_under_the_pointer_wins_a_shared_cut() {
    todo!()
}

// Cmd+Shift roll, Cmd ripple, no-modifier roll on the cut and ripple elsewhere.
#[test]
#[ignore = "skeleton"]
fn selection_trim_follows_modifiers_and_the_setting() {
    todo!()
}

// snap_time snaps within 9 pt and reports the target.
#[test]
#[ignore = "skeleton"]
fn snap_time_takes_the_nearest_target_in_reach() {
    todo!()
}

// marquee_clips returns the clips in the rectangle.
#[test]
#[ignore = "skeleton"]
fn marquee_selects_the_clips_it_touches() {
    todo!()
}
