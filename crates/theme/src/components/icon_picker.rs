//! Icon pickers: a grid of square cells. Sizes only: the colours are the surface, text and state tokens of the controls inside, and no theme overrides these.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// IconPicker tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct IconPickerTokens {
    /// Side of a cell: `icon_picker.cell`.
    pub cell: f32,
    /// Between cells: `icon_picker.gap`.
    pub gap: f32,
    /// Between a cell's edge and its icon: `icon_picker.inset`.
    pub inset: f32,
    /// Most the grid takes before it scrolls: `icon_picker.max_height`.
    pub max_height: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum IconPickerField {}

impl Family for IconPickerTokens {
    type Field = IconPickerField;
    const FIELDS: &'static [IconPickerField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { IconPickerTokens;
        "cell" 0.0..=256.0 => cell;
        "gap" 0.0..=64.0 => gap;
        "inset" 0.0..=64.0 => inset;
        "max_height" 0.0..=4096.0 => max_height;
    };

    fn defaults(_: &Semantic<'_>) -> IconPickerTokens {
        IconPickerTokens {
            cell: values::ICON_PICKER_CELL,
            gap: values::ICON_PICKER_GAP,
            inset: values::ICON_PICKER_INSET,
            max_height: values::ICON_PICKER_MAX_HEIGHT,
        }
    }

    fn path(f: IconPickerField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<IconPickerField> {
        None
    }

    fn kind(f: IconPickerField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: IconPickerField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: IconPickerField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
