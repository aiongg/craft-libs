//! The pasteboard: the area round the pages of a canvas. Its colour is the user's choice and its
//! page edge a `line.*` colour; the family holds only the sizes, which no theme overrides.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Pasteboard tokens, in points, the same at every Size.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PasteboardTokens {
    /// Space between a page's top edge and its name: `pasteboard.name_gap`.
    pub name_gap: f32,
    /// Width of a page's outline: `pasteboard.outline_width`.
    pub outline_width: f32,
}

/// The pasteboard has no overridable field: its sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PasteboardField {}

impl Family for PasteboardTokens {
    type Field = PasteboardField;
    const FIELDS: &'static [PasteboardField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { PasteboardTokens;
        "name_gap" 0.0..=64.0 => name_gap;
        "outline_width" 0.0..=16.0 => outline_width;
    };

    fn defaults(_: &Semantic<'_>) -> PasteboardTokens {
        PasteboardTokens { name_gap: values::PASTEBOARD_NAME_GAP, outline_width: values::PASTEBOARD_OUTLINE_WIDTH }
    }

    fn path(f: PasteboardField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<PasteboardField> {
        None
    }

    fn kind(f: PasteboardField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: PasteboardField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: PasteboardField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
