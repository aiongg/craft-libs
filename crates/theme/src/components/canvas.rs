//! The canvas: the Rotate View compass. The scroll bars, rulers, grid and guides have their own
//! families.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Canvas tokens, in points, the same at every Size because they sit over a document.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct CanvasTokens {
    /// Side of the Rotate View compass: `canvas.compass`.
    pub compass: f32,
    /// Distance of the compass from the canvas's top and right edges: `canvas.compass_margin`.
    pub compass_margin: f32,
}

/// The canvas has no overridable field: its sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CanvasField {}

impl Family for CanvasTokens {
    type Field = CanvasField;
    const FIELDS: &'static [CanvasField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { CanvasTokens;
        "compass" 0.0..=1024.0 => compass;
        "compass_margin" 0.0..=1024.0 => compass_margin;
    };

    fn defaults(_: &Semantic<'_>) -> CanvasTokens {
        CanvasTokens { compass: values::CANVAS_COMPASS, compass_margin: values::CANVAS_COMPASS_MARGIN }
    }

    fn path(f: CanvasField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<CanvasField> {
        None
    }

    fn kind(f: CanvasField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: CanvasField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: CanvasField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: the compass draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
