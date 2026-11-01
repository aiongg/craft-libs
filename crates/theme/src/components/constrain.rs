//! Constrained moves: how near a press of a modifier key counts and how far a stroke must go before its direction locks. The same at every Size (screen points on the pointer).

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Constrained moves: the copy reach and the stroke lock.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ConstrainTokens {
    /// Screen points within which a press copies instead of moves: `constrain.copy_reach`.
    pub copy_reach: f32,
    /// Screen points a stroke travels before its direction locks: `constrain.stroke_lock`.
    pub stroke_lock: f32,
}

/// Constrained moves have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ConstrainField {}

impl Family for ConstrainTokens {
    type Field = ConstrainField;
    const FIELDS: &'static [ConstrainField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { ConstrainTokens;
        "copy_reach" 0.0..=64.0 => copy_reach;
        "stroke_lock" 0.0..=64.0 => stroke_lock;
    };

    fn defaults(_: &Semantic<'_>) -> ConstrainTokens {
        ConstrainTokens { copy_reach: values::CONSTRAIN_COPY_REACH, stroke_lock: values::CONSTRAIN_STROKE_LOCK }
    }

    fn path(f: ConstrainField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<ConstrainField> {
        None
    }

    fn kind(f: ConstrainField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: ConstrainField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: ConstrainField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
