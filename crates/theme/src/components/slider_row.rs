//! Slider rows: the number field width and the space after a caption, field and slider row. Sizes only (no colours): they scale with the
//! control height, and no theme overrides them.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::values;

/// Slider row tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SliderRowTokens {
    /// Width of the number field at the reading end of the caption line: `slider_row.field_width`.
    pub field_width: f32,
    /// Space after the row: `slider_row.gap`.
    pub gap: f32,
}

/// Slider rows have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SliderRowField {}

impl Family for SliderRowTokens {
    type Field = SliderRowField;
    const FIELDS: &'static [SliderRowField] = &[];

    /// The Compact sizes (PhotoCraft's), scaled by the control height of the size and rounded to
    /// whole points.
    fn defaults(s: &Semantic<'_>) -> SliderRowTokens {
        let k = s.size.control.md / values::SLIDER_ROW_BASE_CONTROL;
        let scale = |w: f32| (w * k).round();
        SliderRowTokens { field_width: scale(values::SLIDER_ROW_FIELD_WIDTH), gap: scale(values::SLIDER_ROW_GAP) }
    }

    fn path(f: SliderRowField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<SliderRowField> {
        None
    }

    fn kind(f: SliderRowField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: SliderRowField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: SliderRowField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
