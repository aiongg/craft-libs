//! Tests of Hue/Saturation (task AD-4).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Colorize on: the range select is disabled and shows Master, Hue −90 becomes 270, Saturation 10 becomes 25; off: Hue 270 becomes −90.
#[test]
#[ignore = "skeleton"]
fn hue_saturation_colorize_forces_master_and_wraps_hue() {
    todo!()
}

// Dragging the Reds' range-start thumb across 0° keeps the four bounds increasing.
#[test]
#[ignore = "skeleton"]
fn hue_saturation_range_thumbs_wrap_through_zero() {
    todo!()
}

// `HueRangeValue::move_bound` canonicalises before clamping (PhotoCraft's `hue_range_drag_canonicalizes_bounds_before_clamping`).
#[test]
#[ignore = "skeleton"]
fn hue_saturation_range_thumb_move_canonicalises_bounds_first() {
    todo!()
}

// The Saturation track runs grey to the current hue; the Lightness track is black, grey, white at any hue (PhotoCraft's two track tests).
#[test]
#[ignore = "skeleton"]
fn hue_saturation_saturation_track_follows_hue_and_lightness_track_is_fixed() {
    todo!()
}

// Each range thumb is a Tab stop and a `Slider` with its name and degrees; → moves it 1°, Shift 10°.
#[test]
#[ignore = "skeleton"]
fn hue_saturation_range_thumbs_take_the_keyboard() {
    todo!()
}
