//! Select and Mask's canvas marks: the Polygonal Lasso's close reach, the Object Selection click reach and the stroke being drawn. The same at every Size (they sit on the image).

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
use crate::values;

/// Select and Mask's canvas marks.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SelectMaskTokens {
    /// A click this near the Polygonal Lasso's first vertex closes it, in screen points: `select_mask.close_reach`.
    pub close_reach: f32,
    /// An Object Selection drag shorter than this many screen points is a click: `select_mask.click_reach`.
    pub click_reach: f32,
    /// The brush stroke being drawn: `select_mask.trail`, `accent` at 0.43 opacity (PhotoCraft's alpha 110).
    pub trail: Rgba,
}

/// This family has no overridable field: its sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SelectMaskField {}

impl Family for SelectMaskTokens {
    type Field = SelectMaskField;
    const FIELDS: &'static [SelectMaskField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { SelectMaskTokens;
        "close_reach" 0.0..=64.0 => close_reach;
        "click_reach" 0.0..=64.0 => click_reach;
    };

    fn defaults(s: &Semantic<'_>) -> SelectMaskTokens {
        SelectMaskTokens {
            close_reach: values::SELECT_MASK_CLOSE_REACH,
            click_reach: values::SELECT_MASK_CLICK_REACH,
            trail: s.color.accent.solid.faded(values::SELECT_MASK_TRAIL_OPACITY),
        }
    }

    fn path(f: SelectMaskField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<SelectMaskField> {
        None
    }

    fn kind(f: SelectMaskField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: SelectMaskField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: SelectMaskField, _: OverrideValue) {
        match f {}
    }

    // The colour is a translucent accent over the image, read as it is.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
