//! Outlines being drawn: how near a press must be to close one. The reach is in screen points, so it is the same at every Size.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Outlines being drawn by the lasso tools: the close reach and the same-point reach.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct OutlineDraftTokens {
    /// A press this near the first point closes an outline, in screen points: `outline_draft.close`.
    pub close: f32,
    /// A press this near the last point is the same click again: `outline_draft.same_point`.
    pub same_point: f32,
}

/// Outline drafts have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum OutlineDraftField {}

impl Family for OutlineDraftTokens {
    type Field = OutlineDraftField;
    const FIELDS: &'static [OutlineDraftField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { OutlineDraftTokens;
        "close" 0.0..=64.0 => close;
        "same_point" 0.0..=64.0 => same_point;
    };

    fn defaults(_: &Semantic<'_>) -> OutlineDraftTokens {
        OutlineDraftTokens { close: values::OUTLINE_DRAFT_CLOSE, same_point: values::OUTLINE_DRAFT_SAME_POINT }
    }

    fn path(f: OutlineDraftField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<OutlineDraftField> {
        None
    }

    fn kind(f: OutlineDraftField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: OutlineDraftField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: OutlineDraftField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
