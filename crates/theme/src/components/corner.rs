//! Corner widgets: the round marks at a rectangle's corners and their reach. The same at every Size (they sit on the document). Their colours are the semantic `overlay.handle_fill` and `overlay.line`.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Corner widgets: the round marks that set a rectangle's corner radius.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct CornerTokens {
    /// The widget's diameter: `corner.widget`.
    pub widget: f32,
    /// The inner dot's diameter: `corner.dot`.
    pub dot: f32,
    /// The outline's width: `corner.line`.
    pub line: f32,
    /// The pointer's reach: `corner.hit`.
    pub hit: f32,
    /// The least distance inside a corner: `corner.inset`.
    pub inset: f32,
    /// The shortest side, on the screen, whose corners show widgets: `corner.min_side`.
    pub min_side: f32,
}

/// This family has no overridable field: its sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CornerField {}

impl Family for CornerTokens {
    type Field = CornerField;
    const FIELDS: &'static [CornerField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { CornerTokens;
        "widget" 0.0..=512.0 => widget;
        "dot" 0.0..=512.0 => dot;
        "line" 0.0..=16.0 => line;
        "hit" 0.0..=64.0 => hit;
        "inset" 0.0..=512.0 => inset;
        "min_side" 0.0..=4096.0 => min_side;
    };

    fn defaults(_: &Semantic<'_>) -> CornerTokens {
        CornerTokens {
            widget: values::CORNER_WIDGET,
            dot: values::CORNER_DOT,
            line: values::CORNER_LINE,
            hit: values::CORNER_HIT,
            inset: values::CORNER_INSET,
            min_side: values::CORNER_MIN_SIDE,
        }
    }

    fn path(f: CornerField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<CornerField> {
        None
    }

    fn kind(f: CornerField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: CornerField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: CornerField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
