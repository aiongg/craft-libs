//! Tests of the preview stroke and the stroke strip (task BR-BS3).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// 65 samples; the first at x = 10 and the last at width − 10, both at mid-height with pressure
// 0.05; the curve's top at s = 0.25 is 0.22 × height above the middle; `fit` is 1 for a small
// brush and 0.55 × height / diameter for a large one.
#[test]
#[ignore = "skeleton"]
fn preview_stroke_samples_match_the_s_curve() {
    todo!()
}

// The strip's render closure runs on the first pass, not again for the same revision and size,
// and again after the revision changes.
#[test]
#[ignore = "skeleton"]
fn strip_renders_once_per_revision() {
    todo!()
}
