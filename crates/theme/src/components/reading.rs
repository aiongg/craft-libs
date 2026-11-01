//! The reading view: columns, the page-turn bar and the column widths. No theme overrides it.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Reading tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ReadingTokens {
    /// Between columns: [`values::READING_COLUMN_GAP`].
    pub column_gap: f32,
    /// Room under the columns for the page-turn bar: [`values::READING_NAV_BAR`].
    pub nav_bar: f32,
    /// One column per this much width: [`values::READING_COLUMN_TARGET`].
    pub column_target: f32,
    /// The least column width: [`values::READING_COLUMN_MIN`].
    pub column_min: f32,
}

/// Reading tokens have no overridable field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ReadingField {}

impl Family for ReadingTokens {
    type Field = ReadingField;
    const FIELDS: &'static [ReadingField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { ReadingTokens;
        "column_gap" 0.0..=128.0 => column_gap;
        "nav_bar" 0.0..=256.0 => nav_bar;
        "column_target" 64.0..=2048.0 => column_target;
        "column_min" 32.0..=2048.0 => column_min;
    };

    fn defaults(_: &Semantic<'_>) -> ReadingTokens {
        ReadingTokens {
            column_gap: values::READING_COLUMN_GAP,
            nav_bar: values::READING_NAV_BAR,
            column_target: values::READING_COLUMN_TARGET,
            column_min: values::READING_COLUMN_MIN,
        }
    }

    fn path(f: ReadingField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<ReadingField> {
        None
    }

    fn kind(f: ReadingField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: ReadingField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: ReadingField, _: OverrideValue) {
        match f {}
    }

    // Marks and tints drawn over surfaces other families check.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
