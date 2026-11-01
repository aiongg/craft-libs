//! The border edge picker: the square and its lines. No theme overrides it.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
use crate::values;

/// EdgePicker tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct EdgePickerTokens {
    /// The square's side: [`values::EDGE_PICKER_SIDE`].
    pub side: f32,
    /// The lines' inset from the square: [`values::EDGE_PICKER_INSET`].
    pub inset: f32,
    /// A chosen line: [`values::EDGE_PICKER_LINE_ON`].
    pub line_on: f32,
    /// A line not chosen: [`values::EDGE_PICKER_LINE_OFF`].
    pub line_off: f32,
    /// The line under the pointer: [`values::EDGE_PICKER_LINE_HOVER`].
    pub line_hover: f32,
    /// The field inside the inset: `edge_picker.fill`, `surface.sunken`.
    pub fill: Rgba,
    /// The colour of a line not chosen: `edge_picker.line_off_color`, `text.faint`.
    pub line_off_color: Rgba,
}

/// EdgePicker tokens have no overridable field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EdgePickerField {}

impl Family for EdgePickerTokens {
    type Field = EdgePickerField;
    const FIELDS: &'static [EdgePickerField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { EdgePickerTokens;
        "side" 16.0..=256.0 => side;
        "inset" 0.0..=64.0 => inset;
        "line_on" 0.0..=16.0 => line_on;
        "line_off" 0.0..=16.0 => line_off;
        "line_hover" 0.0..=16.0 => line_hover;
    };

    fn defaults(s: &Semantic<'_>) -> EdgePickerTokens {
        let c = s.color;
        EdgePickerTokens {
            side: values::EDGE_PICKER_SIDE,
            inset: values::EDGE_PICKER_INSET,
            line_on: values::EDGE_PICKER_LINE_ON,
            line_off: values::EDGE_PICKER_LINE_OFF,
            line_hover: values::EDGE_PICKER_LINE_HOVER,
            fill: c.surface.sunken,
            line_off_color: c.text.faint,
        }
    }

    fn path(f: EdgePickerField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<EdgePickerField> {
        None
    }

    fn kind(f: EdgePickerField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: EdgePickerField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: EdgePickerField, _: OverrideValue) {
        match f {}
    }

    // Marks and tints drawn over surfaces other families check.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
