//! The Shape tools' drag: the preview outline and the click slop. The same at every Size (they sit on the document).

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// The Shape tools' drag: the preview outline and the click slop. The same at every Size (they sit on the document).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ShapeDragTokens {
    /// A press and release closer than this many screen points is a click: `shape_drag.click_slop`.
    pub click_slop: f32,
    /// The outline of the shape being drawn: `shape_drag.preview_line`.
    pub preview_line: f32,
    /// The dark halo's extra width on each side of the preview outline: `shape_drag.preview_halo`.
    pub preview_halo: f32,
}

/// This family has no overridable field: its sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ShapeDragField {}

impl Family for ShapeDragTokens {
    type Field = ShapeDragField;
    const FIELDS: &'static [ShapeDragField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { ShapeDragTokens;
        "click_slop" 0.0..=64.0 => click_slop;
        "preview_line" 0.0..=16.0 => preview_line;
        "preview_halo" 0.0..=16.0 => preview_halo;
    };

    fn defaults(_: &Semantic<'_>) -> ShapeDragTokens {
        ShapeDragTokens {
            click_slop: values::SHAPE_DRAG_CLICK_SLOP,
            preview_line: values::SHAPE_DRAG_PREVIEW_LINE,
            preview_halo: values::SHAPE_DRAG_PREVIEW_HALO,
        }
    }

    fn path(f: ShapeDragField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<ShapeDragField> {
        None
    }

    fn kind(f: ShapeDragField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: ShapeDragField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: ShapeDragField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
