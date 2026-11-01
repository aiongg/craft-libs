//! The selected table cells: the tint and the range outline. No theme overrides it.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
use crate::values;

/// CellSelection tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct CellSelectionTokens {
    /// The range outline: [`values::CELL_SELECTION_OUTLINE`].
    pub outline: f32,
    /// The selected cells' tint: `cell_selection.fill`, `accent.solid` at alpha [`values::CELL_SELECTION_FILL_ALPHA`].
    pub fill: Rgba,
}

/// CellSelection tokens have no overridable field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CellSelectionField {}

impl Family for CellSelectionTokens {
    type Field = CellSelectionField;
    const FIELDS: &'static [CellSelectionField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { CellSelectionTokens;
        "outline" 0.0..=8.0 => outline;
    };

    fn defaults(s: &Semantic<'_>) -> CellSelectionTokens {
        let c = s.color;
        CellSelectionTokens { outline: values::CELL_SELECTION_OUTLINE, fill: c.accent.solid.with_alpha(values::CELL_SELECTION_FILL_ALPHA) }
    }

    fn path(f: CellSelectionField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<CellSelectionField> {
        None
    }

    fn kind(f: CellSelectionField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: CellSelectionField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: CellSelectionField, _: OverrideValue) {
        match f {}
    }

    // Marks and tints drawn over surfaces other families check.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
