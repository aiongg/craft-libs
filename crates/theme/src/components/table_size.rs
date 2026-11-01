//! The table size picker: a square and the gap between squares. No theme overrides it.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// TableSize tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TableSizeTokens {
    /// A square's side: [`values::TABLE_SIZE_CELL`].
    pub cell: f32,
    /// The space between squares: [`values::TABLE_SIZE_GAP`].
    pub gap: f32,
}

/// TableSize tokens have no overridable field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TableSizeField {}

impl Family for TableSizeTokens {
    type Field = TableSizeField;
    const FIELDS: &'static [TableSizeField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { TableSizeTokens;
        "cell" 4.0..=64.0 => cell;
        "gap" 0.0..=16.0 => gap;
    };

    fn defaults(_: &Semantic<'_>) -> TableSizeTokens {
        TableSizeTokens { cell: values::TABLE_SIZE_CELL, gap: values::TABLE_SIZE_GAP }
    }

    fn path(f: TableSizeField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<TableSizeField> {
        None
    }

    fn kind(f: TableSizeField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: TableSizeField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: TableSizeField, _: OverrideValue) {
        match f {}
    }

    // Marks and tints drawn over surfaces other families check.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
