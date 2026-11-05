//! Rulers: the strips along the top and left of a canvas. They draw in `surface.chrome`,
//! `line.divider` and the text colours; the family holds their geometry, the same at every Size
//! and Text size (a 16 point strip has no room for larger labels).

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
use crate::values;

/// Ruler tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct RulerTokens {
    /// Thickness of a ruler strip and side of the corner square, in points: `ruler.size`.
    pub size: f32,
    /// Label font size: `ruler.label`.
    pub label: f32,
    /// Between the characters of the left ruler's stacked labels: `ruler.label_pitch`.
    pub label_pitch: f32,
    /// Between a top-ruler label and its tick: `ruler.label_pad`.
    pub label_pad: f32,
    /// Least distance between labelled ticks: `ruler.step_min`.
    pub step_min: f32,
    /// Length of the tick halfway between labelled ticks: `ruler.tick_mid`.
    pub tick_mid: f32,
    /// Length of the other minor ticks: `ruler.tick_minor`.
    pub tick_minor: f32,
    /// Half the width of a text-ruler marker: `ruler.marker_half`.
    pub marker_half: f32,
    /// The width of a table-ruler column marker: `values::RULER_MARKER_COLUMN`.
    pub marker_column: f32,
    /// Height of a text-ruler indent triangle: `ruler.marker_tip`.
    pub marker_tip: f32,
    /// Height of the left indent's box under its triangle: `ruler.marker_bar`.
    pub marker_bar: f32,
    /// Height of a tab stop's stem: `ruler.marker_stem`.
    pub marker_stem: f32,
    /// Length of a tab stop's foot, to either side of the stem: `ruler.marker_foot`.
    pub marker_foot: f32,
    /// Side of the tab selector's box: `ruler.marker_selector`.
    pub marker_selector: f32,
    /// The margin part of a ruler, where the text part keeps the ruler's fill: `ruler.margin_shade`, `surface.sunken`.
    pub margin_shade: Rgba,
}

/// Rulers have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RulerField {}

impl Family for RulerTokens {
    type Field = RulerField;
    const FIELDS: &'static [RulerField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { RulerTokens;
        "size" 0.0..=1024.0 => size;
        "label" 0.0..=1024.0 => label;
        "label_pitch" 0.0..=64.0 => label_pitch;
        "label_pad" 0.0..=64.0 => label_pad;
        "step_min" 0.0..=1024.0 => step_min;
        "tick_mid" 0.0..=1024.0 => tick_mid;
        "tick_minor" 0.0..=1024.0 => tick_minor;
        "marker_half" 0.0..=64.0 => marker_half;
        "marker_column" 0.0..=64.0 => marker_column;
        "marker_tip" 0.0..=64.0 => marker_tip;
        "marker_bar" 0.0..=64.0 => marker_bar;
        "marker_stem" 0.0..=64.0 => marker_stem;
        "marker_foot" 0.0..=64.0 => marker_foot;
        "marker_selector" 0.0..=64.0 => marker_selector;
    };

    fn defaults(s: &Semantic<'_>) -> RulerTokens {
        RulerTokens {
            size: values::RULER_SIZE,
            label: values::RULER_LABEL,
            label_pitch: values::RULER_LABEL_PITCH,
            label_pad: values::RULER_LABEL_PAD,
            step_min: values::RULER_STEP_MIN,
            tick_mid: values::RULER_TICK_MID,
            tick_minor: values::RULER_TICK_MINOR,
            marker_half: values::RULER_MARKER_HALF,
            marker_column: values::RULER_MARKER_COLUMN,
            marker_tip: values::RULER_MARKER_TIP,
            marker_bar: values::RULER_MARKER_BAR,
            marker_stem: values::RULER_MARKER_STEM,
            marker_foot: values::RULER_MARKER_FOOT,
            marker_selector: values::RULER_MARKER_SELECTOR,
            margin_shade: s.color.surface.sunken,
        }
    }

    fn path(f: RulerField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<RulerField> {
        None
    }

    fn kind(f: RulerField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: RulerField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: RulerField, _: OverrideValue) {
        match f {}
    }

    // Draws in semantic tokens; the margin shade sits beside `surface.chrome` strips.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
