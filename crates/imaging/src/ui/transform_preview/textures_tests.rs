//! Tests of the preview textures (task TR-T4).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Below ½ screen pixel per source pixel a mip level (zoom 0.25 → level 2, at most 12); around 1:1
// level 0 linear; at zoom ≥ 2 with scale ≤ 1.05 level 0 nearest; a scale over 1.05 stays linear.
#[test]
#[ignore = "skeleton"]
fn the_texture_level_follows_the_on_screen_size() {
    todo!()
}

// With nearest interpolation every zoom and scale gives level 0 with nearest filtering.
#[test]
#[ignore = "skeleton"]
fn nearest_interpolation_keeps_level_zero_and_nearest_filtering() {
    todo!()
}

// A display conversion changes only the displayed upload: `level_image` stays raw, a new key
// replaces the uploads of the old one.
#[test]
#[ignore = "skeleton"]
fn the_display_conversion_changes_only_the_displayed_upload() {
    todo!()
}

// A mask value of 0 at density 1 clears a texel; 128 at density 0.5 keeps three quarters of its
// alpha and colour.
#[test]
#[ignore = "skeleton"]
fn the_mask_scales_the_texel_alpha() {
    todo!()
}

// Mip levels are built only when asked for, halve down to one texel, and keep the uv extent of the
// source.
#[test]
#[ignore = "skeleton"]
fn mip_levels_are_built_on_demand_and_keep_the_source_extent() {
    todo!()
}
