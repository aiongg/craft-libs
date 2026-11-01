//! Tests of the styled transform preview (task TR-T11).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Two passes with the same box, zoom, revision, display key, level and filter build once; a
// change of any one builds again.
#[test]
#[ignore = "skeleton"]
fn the_styled_frame_is_rebuilt_only_when_its_key_changes() {
    todo!()
}

// A costly style shrinks the work scale until the content's area fits the budget, in up to 6
// rounds; a box off the canvas gives None.
#[test]
#[ignore = "skeleton"]
fn the_budget_shrinks_the_work_scale_until_the_composite_fits() {
    todo!()
}

// The budget is 30 ms dragging and 120 ms at rest over the cost per pixel, clamped to 0.25 to 8
// million pixels.
#[test]
#[ignore = "skeleton"]
fn the_budget_follows_what_the_style_costs() {
    todo!()
}

// Resampling a texel image through a box keeps texels whole with nearest, fades the source's
// edges over one texel, and leaves empty texels transparent.
#[test]
#[ignore = "skeleton"]
fn resampling_handles_empty_texels_and_nearest_neighbor() {
    todo!()
}
