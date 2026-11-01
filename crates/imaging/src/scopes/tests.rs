//! The pure scope helpers.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Mean, Std Dev, the median rule (half reached), the percentile of a level and of a range, and the
// clipped percents, against counts worked out by hand (PhotoCraft's `histogram_panel/tests.rs`).
#[test]
#[ignore = "skeleton"]
fn histogram_statistics_match_the_reference_counts() {
    todo!()
}

// Channels at <= 0 or >= 1 clip; transparent and NaN pixels clip nothing; `ClippingMask` agrees
// pixel by pixel and `any` folds them.
#[test]
#[ignore = "skeleton"]
fn clipping_rule_uses_exact_endpoints() {
    todo!()
}

// Red, yellow, green and blue land in their hue cells at full saturation; grey at saturation 0;
// transparent and NaN pixels are skipped (PhotoCraft's `rgb_primary_positions_neutrals_and_transparency`).
#[test]
#[ignore = "skeleton"]
fn vectorscope_counts_primaries_neutrals_and_skips_transparent() {
    todo!()
}
