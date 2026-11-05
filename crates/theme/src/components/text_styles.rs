//! The Character Styles and Paragraph Styles panels: their list.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// TextStyles tokens, in points, the same at every Size.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TextStylesTokens {
    /// Height of the list, in points: `text_styles.list_height`.
    pub list_height: f32,
    /// Side of a style tile in the picker's grid view, in points: `text_styles.tile_side`.
    pub tile_side: f32,
    /// Side of a style's preview square in the chip and the list rows, in points: `text_styles.preview_side`.
    pub preview_side: f32,
}

/// TextStyles has no overridable field: its sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TextStylesField {}

impl Family for TextStylesTokens {
    type Field = TextStylesField;
    const FIELDS: &'static [TextStylesField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { TextStylesTokens;
        "list_height" 0.0..=2048.0 => list_height;
        "tile_side" 4.0..=256.0 => tile_side;
        "preview_side" 4.0..=256.0 => preview_side;
    };

    fn defaults(_: &Semantic<'_>) -> TextStylesTokens {
        TextStylesTokens {
            list_height: values::TEXT_STYLES_LIST_HEIGHT,
            tile_side: values::TEXT_STYLES_TILE_SIDE,
            preview_side: values::TEXT_STYLES_PREVIEW_SIDE,
        }
    }

    fn path(f: TextStylesField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<TextStylesField> {
        None
    }

    fn kind(f: TextStylesField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: TextStylesField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: TextStylesField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
