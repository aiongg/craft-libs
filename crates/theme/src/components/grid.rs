//! Canvas grid: major and minor lines over a document. Colours are the semantic `overlay.*` tokens; the family holds sizes and
//! shares, the same at every Size because the overlay sits on the document.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::values;

/// Grid tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct GridTokens {
    /// Line width: `grid.width`.
    pub width: f32,
    /// Minor lines' share of the colour: `grid.minor_alpha`.
    pub minor_alpha: f32,
    /// Smallest step in screen points at which an axis draws: `grid.min_step`.
    pub min_step: f32,
    /// Side of a Crossings dot: `grid.crossing`.
    pub crossing: f32,
    /// Smallest major spacing at which Crossings draw: `grid.crossing_min`.
    pub crossing_min: f32,
}

/// Grid has no overridable field: its sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GridField {}

impl Family for GridTokens {
    type Field = GridField;
    const FIELDS: &'static [GridField] = &[];

    fn defaults(_: &Semantic<'_>) -> GridTokens {
        GridTokens {
            width: values::GRID_WIDTH,
            minor_alpha: values::GRID_MINOR_ALPHA,
            min_step: values::GRID_MIN_STEP,
            crossing: values::GRID_CROSSING,
            crossing_min: values::GRID_CROSSING_MIN,
        }
    }

    fn path(f: GridField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<GridField> {
        None
    }

    fn kind(f: GridField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: GridField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: GridField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
