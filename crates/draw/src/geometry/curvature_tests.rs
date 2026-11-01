//! Tests of the Curvature Pen's handles (task PE-A2).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// An inner anchor's handles are collinear, parallel to the neighbours' chord, and a third of the
// distance to each neighbour long (uneven spacing).
#[test]
#[ignore = "skeleton"]
fn curvature_handles_are_collinear_and_one_third_of_the_neighbour_distance() {
    todo!()
}

// An open path's first and last anchors aim at their neighbour's nearer handle, a third of the
// distance long; their outer handles stay on the anchor.
#[test]
#[ignore = "skeleton"]
fn curvature_open_ends_aim_at_the_neighbours_nearer_handle() {
    todo!()
}

// `corners[i]` keeps anchor i's handles on the anchor; one or two anchors give straight segments,
// open or closed.
#[test]
#[ignore = "skeleton"]
fn curvature_corners_retract_and_two_anchors_are_straight() {
    todo!()
}

// Two or three coincident anchors give handles on the anchor and no NaN.
#[test]
#[ignore = "skeleton"]
fn curvature_coincident_points_give_retracted_handles() {
    todo!()
}
