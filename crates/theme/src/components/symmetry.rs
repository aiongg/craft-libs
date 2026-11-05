//! Symmetry guides: the axis lines and the reach of the Transform Symmetry handles. The same at every Size (they sit on the document). The line is `overlay.line`.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Symmetry guides: the axis lines and the reach of the Transform Symmetry handles.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SymmetryTokens {
    /// Width of each axis line: `symmetry.line`.
    pub line: f32,
    /// A press within this many screen points of the centre or rotation handle takes it: `symmetry.hit`.
    pub hit: f32,
    /// The rotation handle's distance from the centre as a share of the document's shorter side (a ratio, not a size): `symmetry.handle_share`.
    pub handle_share: f32,
}

/// This family has no overridable field: its sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SymmetryField {}

impl Family for SymmetryTokens {
    type Field = SymmetryField;
    const FIELDS: &'static [SymmetryField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { SymmetryTokens;
        "line" 0.0..=16.0 => line;
        "hit" 0.0..=64.0 => hit;
        "handle_share" 0.0..=1.0 => handle_share;
    };

    fn defaults(_: &Semantic<'_>) -> SymmetryTokens {
        SymmetryTokens { line: values::SYMMETRY_LINE, hit: values::SYMMETRY_HIT, handle_share: values::SYMMETRY_HANDLE_SHARE }
    }

    fn path(f: SymmetryField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<SymmetryField> {
        None
    }

    fn kind(f: SymmetryField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: SymmetryField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: SymmetryField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
