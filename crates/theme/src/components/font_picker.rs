//! The font picker: the open list and its class column.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// FontPicker tokens, in points, the same at every Size.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct FontPickerTokens {
    /// Default height of the open list, in points: `font_picker.list_height`.
    pub list_height: f32,
    /// Least width of the open list, in points: `font_picker.list_width`.
    pub list_width: f32,
    /// Width of the class list, in points: `font_picker.class_column`.
    pub class_column: f32,
    /// Width of a row's class glyph, in points: `font_picker.class_glyph`.
    pub class_glyph: f32,
}

/// FontPicker has no overridable field: its sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FontPickerField {}

impl Family for FontPickerTokens {
    type Field = FontPickerField;
    const FIELDS: &'static [FontPickerField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { FontPickerTokens;
        "list_height" 0.0..=2048.0 => list_height;
        "list_width" 0.0..=2048.0 => list_width;
        "class_column" 0.0..=1024.0 => class_column;
        "class_glyph" 0.0..=256.0 => class_glyph;
    };

    fn defaults(_: &Semantic<'_>) -> FontPickerTokens {
        FontPickerTokens {
            list_height: values::FONT_PICKER_LIST_HEIGHT,
            list_width: values::FONT_PICKER_LIST_WIDTH,
            class_column: values::FONT_PICKER_CLASS_COLUMN,
            class_glyph: values::FONT_PICKER_CLASS_GLYPH,
        }
    }

    fn path(f: FontPickerField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<FontPickerField> {
        None
    }

    fn kind(f: FontPickerField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: FontPickerField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: FontPickerField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
