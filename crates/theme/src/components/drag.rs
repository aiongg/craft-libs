//! Drag badges: the pill that follows the pointer during a drag. Sizes only: the colours are the surface, text and state tokens of the controls inside, and no theme overrides these.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Drag tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct DragTokens {
    /// Horizontal distance of the badge from the pointer: `drag.badge_offset_x`.
    pub badge_offset_x: f32,
    /// Vertical distance of the badge from the pointer: `drag.badge_offset_y`.
    pub badge_offset_y: f32,
    /// Padding beside the badge's text: `drag.badge_pad_x`.
    pub badge_pad_x: f32,
    /// Padding above and below the badge's text: `drag.badge_pad_y`.
    pub badge_pad_y: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DragField {}

impl Family for DragTokens {
    type Field = DragField;
    const FIELDS: &'static [DragField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { DragTokens;
        "badge_offset_x" 0.0..=256.0 => badge_offset_x;
        "badge_offset_y" 0.0..=256.0 => badge_offset_y;
        "badge_pad_x" 0.0..=64.0 => badge_pad_x;
        "badge_pad_y" 0.0..=64.0 => badge_pad_y;
    };

    fn defaults(_: &Semantic<'_>) -> DragTokens {
        DragTokens {
            badge_offset_x: values::DRAG_BADGE_OFFSET_X,
            badge_offset_y: values::DRAG_BADGE_OFFSET_Y,
            badge_pad_x: values::DRAG_BADGE_PAD_X,
            badge_pad_y: values::DRAG_BADGE_PAD_Y,
        }
    }

    fn path(f: DragField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<DragField> {
        None
    }

    fn kind(f: DragField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: DragField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: DragField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
