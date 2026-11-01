//! Split views: the canvas area divided into two or four panes.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Split view tokens, in points, the same at every Size.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SplitTokens {
    /// Space between panes: `split.gap`.
    pub gap: f32,
    /// Width of the line round the active pane: `split.active_edge`.
    pub active_edge: f32,
}

/// Split views have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SplitField {}

impl Family for SplitTokens {
    type Field = SplitField;
    const FIELDS: &'static [SplitField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { SplitTokens;
        "gap" 0.0..=64.0 => gap;
        "active_edge" 0.0..=8.0 => active_edge;
    };

    fn defaults(_: &Semantic<'_>) -> SplitTokens {
        SplitTokens { gap: values::SPLIT_GAP, active_edge: values::SPLIT_ACTIVE_EDGE }
    }

    fn path(f: SplitField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<SplitField> {
        None
    }

    fn kind(f: SplitField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: SplitField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: SplitField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: the active edge is `line.focus`.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
