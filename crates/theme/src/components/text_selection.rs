//! Text selection on a page: the selection tint, its corner radius and the column rectangle's outline. No theme overrides it.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
use crate::values;

/// TextSelection tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TextSelectionTokens {
    /// Corner radius of the selection rects: [`values::TEXT_SELECTION_RADIUS`].
    pub radius: f32,
    /// The column rectangle's outline while dragging: [`values::TEXT_SELECTION_COLUMN_OUTLINE`].
    pub column_outline: f32,
    /// The selection tint: `text_selection.fill`, `accent.solid` at alpha [`values::TEXT_SELECTION_FILL_ALPHA`].
    pub fill: Rgba,
}

/// TextSelection tokens have no overridable field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TextSelectionField {}

impl Family for TextSelectionTokens {
    type Field = TextSelectionField;
    const FIELDS: &'static [TextSelectionField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { TextSelectionTokens;
        "radius" 0.0..=16.0 => radius;
        "column_outline" 0.0..=8.0 => column_outline;
    };

    fn defaults(s: &Semantic<'_>) -> TextSelectionTokens {
        let c = s.color;
        TextSelectionTokens {
            radius: values::TEXT_SELECTION_RADIUS,
            column_outline: values::TEXT_SELECTION_COLUMN_OUTLINE,
            fill: c.accent.solid.with_alpha(values::TEXT_SELECTION_FILL_ALPHA),
        }
    }

    fn path(f: TextSelectionField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<TextSelectionField> {
        None
    }

    fn kind(f: TextSelectionField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: TextSelectionField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: TextSelectionField, _: OverrideValue) {
        match f {}
    }

    // Marks and tints drawn over surfaces other families check.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
