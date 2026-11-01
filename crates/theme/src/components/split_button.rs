//! Split buttons: a push button with an arrow part that opens a menu. Sizes only: the colours are the surface, text and state tokens of the controls inside, and no theme overrides these.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// SplitButton tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SplitButtonTokens {
    /// Width of the arrow part: `split_button.arrow_width`.
    pub arrow_width: f32,
    /// Between the divider line's ends and the button's top and bottom: `split_button.divider_inset`.
    pub divider_inset: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SplitButtonField {}

impl Family for SplitButtonTokens {
    type Field = SplitButtonField;
    const FIELDS: &'static [SplitButtonField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { SplitButtonTokens;
        "arrow_width" 0.0..=256.0 => arrow_width;
        "divider_inset" 0.0..=64.0 => divider_inset;
    };

    fn defaults(_: &Semantic<'_>) -> SplitButtonTokens {
        SplitButtonTokens { arrow_width: values::SPLIT_BUTTON_ARROW_WIDTH, divider_inset: values::SPLIT_BUTTON_DIVIDER_INSET }
    }

    fn path(f: SplitButtonField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<SplitButtonField> {
        None
    }

    fn kind(f: SplitButtonField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: SplitButtonField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: SplitButtonField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
