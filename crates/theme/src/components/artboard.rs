//! The Artboard tool: the drag preview and the click slop. The same at every Size (they sit on the document). Its handles are the `handle` family's.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// The Artboard tool: the drag preview and the click slop. The same at every Size (they sit on the document). Its handles are the `handle` family's.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ArtboardTokens {
    /// A drag shorter than this many screen points draws nothing: `artboard.click_slop`.
    pub click_slop: f32,
    /// The drag preview's width: `artboard.preview_line`.
    pub preview_line: f32,
}

/// This family has no overridable field: its sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ArtboardField {}

impl Family for ArtboardTokens {
    type Field = ArtboardField;
    const FIELDS: &'static [ArtboardField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { ArtboardTokens;
        "click_slop" 0.0..=64.0 => click_slop;
        "preview_line" 0.0..=16.0 => preview_line;
    };

    fn defaults(_: &Semantic<'_>) -> ArtboardTokens {
        ArtboardTokens { click_slop: values::ARTBOARD_CLICK_SLOP, preview_line: values::ARTBOARD_PREVIEW_LINE }
    }

    fn path(f: ArtboardField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<ArtboardField> {
        None
    }

    fn kind(f: ArtboardField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: ArtboardField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: ArtboardField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
