//! The History panel: the snapshot thumbnail and the Undo list's width. Rows, fills and the footer
//! are the list and panel footer families'.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// History tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct HistoryTokens {
    /// Side of a snapshot's thumbnail: `values::HISTORY_SNAPSHOT_THUMB`.
    pub snapshot_thumb: f32,
    /// Width of the Undo list's popup: `values::HISTORY_UNDO_LIST_WIDTH`.
    pub undo_list_width: f32,
}

/// History has no overridable field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HistoryField {}

impl Family for HistoryTokens {
    type Field = HistoryField;
    const FIELDS: &'static [HistoryField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { HistoryTokens;
        "snapshot_thumb" 12.0..=96.0 => snapshot_thumb;
        "undo_list_width" 120.0..=480.0 => undo_list_width;
    };

    fn defaults(_: &Semantic<'_>) -> HistoryTokens {
        HistoryTokens { snapshot_thumb: values::HISTORY_SNAPSHOT_THUMB, undo_list_width: values::HISTORY_UNDO_LIST_WIDTH }
    }

    fn path(f: HistoryField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<HistoryField> {
        None
    }

    fn kind(f: HistoryField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: HistoryField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: HistoryField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
