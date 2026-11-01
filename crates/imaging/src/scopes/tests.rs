//! The pure scope helpers.

use super::*;

// Mean, Std Dev, the median rule (half reached), the percentile of a level and of a range, and the
// clipped percents, against counts worked out by hand (PhotoCraft's `histogram_panel/tests.rs`).
#[test]
fn histogram_statistics_match_the_reference_counts() {
    assert!(histogram_statistics(&[0; 256], None).is_none());

    // 2 pixels at 0, 1 at 100, 1 at 255: N = 4, mean 88.75.
    let mut bins = [0u32; 256];
    bins[0] = 2;
    bins[100] = 1;
    bins[255] = 1;
    let all = histogram_statistics(&bins, None).unwrap();
    assert_eq!(all.pixels, 4);
    assert!((all.mean - 88.75).abs() < 1e-9);
    let variance = (2.0 * 88.75f64.powi(2) + 11.25f64.powi(2) + 166.25f64.powi(2)) / 4.0;
    assert!((all.standard_deviation - variance.sqrt()).abs() < 1e-9);
    assert_eq!(all.median, 0, "half the pixels are reached at level 0");
    assert_eq!((all.level, all.count, all.percentile), (None, None, None));
    assert_eq!(all.clipped, [50.0, 25.0]);

    let one = histogram_statistics(&bins, Some([100, 100])).unwrap();
    assert_eq!((one.level, one.count), (Some([100, 100]), Some(1)));
    assert_eq!(one.percentile, Some(75.0), "a level reports the percent at or below it");

    let range = histogram_statistics(&bins, Some([255, 100])).unwrap();
    assert_eq!((range.level, range.count), (Some([100, 255]), Some(2)));
    assert_eq!(range.percentile, Some(50.0), "a range reports the percent inside it");
}

// Channels at <= 0 or >= 1 clip; transparent and NaN pixels clip nothing; `ClippingMask` agrees
// pixel by pixel and `any` folds them.
#[test]
fn clipping_rule_uses_exact_endpoints() {
    let pixels = [
        [0.0, 0.5, 1.0, 1.0],
        [0.001, 0.999, 0.5, 1.0],
        [0.0, 1.0, 0.0, 0.0],
        [f32::NAN, 1.0, 0.0, 1.0],
        [-0.5, 2.0, 0.5, 1.0],
        [0.5, 0.5, 0.5, 1.0],
    ];
    let first = clipping(pixels[0]);
    assert_eq!((first.shadows, first.highlights), (ChannelMask::R, ChannelMask::B));
    assert_eq!(clipping(pixels[1]), ClippedChannels::default());
    assert_eq!(clipping(pixels[2]), ClippedChannels::default());
    assert_eq!(clipping(pixels[3]), ClippedChannels::default());

    assert!(ClippingMask::from_rgba([2, 2], &pixels).is_none());
    let mask = ClippingMask::from_rgba([3, 2], &pixels).unwrap();
    for (index, pixel) in pixels.iter().enumerate() {
        assert_eq!(mask.at(index % 3, index / 3), clipping(*pixel));
    }
    assert_eq!(mask.at(3, 0), ClippedChannels::default());
    assert_eq!(mask.at(0, 2), ClippedChannels::default());
    let any = mask.any();
    assert_eq!((any.shadows, any.highlights), (ChannelMask::R, ChannelMask::G.union(ChannelMask::B)));
    assert_eq!(ClippingMask::new([2, 3]).any(), ClippedChannels::default());
}

// Red, yellow, green and blue land in their hue cells at full saturation; grey at saturation 0;
// transparent and NaN pixels are skipped (PhotoCraft's `rgb_primary_positions_neutrals_and_transparency`).
#[test]
fn vectorscope_counts_primaries_neutrals_and_skips_transparent() {
    for (rgb, hue) in [([1.0, 0.0, 0.0], 0), ([1.0, 1.0, 0.0], HUES / 6), ([0.0, 1.0, 0.0], HUES / 3), ([0.0, 0.0, 1.0], HUES * 2 / 3)] {
        let scope = Vectorscope::from_rgba(&[[rgb[0], rgb[1], rgb[2], 1.0]]);
        let index = hue * SATURATIONS + SATURATIONS - 1;
        assert_eq!(scope.counts[index], 1, "{rgb:?}");
        let (turns, saturation) = Vectorscope::cell(index);
        assert!((turns - hue as f32 / HUES as f32).abs() < 1e-6 && saturation == 1.0);
    }
    let scope = Vectorscope::from_rgba(&[[0.5, 0.5, 0.5, 1.0], [1.0, 0.0, 0.0, 1.0], [0.0, 1.0, 0.0, 0.0], [f32::NAN, 0.0, 0.0, 1.0]]);
    assert_eq!(scope.samples, 2);
    assert_eq!(scope.counts[0], 1);
    assert_eq!(scope.counts[SATURATIONS - 1], 1);
    assert_eq!(scope.peak(), 1);

    let mut merged = scope.clone();
    merged.merge(&scope);
    assert_eq!((merged.samples, merged.peak()), (4, 2));

    assert!(Vectorscope::default().summary().is_none());
    let summary = Vectorscope::from_rgba(&[[0.0, 1.0, 0.0, 1.0]]).summary().unwrap();
    assert_eq!(summary.peak_saturation, 1.0);
    assert!((summary.mean_hue - 120.0).abs() < 2.0);
}
