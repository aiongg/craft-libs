//! Gradient lines: the line, end handles, colour stops and midpoint diamonds a gradient is edited
//! with on the canvas. Their colours are the semantic `overlay.*` tokens or the stop's own; the
//! family holds their geometry, the same at every Size because the line sits on the document.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::values;

/// Gradient line tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct GradientLineTokens {
    /// Width of the gradient line: `gradient_line.line`.
    pub line: f32,
    /// Width of the dark halo on each side of the line: `gradient_line.halo`.
    pub halo: f32,
    /// Radius of an end handle's ring: `gradient_line.end_ring`.
    pub end_ring: f32,
    /// Width of an end handle's ring: `gradient_line.end_width`.
    pub end_width: f32,
    /// Width of the dark halo on each side of an end handle's ring: `gradient_line.end_halo`.
    pub end_halo: f32,
    /// Radius of a colour stop's disc: `gradient_line.stop`.
    pub stop: f32,
    /// Width of the ring round a colour stop's disc: `gradient_line.stop_ring`.
    pub stop_ring: f32,
    /// Radius of a midpoint's diamond: `gradient_line.diamond`.
    pub diamond: f32,
    /// Distance from a part within which the pointer takes it: `gradient_line.hit`.
    pub hit: f32,
    /// Farthest a press may move and still be a click: `gradient_line.click_slop`.
    pub click_slop: f32,
    /// Distance from the line past which a dragged stop is torn off: `gradient_line.tear_off`.
    pub tear_off: f32,
    /// Dash length of a mirrored extent's line: `gradient_line.dash`.
    pub dash: f32,
    /// Gap between a mirrored extent's dashes: `gradient_line.gap`.
    pub gap: f32,
}

/// Gradient lines have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GradientLineField {}

impl Family for GradientLineTokens {
    type Field = GradientLineField;
    const FIELDS: &'static [GradientLineField] = &[];

    fn defaults(_: &Semantic<'_>) -> GradientLineTokens {
        GradientLineTokens {
            line: values::GRADIENT_LINE_LINE,
            halo: values::GRADIENT_LINE_HALO,
            end_ring: values::GRADIENT_LINE_END_RING,
            end_width: values::GRADIENT_LINE_END_WIDTH,
            end_halo: values::GRADIENT_LINE_END_HALO,
            stop: values::GRADIENT_LINE_STOP,
            stop_ring: values::GRADIENT_LINE_STOP_RING,
            diamond: values::GRADIENT_LINE_DIAMOND,
            hit: values::GRADIENT_LINE_HIT,
            click_slop: values::GRADIENT_LINE_CLICK_SLOP,
            tear_off: values::GRADIENT_LINE_TEAR_OFF,
            dash: values::GRADIENT_LINE_DASH,
            gap: values::GRADIENT_LINE_GAP,
        }
    }

    fn path(f: GradientLineField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<GradientLineField> {
        None
    }

    fn kind(f: GradientLineField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: GradientLineField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: GradientLineField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
