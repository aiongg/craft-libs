//! The column bar over a table: its outline, dividers and their grab zone. No theme overrides it.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// ColumnBar tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ColumnBarTokens {
    /// The bar's outline: [`values::COLUMN_BAR_OUTLINE`].
    pub outline: f32,
    /// A divider at rest: [`values::COLUMN_BAR_DIVIDER`].
    pub divider: f32,
    /// A divider hovered, dragged or focused: [`values::COLUMN_BAR_DIVIDER_ACTIVE`].
    pub divider_active: f32,
    /// How far either side of a divider grabs it: [`values::COLUMN_BAR_GRAB`].
    pub grab: f32,
}

/// ColumnBar tokens have no overridable field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ColumnBarField {}

impl Family for ColumnBarTokens {
    type Field = ColumnBarField;
    const FIELDS: &'static [ColumnBarField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { ColumnBarTokens;
        "outline" 0.0..=8.0 => outline;
        "divider" 0.0..=8.0 => divider;
        "divider_active" 0.0..=16.0 => divider_active;
        "grab" 0.0..=32.0 => grab;
    };

    fn defaults(_: &Semantic<'_>) -> ColumnBarTokens {
        ColumnBarTokens {
            outline: values::COLUMN_BAR_OUTLINE,
            divider: values::COLUMN_BAR_DIVIDER,
            divider_active: values::COLUMN_BAR_DIVIDER_ACTIVE,
            grab: values::COLUMN_BAR_GRAB,
        }
    }

    fn path(f: ColumnBarField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<ColumnBarField> {
        None
    }

    fn kind(f: ColumnBarField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: ColumnBarField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: ColumnBarField, _: OverrideValue) {
        match f {}
    }

    // Marks and tints drawn over surfaces other families check.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
