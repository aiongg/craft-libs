//! Metadata tables: name and value columns of a read-only list. Sizes only: the colours are the surface, text and state tokens of the controls inside, and no theme overrides these.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Metadata tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct MetadataTokens {
    /// Between the name and value columns: `metadata.column_gap`.
    pub column_gap: f32,
    /// Between rows: `metadata.row_gap`.
    pub row_gap: f32,
    /// Above a group's title, except the first: `metadata.group_gap`.
    pub group_gap: f32,
    /// The most the rows take before they scroll when the host gives no height: `metadata.max_height`.
    pub max_height: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MetadataField {}

impl Family for MetadataTokens {
    type Field = MetadataField;
    const FIELDS: &'static [MetadataField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { MetadataTokens;
        "column_gap" 0.0..=64.0 => column_gap;
        "row_gap" 0.0..=64.0 => row_gap;
        "group_gap" 0.0..=64.0 => group_gap;
        "max_height" 0.0..=4096.0 => max_height;
    };

    fn defaults(_: &Semantic<'_>) -> MetadataTokens {
        MetadataTokens {
            column_gap: values::METADATA_COLUMN_GAP,
            row_gap: values::METADATA_ROW_GAP,
            group_gap: values::METADATA_GROUP_GAP,
            max_height: values::METADATA_MAX_HEIGHT,
        }
    }

    fn path(f: MetadataField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<MetadataField> {
        None
    }

    fn kind(f: MetadataField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: MetadataField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: MetadataField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
