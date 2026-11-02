//! The adjustment editors: Hue/Saturation's spectrum bars and range thumbs, the point pickers' tone dot and Threshold's histogram. The same at every Size (they sit on the image).

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
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
    /// A range thumb's corner radius: `adjust.range_thumb_radius`.
    pub range_thumb_radius: f32,
    /// Distance from a press within which a range thumb is grabbed: `adjust.hue_grab`.
    pub hue_grab: f32,
    /// A point picker's tone dot radius: `adjust.picker_dot`.
    pub picker_dot: f32,
    /// The tone dot's centre, in from the button's bottom-trailing corner: `adjust.picker_dot_inset`.
    pub picker_dot_inset: f32,
    /// The 1 pt ring round a tone dot: `adjust.picker_ring`, `line.control_edge`.
    pub picker_ring: Rgba,
    /// How far a disabled picker's tone dot fades, as an alpha multiplier: `adjust.disabled_alpha`.
    pub disabled_alpha: f32,
    /// Threshold's histogram height: `adjust.threshold_histogram`.
    pub threshold_histogram: f32,
    /// Threshold's histogram widest width: `adjust.threshold_width`.
    pub threshold_width: f32,
    /// Threshold's level marker, in from the histogram's edges so the marker stays inside the
    /// plot at either end: `adjust.threshold_inset`.
    pub threshold_inset: f32,
    /// Width of Threshold's level marker: `adjust.threshold_marker_width`.
    pub threshold_marker_width: f32,
}

/// An overridable adjustment field. Path: `picker_ring`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AdjustField {
    /// `adjust.picker_ring`
    PickerRing,
}

impl Family for AdjustTokens {
    type Field = AdjustField;
    const FIELDS: &'static [AdjustField] = &[AdjustField::PickerRing];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { AdjustTokens;
        "spectrum_bar" 0.0..=64.0 => spectrum_bar;
        "range_band" 0.0..=64.0 => range_band;
        "spectrum_master_extra" 0.0..=128.0 => spectrum_master_extra;
        "range_thumb_width" 0.0..=64.0 => range_thumb_width;
        "range_thumb_height" 0.0..=64.0 => range_thumb_height;
        "range_thumb_radius" 0.0..=32.0 => range_thumb_radius;
        "hue_grab" 0.0..=64.0 => hue_grab;
        "picker_dot" 0.0..=32.0 => picker_dot;
        "picker_dot_inset" 0.0..=32.0 => picker_dot_inset;
        "threshold_histogram" 0.0..=512.0 => threshold_histogram;
        "threshold_width" 0.0..=2048.0 => threshold_width;
        "threshold_inset" 0.0..=64.0 => threshold_inset;
        "threshold_marker_width" 0.0..=16.0 => threshold_marker_width;
    };

    fn defaults(s: &Semantic<'_>) -> AdjustTokens {
        AdjustTokens {
            spectrum_bar: values::ADJUST_SPECTRUM_BAR,
            range_band: values::ADJUST_RANGE_BAND,
            spectrum_master_extra: values::ADJUST_SPECTRUM_MASTER_EXTRA,
            range_thumb_width: values::ADJUST_RANGE_THUMB_WIDTH,
            range_thumb_height: values::ADJUST_RANGE_THUMB_HEIGHT,
            range_thumb_radius: values::ADJUST_RANGE_THUMB_RADIUS,
            hue_grab: values::ADJUST_HUE_GRAB,
            picker_dot: values::ADJUST_PICKER_DOT,
            picker_dot_inset: values::ADJUST_PICKER_DOT_INSET,
            picker_ring: s.color.line.control_edge,
            disabled_alpha: values::ADJUST_DISABLED_ALPHA,
            threshold_histogram: values::ADJUST_THRESHOLD_HISTOGRAM,
            threshold_width: values::ADJUST_THRESHOLD_WIDTH,
            threshold_inset: values::ADJUST_THRESHOLD_INSET,
            threshold_marker_width: values::ADJUST_THRESHOLD_MARKER_WIDTH,
        }
    }

    fn path(_: AdjustField) -> String {
        "picker_ring".to_owned()
    }

    fn from_path(path: &str) -> Option<AdjustField> {
        (path == "picker_ring").then_some(AdjustField::PickerRing)
    }

    fn kind(_: AdjustField) -> FieldKind {
        FieldKind::Color
    }

    fn get(&self, _: AdjustField) -> OverrideValue {
        OverrideValue::Color(self.picker_ring)
    }

    fn set(&mut self, _: AdjustField, v: OverrideValue) {
        if let OverrideValue::Color(c) = v {
            self.picker_ring = c;
        }
    }

    // The ring is decoration round an image tone, so it has no contrast pair.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
