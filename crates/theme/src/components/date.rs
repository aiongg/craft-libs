//! Date pickers: the day grid and its popup. Sizes only: the colours are the surface, text and state tokens of the controls inside, and no theme overrides these.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Date tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct DateTokens {
    /// Least width and height of a day, month or year cell: `date.cell`.
    pub cell: f32,
    /// Between cells: `date.gap`.
    pub gap: f32,
    /// Width of the popup: `date.popup_width`.
    pub popup_width: f32,
    /// Side of the Previous and Next buttons: `date.page_button`.
    pub page_button: f32,
    /// Either side of the text of a month or year cell: `date.cell_pad`.
    pub cell_pad: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DateField {}

impl Family for DateTokens {
    type Field = DateField;
    const FIELDS: &'static [DateField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { DateTokens;
        "cell" 0.0..=256.0 => cell;
        "gap" 0.0..=64.0 => gap;
        "popup_width" 0.0..=1024.0 => popup_width;
        "page_button" 0.0..=256.0 => page_button;
        "cell_pad" 0.0..=64.0 => cell_pad;
    };

    fn defaults(_: &Semantic<'_>) -> DateTokens {
        DateTokens {
            cell: values::DATE_CELL,
            gap: values::DATE_GAP,
            popup_width: values::DATE_POPUP_WIDTH,
            page_button: values::DATE_PAGE_BUTTON,
            cell_pad: values::DATE_CELL_PAD,
        }
    }

    fn path(f: DateField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<DateField> {
        None
    }

    fn kind(f: DateField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: DateField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: DateField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
