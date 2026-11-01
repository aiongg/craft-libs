//! Tests of the path operations (task PE-A2).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A curve split at t = 0.3 draws the same curve (sampled); a straight segment gets a corner on the
// cubic's point at t; t of 0, 1 or NaN is refused and changes nothing.
#[test]
#[ignore = "skeleton"]
fn edit_subpath_insert_at_splits_a_curve_without_changing_it() {
    todo!()
}

// Removing an inner knot joins its neighbours with the previous out and next in handles; a closed
// subpath stays closed; removing the only knot leaves an empty subpath.
#[test]
#[ignore = "skeleton"]
fn edit_subpath_remove_knot_joins_neighbours_and_keeps_closed() {
    todo!()
}

// PhotoCraft's closed four-anchor loop: moving one anchor reflows every smooth knot (the
// neighbours turn), corner knots keep their handles.
#[test]
#[ignore = "skeleton"]
fn path_edit_move_curvature_reflows_every_smooth_knot() {
    todo!()
}

// A corner square: toggling a knot makes it smooth and the segments around it curve; toggling
// again retracts its handles.
#[test]
#[ignore = "skeleton"]
fn path_edit_toggle_curvature_flips_smooth_and_corner() {
    todo!()
}

// `InsertCurvature` on a straight segment adds a smooth knot moved by `by` and reflows.
#[test]
#[ignore = "skeleton"]
fn path_edit_insert_curvature_adds_a_smooth_anchor_and_moves_it() {
    todo!()
}

// MoveAnchors, MoveHandle (smooth and alone), BendSegment (straight and curved), Convert give
// PhotoCraft's kernel results; a wrong key or index changes nothing and returns false.
#[test]
#[ignore = "skeleton"]
fn path_edit_apply_matches_the_kernel_for_every_existing_edit() {
    todo!()
}
