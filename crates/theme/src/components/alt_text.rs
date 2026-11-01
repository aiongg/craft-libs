//! Alternative text editors: the box. Sizes only: the colours are the surface, text and state tokens of the controls inside, and no theme overrides these.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// AltText tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct AltTextTokens {
    /// Width of the text box: `alt_text.box_width`.
    pub box_width: f32,
    /// The least gap between the editor's rows: `alt_text.row_gap`.
    pub row_gap: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AltTextField {}

impl Family for AltTextTokens {
    type Field = AltTextField;
    const FIELDS: &'static [AltTextField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { AltTextTokens;
        "box_width" 0.0..=4096.0 => box_width;
        "row_gap" 0.0..=256.0 => row_gap;
    };

    fn defaults(_: &Semantic<'_>) -> AltTextTokens {
        AltTextTokens { box_width: values::ALT_TEXT_BOX_WIDTH, row_gap: values::ALT_TEXT_ROW_GAP }
    }

    fn path(f: AltTextField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<AltTextField> {
        None
    }

    fn kind(f: AltTextField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: AltTextField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: AltTextField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
