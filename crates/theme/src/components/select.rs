//! Selection tools: how near a press closes an outline, and the Magnetic Lasso's fastening point and closing ring. The marks sit on the document, so they are the same at every Size. Their colours are the semantic `overlay.handle_fill`, `overlay.handle_stroke` and `overlay.halo`.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Selection tools: the lasso close reach, the Magnetic Lasso fastening point and closing ring.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SelectTokens {
    /// A press this near the first point closes an outline, in screen points: `select.close_radius`.
    pub close_radius: f32,
    /// A press this near the last point is the same click again: `select.same_point`.
    pub same_point: f32,
    /// Side of a Magnetic Lasso fastening point: `select.fastening`.
    pub fastening: f32,
    /// Radius of the Magnetic Lasso closing ring: `select.ring_radius`.
    pub ring_radius: f32,
    /// Offset of the ring right of and below the pointer: `select.ring_offset`.
    pub ring_offset: f32,
    /// Width of the ring halo: `select.ring_halo`.
    pub ring_halo: f32,
    /// Width of the ring line: `select.ring_line`.
    pub ring_line: f32,
}

/// Selection tools have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SelectField {}

impl Family for SelectTokens {
    type Field = SelectField;
    const FIELDS: &'static [SelectField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { SelectTokens;
        "close_radius" 0.0..=64.0 => close_radius;
        "same_point" 0.0..=64.0 => same_point;
        "fastening" 0.0..=64.0 => fastening;
        "ring_radius" 0.0..=64.0 => ring_radius;
        "ring_offset" 0.0..=64.0 => ring_offset;
        "ring_halo" 0.0..=16.0 => ring_halo;
        "ring_line" 0.0..=16.0 => ring_line;
    };

    fn defaults(_: &Semantic<'_>) -> SelectTokens {
        SelectTokens {
            close_radius: values::SELECT_CLOSE_RADIUS,
            same_point: values::SELECT_SAME_POINT,
            fastening: values::SELECT_FASTENING,
            ring_radius: values::SELECT_RING_RADIUS,
            ring_offset: values::SELECT_RING_OFFSET,
            ring_halo: values::SELECT_RING_HALO,
            ring_line: values::SELECT_RING_LINE,
        }
    }

    fn path(f: SelectField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<SelectField> {
        None
    }

    fn kind(f: SelectField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: SelectField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: SelectField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
