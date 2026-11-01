//! Tests of the symmetry model (task TO-S1).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// `axes`: Vertical and Dual start at 90°, Horizontal at 0°, Diagonal at 45° (its direction's x equals y), plus the rotation clockwise; Dual has two.
#[test]
#[ignore = "skeleton"]
fn axes_start_at_90_0_45_degrees_plus_rotation_and_dual_adds_the_perpendicular() {
    todo!()
}

// PhotoCraft's cases: every mode on 100 x 100 and 101 x 53 gives segments inside the page; a horizontal axis at y -5 gives none.
#[test]
#[ignore = "skeleton"]
fn segments_clip_to_the_page_and_vanish_when_the_centre_line_misses_it() {
    todo!()
}

// `Symmetry::new` refuses NaN, infinity and a centre past 1,000,000 and wraps 370 to 10 and -90 to 270.
#[test]
#[ignore = "skeleton"]
fn new_rejects_non_finite_and_wraps_rotation() {
    todo!()
}

// At 0.35 of the shorter side along the first axis, at least 1 unit from the centre.
#[test]
#[ignore = "skeleton"]
fn the_rotation_handle_sits_at_a_share_of_the_shorter_side() {
    todo!()
}
