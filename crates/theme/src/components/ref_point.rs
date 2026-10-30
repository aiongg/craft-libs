//! Reference point pickers: the 3 × 3 grid of a box's nine points. Their colours are semantic
//! tokens; the family holds their sizes, the same at every Size because they stand for document
//! positions.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Reference point picker tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct RefPointTokens {
    /// Side of the compact picker in an options bar, in points: `ref_point.compact`.
    pub compact: f32,
    /// Side of the large picker with growth arrows (Canvas Size): `ref_point.large`.
    pub large: f32,
    /// Side of a point's square dot: `ref_point.dot`.
    pub dot: f32,
}

/// Reference point pickers have no overridable field: their sizes come from the code, as every
/// size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RefPointField {}

impl Family for RefPointTokens {
    type Field = RefPointField;
    const FIELDS: &'static [RefPointField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { RefPointTokens;
        "compact" 0.0..=1024.0 => compact;
        "large" 0.0..=1024.0 => large;
        "dot" 0.0..=1024.0 => dot;
    };

    fn defaults(_: &Semantic<'_>) -> RefPointTokens {
        RefPointTokens { compact: values::REF_POINT_COMPACT, large: values::REF_POINT_LARGE, dot: values::REF_POINT_DOT }
    }

    fn path(f: RefPointField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<RefPointField> {
        None
    }

    fn kind(f: RefPointField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: RefPointField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: RefPointField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
