//! Canvas handles: the squares and dots on selections, paths and transforms. Their colours are
//! the semantic `overlay.*` tokens; the family holds only their geometry, which no theme
//! overrides.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::values;

/// Handle tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct HandleTokens {
    /// Side of a square handle, diameter of a round one, in points.
    pub size: f32,
    /// Outline width, drawn inside `size`.
    pub stroke: f32,
    /// Halo width, drawn outside `size`.
    pub halo: f32,
    /// Side of the pointer target, centred on the handle.
    pub hit: f32,
}

/// Handles have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HandleField {}

impl Family for HandleTokens {
    type Field = HandleField;
    const FIELDS: &'static [HandleField] = &[];

    fn defaults(_: &Semantic<'_>) -> HandleTokens {
        HandleTokens { size: values::HANDLE_SIZE, stroke: values::HANDLE_STROKE, halo: values::HANDLE_HALO, hit: values::HANDLE_HIT }
    }

    fn path(f: HandleField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<HandleField> {
        None
    }

    fn kind(f: HandleField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: HandleField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: HandleField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: handles draw in the semantic `overlay.*` tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
