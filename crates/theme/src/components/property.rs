//! Property rows: the columns of an inspector row. Sizes only (no colours): they scale with the
//! control height, and no theme overrides them.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::values;

/// Property row tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PropertyTokens {
    /// Width of the label column of a glyph label (W, H, X, Y): `property.glyph_width`.
    pub glyph_width: f32,
    /// Width of the label column of a word label (Opacity, Angle): `property.word_width`.
    pub word_width: f32,
    /// The narrowest a control is drawn before a redundant label hides: `property.min_control`.
    pub min_control: f32,
    /// Between the two columns of a pair row: `property.column_gap`.
    pub column_gap: f32,
    /// Width of the slot between a pair's columns (a link toggle): `property.link_width`.
    pub link_width: f32,
}

/// Property rows have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PropertyField {}

impl Family for PropertyTokens {
    type Field = PropertyField;
    const FIELDS: &'static [PropertyField] = &[];

    /// The Compact widths (PhotoCraft's), scaled by the control height of the size and rounded
    /// to whole points.
    fn defaults(s: &Semantic<'_>) -> PropertyTokens {
        let k = s.size.control.md / values::PROPERTY_BASE_CONTROL;
        let scale = |w: f32| (w * k).round();
        PropertyTokens {
            glyph_width: scale(values::PROPERTY_GLYPH_WIDTH),
            word_width: scale(values::PROPERTY_WORD_WIDTH),
            min_control: scale(values::PROPERTY_MIN_CONTROL),
            column_gap: scale(values::PROPERTY_COLUMN_GAP),
            link_width: scale(values::PROPERTY_LINK_WIDTH),
        }
    }

    fn path(f: PropertyField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<PropertyField> {
        None
    }

    fn kind(f: PropertyField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: PropertyField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: PropertyField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
