//! Tests of Brightness/Contrast, Threshold, Color Balance and Photo Filter (task AD-3).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Turning Use Legacy off with Contrast at −80 reports one `Set` with Contrast −50 and legacy false.
#[test]
#[ignore = "skeleton"]
fn brightness_contrast_use_legacy_off_clamps_contrast_to_minus_fifty() {
    todo!()
}

// Threshold's marker sits at `level / 255` across the histogram and the row reports the level.
#[test]
#[ignore = "skeleton"]
fn threshold_marker_follows_the_level() {
    todo!()
}

// The tone picked (Shadows) survives a value change from outside; `reset_tone` shows Midtones again; a double-click zeroes only that tone's row.
#[test]
#[ignore = "skeleton"]
fn color_balance_tone_choice_survives_a_value_change_and_resets_to_midtones() {
    todo!()
}

// Clicking Preserve Luminosity reports one `Set` with it toggled.
#[test]
#[ignore = "skeleton"]
fn color_balance_preserve_luminosity_reports_a_set() {
    todo!()
}

// Picking Cooling Filter (80) reports a `Set` of (0, 109, 255); a click on the swatch reports a `ColorRequest` keyed `photoFilter.color`.
#[test]
#[ignore = "skeleton"]
fn photo_filter_preset_sets_its_colour_and_the_swatch_asks_for_one() {
    todo!()
}
