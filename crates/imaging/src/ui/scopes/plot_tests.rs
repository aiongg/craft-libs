//! The plot's data choices.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Colors in an RGB set is `HistogramData::rgb` (additive); in a CMYK set `HistogramData::channels`
// of the four inks (multiplied).
#[test]
#[ignore = "skeleton"]
fn scope_colors_channel_overlays_light_and_ink() {
    todo!()
}

// A channel key the mode lacks resolves to the composite; Colors without `colors` does too.
#[test]
#[ignore = "skeleton"]
fn scope_channel_list_follows_the_mode_and_falls_back_to_composite() {
    todo!()
}
