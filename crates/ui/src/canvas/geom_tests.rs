//! Canvas geometry.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// `point_in_polygon`: inside a square, outside it, and inside both loops of a figure eight drawn the same way round.
#[test]
#[ignore = "skeleton"]
fn nonzero_winding_fills_a_self_crossing_outline() {
    todo!()
}

// `clip_segment`: a crossing segment is cut at both edges; one outside is `None`; one inside is unchanged.
#[test]
#[ignore = "skeleton"]
fn segments_clip_to_the_rect() {
    todo!()
}

// `angle_deg`: (1, 0) is 0, (0, 1) is 90, (−1, 0) is 180.
#[test]
#[ignore = "skeleton"]
fn angles_run_clockwise_on_screen() {
    todo!()
}

// `project_clipped`: with the identity every point stays; a matrix putting two corners at w < 0.5 cuts the polygon at w = 0.5.
#[test]
#[ignore = "skeleton"]
fn a_polygon_behind_the_camera_is_cut() {
    todo!()
}
