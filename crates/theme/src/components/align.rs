//! Align bars: the icon buttons, which follow the icon button's sizes. Sizes only: the colours are the surface, text and state tokens of the controls inside, and no theme overrides these.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Align tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct AlignTokens {
    /// Side of an icon button: `align.button`; the control height, as the options bar's buttons.
    pub button: f32,
    /// Side of the icon in a button: `align.icon`; the medium icon size.
    pub icon: f32,
    /// Between the align buttons and the ⋯ button: `align.menu_gap`; scales with the control
    /// height like the options bar's gaps.
    pub menu_gap: f32,
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
        "menu_gap" 0.0..=64.0 => menu_gap;
    };

    fn defaults(s: &Semantic<'_>) -> AlignTokens {
        let k = s.size.control.md / values::OPTIONS_BAR_BASE_CONTROL;
        AlignTokens { button: s.size.control.md, icon: s.size.icon.md, menu_gap: (values::ALIGN_MENU_GAP * k).round_ties_even() }
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
