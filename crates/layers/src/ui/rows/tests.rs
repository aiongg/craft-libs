//! Screen tests of `BlendRows` (L3).

#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A wheel notch over the closed blend select reports one `Blend(Set)` per notch; hovering a mode
// in the open list reports `BlendHover(Some(key))`, leaving it `BlendHover(None)`.
#[test]
#[ignore = "skeleton"]
fn blend_select_steps_with_the_wheel_and_reports_hover() {
    todo!()
}

// A scrub of "Opacity:" reports `Begin`, `Update`s and one `Commit`; Esc during it reports `Cancel`.
#[test]
#[ignore = "skeleton"]
fn opacity_drag_is_one_gesture_and_cancel_restores() {
    todo!()
}

// The "Opacity:" label hides below `label + field + 2 gaps + 16 + layers.blend.min_width`; the
// field stays (`opacity_row_layout` and the drawn rows).
#[test]
#[ignore = "skeleton"]
fn opacity_label_hides_on_a_narrow_panel() {
    todo!()
}

// With Lock image pixels on, Lock transparent pixels is disabled and described as included.
#[test]
#[ignore = "skeleton"]
fn lock_row_image_pixels_disables_transparent_pixels() {
    todo!()
}

// `editable` false greys the select, Opacity and Fill but not the locks; no properties greys every
// control at Normal, 100 % and unlocked.
#[test]
#[ignore = "skeleton"]
fn background_and_no_layer_grey_the_property_rows() {
    todo!()
}
