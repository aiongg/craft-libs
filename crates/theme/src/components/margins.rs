//! The margin handles on the rulers. No theme overrides it.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Margins tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct MarginsTokens {
    /// How far either side of an edge grabs it: [`values::MARGINS_EDGE_GRAB`].
    pub edge_grab: f32,
}

/// Margins tokens have no overridable field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MarginsField {}

impl Family for MarginsTokens {
    type Field = MarginsField;
    const FIELDS: &'static [MarginsField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { MarginsTokens;
        "edge_grab" 0.0..=32.0 => edge_grab;
    };

    fn defaults(_: &Semantic<'_>) -> MarginsTokens {
        MarginsTokens { edge_grab: values::MARGINS_EDGE_GRAB }
    }

    fn path(f: MarginsField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<MarginsField> {
        None
    }

    fn kind(f: MarginsField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: MarginsField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: MarginsField, _: OverrideValue) {
        match f {}
    }

    // Marks and tints drawn over surfaces other families check.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
