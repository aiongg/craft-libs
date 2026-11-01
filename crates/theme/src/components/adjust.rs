//! The adjustment editors: Hue/Saturation's spectrum bars and range thumbs, the point pickers' tone dot and Threshold's histogram. The same at every Size (they sit on the image).

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// The adjustment editors: Hue/Saturation's spectrum bars and range thumbs, the point pickers' tone dot and Threshold's histogram. The same at every Size (they sit on the image).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct AdjustTokens {
    /// A Hue/Saturation spectrum bar's height: `adjust.spectrum_bar`.
    pub spectrum_bar: f32,
    /// The range band's height between the bars: `adjust.range_band`.
    pub range_band: f32,
    /// The height between the two bars that holds the range thumbs: `adjust.spectrum_master_extra`.
    pub spectrum_master_extra: f32,
    /// A range thumb's width: `adjust.range_thumb_width`.
    pub range_thumb_width: f32,
    /// A range thumb's height: `adjust.range_thumb_height`.
    pub range_thumb_height: f32,
    /// A point picker's tone dot radius: `adjust.picker_dot`.
    pub picker_dot: f32,
    /// The tone dot's centre, in from the button's bottom-trailing corner: `adjust.picker_dot_inset`.
    pub picker_dot_inset: f32,
    /// Threshold's histogram height: `adjust.threshold_histogram`.
    pub threshold_histogram: f32,
    /// Threshold's histogram widest width: `adjust.threshold_width`.
    pub threshold_width: f32,
}

/// This family has no overridable field: its sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AdjustField {}

impl Family for AdjustTokens {
    type Field = AdjustField;
    const FIELDS: &'static [AdjustField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { AdjustTokens;
        "spectrum_bar" 0.0..=64.0 => spectrum_bar;
        "range_band" 0.0..=64.0 => range_band;
        "spectrum_master_extra" 0.0..=128.0 => spectrum_master_extra;
        "range_thumb_width" 0.0..=64.0 => range_thumb_width;
        "range_thumb_height" 0.0..=64.0 => range_thumb_height;
        "picker_dot" 0.0..=32.0 => picker_dot;
        "picker_dot_inset" 0.0..=32.0 => picker_dot_inset;
        "threshold_histogram" 0.0..=512.0 => threshold_histogram;
        "threshold_width" 0.0..=2048.0 => threshold_width;
    };

    fn defaults(_: &Semantic<'_>) -> AdjustTokens {
        AdjustTokens {
            spectrum_bar: values::ADJUST_SPECTRUM_BAR,
            range_band: values::ADJUST_RANGE_BAND,
            spectrum_master_extra: values::ADJUST_SPECTRUM_MASTER_EXTRA,
            range_thumb_width: values::ADJUST_RANGE_THUMB_WIDTH,
            range_thumb_height: values::ADJUST_RANGE_THUMB_HEIGHT,
            picker_dot: values::ADJUST_PICKER_DOT,
            picker_dot_inset: values::ADJUST_PICKER_DOT_INSET,
            threshold_histogram: values::ADJUST_THRESHOLD_HISTOGRAM,
            threshold_width: values::ADJUST_THRESHOLD_WIDTH,
        }
    }

    fn path(f: AdjustField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<AdjustField> {
        None
    }

    fn kind(f: AdjustField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: AdjustField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: AdjustField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
