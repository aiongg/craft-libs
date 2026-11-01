//! Tests of the stroke preview painter (task PE-A4).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A dashed open line gives one set of convex pieces per dash; round caps add a fan at each dash
// end; a closed sample's miter, round and bevel corners add a wedge, a fan and a triangle.
#[test]
#[ignore = "skeleton"]
fn stroke_preview_dashes_follow_cap_and_join() {
    todo!()
}

// An invalid style draws nothing; opacity 0.5 halves the colour's alpha; a right-to-left layout
// still draws left to right.
#[test]
#[ignore = "skeleton"]
fn stroke_preview_skips_invalid_styles_and_applies_opacity() {
    todo!()
}
