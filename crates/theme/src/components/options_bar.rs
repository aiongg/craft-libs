//! Options bars: the bar of the active tool's options under the title bar. Sizes only (no colours):
//! the bar draws in `surface.chrome`, its captions in `text.muted` and its hints in `text.faint`.
//! The heights are the `size.bar` ones.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Options bar tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct OptionsBarTokens {
    /// Height of the bar: `options_bar.height` (`size.bar.options`).
    pub height: f32,
    /// Height of the narrow bar: `options_bar.height_narrow` (`size.bar.options_narrow`).
    pub height_narrow: f32,
    /// Space at the bar's ends: `options_bar.pad`.
    pub pad: f32,
    /// Between two rows of a bar with more than one row: `options_bar.row_gap`.
    pub row_gap: f32,
    /// Between items, and between a caption and its control: `options_bar.gap`.
    pub gap: f32,
    /// Length of a divider between groups; its box is 9 (Compact) wide: `options_bar.divider`.
    pub divider: f32,
    /// Width of the narrow field class: `options_bar.field_sm`.
    pub field_sm: f32,
    /// Width of the medium field class: `options_bar.field_md`.
    pub field_md: f32,
    /// Width of the wide field class: `options_bar.field_lg`.
    pub field_lg: f32,
    /// Width of the floating scroll bar: `options_bar.scroll_bar`.
    pub scroll_bar: f32,
    /// Side of the Home, tool icon, commit and cancel buttons: `options_bar.button`.
    pub button: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum OptionsBarField {}

impl Family for OptionsBarTokens {
    type Field = OptionsBarField;
    const FIELDS: &'static [OptionsBarField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { OptionsBarTokens;
        "height" 0.0..=1024.0 => height;
        "height_narrow" 0.0..=1024.0 => height_narrow;
        "pad" 0.0..=64.0 => pad;
        "row_gap" 0.0..=64.0 => row_gap;
        "gap" 0.0..=64.0 => gap;
        "divider" 0.0..=1024.0 => divider;
        "field_sm" 0.0..=1024.0 => field_sm;
        "field_md" 0.0..=1024.0 => field_md;
        "field_lg" 0.0..=1024.0 => field_lg;
        "scroll_bar" 0.0..=1024.0 => scroll_bar;
        "button" 0.0..=1024.0 => button;
    };

    /// The Compact sizes (PhotoCraft's Pro options bar), scaled by the control height of the size
    /// and rounded to whole points (halves to even). The ends and the scroll bar keep their width.
    fn defaults(s: &Semantic<'_>) -> OptionsBarTokens {
        let k = s.size.control.md / values::OPTIONS_BAR_BASE_CONTROL;
        let scale = |w: f32| (w * k).round_ties_even();
        OptionsBarTokens {
            height: s.size.bar.options,
            height_narrow: s.size.bar.options_narrow,
            pad: values::OPTIONS_BAR_PAD,
            row_gap: scale(values::OPTIONS_BAR_ROW_GAP),
            gap: scale(values::OPTIONS_BAR_GAP),
            divider: scale(values::OPTIONS_BAR_DIVIDER),
            field_sm: scale(values::OPTIONS_BAR_FIELD_SM),
            field_md: scale(values::OPTIONS_BAR_FIELD_MD),
            field_lg: scale(values::OPTIONS_BAR_FIELD_LG),
            scroll_bar: values::OPTIONS_BAR_SCROLL_BAR,
            button: s.set.options_bar_button,
        }
    }

    fn path(f: OptionsBarField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<OptionsBarField> {
        None
    }

    fn kind(f: OptionsBarField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: OptionsBarField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: OptionsBarField, _: OverrideValue) {
        match f {}
    }

    // Captions and hints are semantic text colours, which the text rules hold on `surface.chrome`.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
