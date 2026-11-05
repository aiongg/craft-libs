//! Hover outlines: the outline and mask tint of the object under the pointer, and the lookup's timing. The same at every Size (the outline sits on the document). The colour is the semantic `overlay.line`.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Hover outlines: the object under the pointer.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct HoverTokens {
    /// Line width of a rect or path outline: `hover.outline`.
    pub outline: f32,
    /// A mask texel's alpha per unit of coverage: `hover.fill_alpha`.
    pub fill_alpha: f32,
    /// A mask edge texel's alpha: `hover.edge_alpha`.
    pub edge_alpha: f32,
    /// How long the pointer rests before a lookup, in milliseconds: `hover.debounce_ms`.
    pub debounce_ms: f32,
    /// Repaint interval while a lookup runs, in milliseconds: `hover.busy_ms`.
    pub busy_ms: f32,
}

/// Hover outlines have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HoverField {}

impl Family for HoverTokens {
    type Field = HoverField;
    const FIELDS: &'static [HoverField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { HoverTokens;
        "outline" 0.0..=16.0 => outline;
        "fill_alpha" 0.0..=1.0 => fill_alpha;
        "edge_alpha" 0.0..=1.0 => edge_alpha;
        "debounce_ms" 0.0..=5000.0 => debounce_ms;
        "busy_ms" 0.0..=5000.0 => busy_ms;
    };

    fn defaults(_: &Semantic<'_>) -> HoverTokens {
        HoverTokens {
            outline: values::HOVER_OUTLINE,
            fill_alpha: values::HOVER_FILL_ALPHA,
            edge_alpha: values::HOVER_EDGE_ALPHA,
            debounce_ms: values::HOVER_DEBOUNCE_MS,
            busy_ms: values::HOVER_BUSY_MS,
        }
    }

    fn path(f: HoverField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<HoverField> {
        None
    }

    fn kind(f: HoverField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: HoverField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: HoverField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
