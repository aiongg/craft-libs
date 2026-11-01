//! Tests of the perspective session and plane geometry (task TR-T7).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Corners within 0.5 in the source are one linked group; others are alone.
#[test]
#[ignore = "skeleton"]
fn corners_within_the_link_distance_are_linked() {
    todo!()
}

// A Warp corner drag moves the linked targets together and keeps the sources.
#[test]
#[ignore = "skeleton"]
fn linked_corners_move_together_in_warp_and_layout_is_kept() {
    todo!()
}

// Mode(Warp) unifies linked corners to their means.
#[test]
#[ignore = "skeleton"]
fn entering_warp_unifies_linked_corners() {
    todo!()
}

// Straighten levels near-vertical edges, Level near-horizontal ones, Both both, moving linked
// corners together.
#[test]
#[ignore = "skeleton"]
fn straighten_level_and_both_move_the_corners() {
    todo!()
}

// Mode(Warp) with no planes changes nothing.
#[test]
#[ignore = "skeleton"]
fn warp_needs_at_least_one_plane() {
    todo!()
}

// `replace` bumps the generation and ends a running drag; untouched planes give no result.
#[test]
#[ignore = "skeleton"]
fn replacing_the_planes_bumps_the_generation_and_identity_gives_no_result() {
    todo!()
}

// A drag is one undo step; undo restores planes and mode together.
#[test]
#[ignore = "skeleton"]
fn plane_edits_are_undo_steps() {
    todo!()
}
