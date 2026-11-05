//! The "+" buttons beside a selected object. The same at every Size (they sit on the document).

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// The "+" buttons beside a selected object. The same at every Size (they sit on the document).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct AddBesideTokens {
    /// From the object's edge to a button's centre, in screen points: `add_beside.offset`.
    pub offset: f32,
    /// A button's radius: `add_beside.radius`.
    pub radius: f32,
    /// The plus's arm as a share of the radius (a ratio, not a size): `add_beside.arm`.
    pub arm: f32,
    /// The plus's width: `add_beside.line`.
    pub line: f32,
    /// The ring's width: `add_beside.ring`.
    pub ring: f32,
}

/// This family has no overridable field: its sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AddBesideField {}

impl Family for AddBesideTokens {
    type Field = AddBesideField;
    const FIELDS: &'static [AddBesideField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { AddBesideTokens;
        "offset" 0.0..=128.0 => offset;
        "radius" 0.0..=64.0 => radius;
        "arm" 0.0..=1.0 => arm;
        "line" 0.0..=16.0 => line;
        "ring" 0.0..=16.0 => ring;
    };

    fn defaults(_: &Semantic<'_>) -> AddBesideTokens {
        AddBesideTokens {
            offset: values::ADD_BESIDE_OFFSET,
            radius: values::ADD_BESIDE_RADIUS,
            arm: values::ADD_BESIDE_ARM,
            line: values::ADD_BESIDE_LINE,
            ring: values::ADD_BESIDE_RING,
        }
    }

    fn path(f: AddBesideField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<AddBesideField> {
        None
    }

    fn kind(f: AddBesideField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: AddBesideField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: AddBesideField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
