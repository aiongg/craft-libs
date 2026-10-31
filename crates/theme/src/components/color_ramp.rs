//! Colour ramps: the bar under the slider rows, with white and black boxes at its end. The sizes
//! scale with the control height, and no theme overrides them; the colours are semantic tokens.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// ColorRamp tokens. Sizes are in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ColorRampTokens {
    /// Height when the host gives none: `color_ramp.height`.
    pub height: f32,
    /// Least height: `color_ramp.height_min`.
    pub height_min: f32,
    /// Width of the white and black boxes: `color_ramp.boxes`.
    pub boxes: f32,
    /// Gap between the bar and the boxes: `color_ramp.boxes_gap`.
    pub boxes_gap: f32,
    /// Cells across the bar: `color_ramp.cells_x`.
    pub cells_x: u32,
    /// Cells down the bar: `color_ramp.cells_y`.
    pub cells_y: u32,
}

/// ColorRamps have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ColorRampField {}

impl Family for ColorRampTokens {
    type Field = ColorRampField;
    const FIELDS: &'static [ColorRampField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { ColorRampTokens;
        "height" 0.0..=1024.0 => height;
        "height_min" 0.0..=1024.0 => height_min;
        "boxes" 0.0..=1024.0 => boxes;
        "boxes_gap" 0.0..=64.0 => boxes_gap;
    };

    /// The Compact sizes, scaled by the control height of the size and rounded to whole points.
    fn defaults(s: &Semantic<'_>) -> ColorRampTokens {
        let k = s.size.control.md / values::COLOR_WHEEL_BASE_CONTROL;
        let scale = |w: f32| if k == 1.0 { w } else { (w * k).round() };
        ColorRampTokens {
            height: scale(values::COLOR_RAMP_HEIGHT),
            height_min: scale(values::COLOR_RAMP_HEIGHT_MIN),
            boxes: scale(values::COLOR_RAMP_BOXES),
            boxes_gap: scale(values::COLOR_RAMP_BOXES_GAP),
            cells_x: values::COLOR_RAMP_CELLS_X,
            cells_y: values::COLOR_RAMP_CELLS_Y,
        }
    }

    fn path(f: ColorRampField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<ColorRampField> {
        None
    }

    fn kind(f: ColorRampField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: ColorRampField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: ColorRampField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
