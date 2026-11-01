//! List managers: the list box, the buttons and the gap between them. Sizes only: the colours are the surface, text and state tokens of the controls inside, and no theme overrides these.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// ListManager tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ListManagerTokens {
    /// Size of the list box: `list_manager.list`.
    pub list: [f32; 2],
    /// Least width of a button in the column beside the list: `list_manager.button_width`.
    pub button_width: f32,
    /// Between the list and the button column, and before a row's detail text: `list_manager.column_gap`.
    pub column_gap: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ListManagerField {}

impl Family for ListManagerTokens {
    type Field = ListManagerField;
    const FIELDS: &'static [ListManagerField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { ListManagerTokens;
        "list.0" 0.0..=4096.0 => list[0];
        "list.1" 0.0..=4096.0 => list[1];
        "button_width" 0.0..=1024.0 => button_width;
        "column_gap" 0.0..=256.0 => column_gap;
    };

    fn defaults(_: &Semantic<'_>) -> ListManagerTokens {
        ListManagerTokens {
            list: values::LIST_MANAGER_LIST,
            button_width: values::LIST_MANAGER_BUTTON_WIDTH,
            column_gap: values::LIST_MANAGER_COLUMN_GAP,
        }
    }

    fn path(f: ListManagerField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<ListManagerField> {
        None
    }

    fn kind(f: ListManagerField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: ListManagerField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: ListManagerField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
