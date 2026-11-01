//! Align bars: the icon buttons. Sizes only: the colours are the surface, text and state tokens of the controls inside, and no theme overrides these.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Align tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct AlignTokens {
    /// Side of an icon button: `align.button`.
    pub button: f32,
    /// Side of the icon in a button: `align.icon`.
    pub icon: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AlignField {}

impl Family for AlignTokens {
    type Field = AlignField;
    const FIELDS: &'static [AlignField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { AlignTokens;
        "button" 0.0..=256.0 => button;
        "icon" 0.0..=256.0 => icon;
    };

    fn defaults(_: &Semantic<'_>) -> AlignTokens {
        AlignTokens { button: values::ALIGN_BUTTON, icon: values::ALIGN_ICON }
    }

    fn path(f: AlignField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<AlignField> {
        None
    }

    fn kind(f: AlignField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: AlignField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: AlignField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
