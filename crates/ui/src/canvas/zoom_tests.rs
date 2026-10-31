//! The zoom ladder, fit and fill, zoom about a point, the hold, the Zoom tool's drags, Zoom Resizes Windows.

// 880 % steps in to 1200 % and out to 800 %; 12800 % does not step in.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn zoom_in_and_out_step_from_any_zoom_to_the_next_rung() {
    todo!()
}

// A 2000 × 1500 document goes down to 2/1500; an empty size gives 0.1 %.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn the_lowest_zoom_shows_the_shorter_side_two_pixels() {
    todo!()
}

// A small document fits at 100 % on a 2× display (view.zoom 0.5), centred, 20-point margins.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn fit_never_magnifies_past_one_device_pixel() {
    todo!()
}

// The document covers the canvas; one axis overflows.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn fill_covers_the_canvas_on_one_axis() {
    todo!()
}

// The document point under the pointer maps to the same screen point after the zoom, in a turned and mirrored view.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn a_zoom_about_the_pointer_keeps_its_document_point_under_it() {
    todo!()
}

// With the preference the clicked document point lands at the view centre.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn zoom_clicked_point_to_center_centres_the_point() {
    todo!()
}

// A pointer on the pasteboard anchors at the image's nearest edge point.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn the_wheel_anchor_is_the_nearest_document_point_off_the_image() {
    todo!()
}

// Shown views pass between the start and the target and equal the target after 0.22 s.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn an_animated_zoom_eases_to_the_target_and_ends_on_it() {
    todo!()
}

// Two doublings a second whether passes are 10 or 100 ms apart; a 1 s stall moves at most 0.25 s.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn a_held_press_zooms_at_the_same_rate_at_any_frame_rate() {
    todo!()
}

// Moving 7 points before 0.3 s leaves the zoom alone.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn a_press_that_moves_past_the_slop_never_becomes_a_hold() {
    todo!()
}

// A drag of +100 points doubles the zoom about the press point; -100 halves it.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn scrubby_zoom_doubles_per_hundred_points() {
    todo!()
}

// Releasing a rectangle zooms so it fills the canvas; under 3 points it does nothing.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn a_zoom_rectangle_fits_its_area_and_a_tiny_one_is_a_click() {
    todo!()
}

// The wrapped size includes the chrome, stays at least 240 × 160 and inside the screen less 48.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn a_zoomed_window_wraps_the_image_within_the_screen() {
    todo!()
}

// Same zoom or preference off: None; a fit never resizes.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn only_a_zoom_change_with_the_preference_on_resizes_the_window() {
    todo!()
}
