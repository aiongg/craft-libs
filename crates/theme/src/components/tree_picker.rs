//! Tree pickers: the popup's list. Sizes only: the colours are the surface, text and state tokens of the controls inside, and no theme overrides these.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// TreePicker tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TreePickerTokens {
    /// The most the list takes before it scrolls: `tree_picker.list_height`.
    pub list_height: f32,
    /// Least width of the popup: `tree_picker.min_popup_width`.
    pub min_popup_width: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TreePickerField {}

impl Family for TreePickerTokens {
    type Field = TreePickerField;
    const FIELDS: &'static [TreePickerField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { TreePickerTokens;
        "list_height" 0.0..=4096.0 => list_height;
        "min_popup_width" 0.0..=4096.0 => min_popup_width;
    };

    fn defaults(_: &Semantic<'_>) -> TreePickerTokens {
        TreePickerTokens { list_height: values::TREE_PICKER_LIST_HEIGHT, min_popup_width: values::TREE_PICKER_MIN_POPUP_WIDTH }
    }

    fn path(f: TreePickerField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<TreePickerField> {
        None
    }

    fn kind(f: TreePickerField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: TreePickerField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: TreePickerField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
