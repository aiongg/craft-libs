//! Tests of the Curves panel and its ⌘-click helper (task AD-2).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// A click on the pencil icon in points mode reports `mode: Some(Pencil)`; the current mode's icon reports nothing.
#[test]
#[ignore = "skeleton"]
fn curves_panel_mode_icons_switch_between_points_and_pencil() {
    todo!()
}

// Smooth is disabled in points mode; in pencil mode a click reports one `Set` of the radius-3 moving average of the shown map.
#[test]
#[ignore = "skeleton"]
fn curves_panel_smooth_is_disabled_in_points_mode_and_sets_in_pencil_mode() {
    todo!()
}

// On the RGB composite the editor gets the three other curves as overlays; on Red, and for an isolated channel, none.
#[test]
#[ignore = "skeleton"]
fn curves_panel_overlays_other_channels_on_the_composite_only() {
    todo!()
}

// A sample of grey 0.5 on the composite adds a point at input 128 (Rec. 709 luma), output the curve's there rounded, and returns its index.
#[test]
#[ignore = "skeleton"]
fn curves_add_point_from_sample_inserts_on_the_curve_at_the_sampled_tone() {
    todo!()
}

// With `every_channel` on RGB a sample adds one point to Red, Green and Blue each, at that channel's value.
#[test]
#[ignore = "skeleton"]
fn curves_add_point_from_sample_with_shift_adds_one_per_rgb_channel() {
    todo!()
}

// In pencil mode, for Lab or CMYK, and for an isolated channel the helper returns `None` and the value is unchanged.
#[test]
#[ignore = "skeleton"]
fn curves_add_point_from_sample_does_nothing_in_pencil_mode() {
    todo!()
}
