//! Tool pills: the floating row of tools over a canvas. The family holds sizes only; the colours are semantic tokens.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// ToolPill tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ToolPillTokens {
    /// Kept between the pill and the canvas edge, clear of the canvas scroll bars: `tool_pill.inset`.
    pub inset: f32,
}

/// ToolPill tokens have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ToolPillField {}

impl Family for ToolPillTokens {
    type Field = ToolPillField;
    const FIELDS: &'static [ToolPillField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { ToolPillTokens;
        "inset" 0.0..=1024.0 => inset;
    };

    fn defaults(s: &Semantic<'_>) -> ToolPillTokens {
        ToolPillTokens {
            inset: values::SCROLL_CANVAS_WIDTH + s.space.pad.panel,
        }
    }

    fn path(f: ToolPillField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<ToolPillField> {
        None
    }

    fn kind(f: ToolPillField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: ToolPillField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: ToolPillField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
