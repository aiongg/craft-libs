//! `FullWindowView` and `FullWindowZoomKeys` without a harness.

#![expect(clippy::todo, reason = "skeleton: K16 fills these tests")]

// A new view fits: the image centred with the margin on each side, at most 400 %; a resize
// refits; a zoom or a pan ends the fit and `reset_fit` returns to it.
#[test]
#[ignore = "skeleton"]
fn canvas_fits_on_first_pass_and_until_the_user_zooms() {
    todo!()
}

// `step` follows `ZoomLadder` from the zoom on screen (the fitted zoom too) and stops at the
// ends of the range for the image's size; `apply` maps In, Out, Fit and ActualPixels.
#[test]
#[ignore = "skeleton"]
fn zoom_steps_follow_the_ladder_from_the_zoom_on_screen() {
    todo!()
}
