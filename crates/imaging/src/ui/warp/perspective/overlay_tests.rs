//! Tests of the perspective overlay (task TR-T13).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A Layout drag of 10 × 10 adds a plane; a drag under 4 units each way adds none.
#[test]
#[ignore = "skeleton"]
fn a_drag_in_layout_draws_a_plane_and_a_tiny_drag_adds_none() {
    todo!()
}

// A corner dropped within `transform.hit` of another plane's corner takes its position, which
// links them.
#[test]
#[ignore = "skeleton"]
fn a_corner_dropped_near_another_planes_corner_snaps_and_links() {
    todo!()
}

// When the generation changes in the middle of a corner drag, the drag ends with Cancel and the
// next move reports nothing.
#[test]
#[ignore = "skeleton"]
fn replacing_the_planes_in_the_middle_of_a_corner_drag_ends_the_drag() {
    todo!()
}

// Insert adds the middle-half plane, Page keys pick corners, arrows move them, Delete removes the
// picked corner's plane.
#[test]
#[ignore = "skeleton"]
fn the_keyboard_adds_a_plane_picks_corners_and_removes_a_plane() {
    todo!()
}

// AccessKit: each corner named "Plane 2, corner 3" with value "x, y", "linked" when linked.
#[test]
#[ignore = "skeleton"]
fn every_corner_is_named_and_valued_for_assistive_technology() {
    todo!()
}
