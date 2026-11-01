//! `HistoryPanelStyle` from the `history` tokens.

use super::panel::HistoryPanel;
use crate::theme::UiTokens;

/// The History panel's own sizes; every other part draws with the `list.*` and `panel_footer.*`
/// tokens. Proposed tokens (task TK1): `history.snapshot_thumb` 28 / 28 (Compact / Comfortable),
/// `history.undo_list_width` 210 / 210, both Size data an app overrides through its sizes table.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct HistoryPanelStyle {
    /// The side of a snapshot thumbnail, in points; the row is this plus `list.thumb_pad`.
    pub snapshot_thumb: f32,
    /// The width of the [`UndoList`](super::UndoList) popup.
    pub undo_list_width: f32,
}

impl HistoryPanel<'_> {
    /// The panel's sizes. The skeleton returns PhotoCraft's and WordCraft's values; task TK1
    /// reads the `history.*` tokens instead.
    pub fn style(tokens: &UiTokens) -> HistoryPanelStyle {
        // pc: crates/ui-egui/src/panels/history.rs:284 (28 pt thumbnail); wordcraft: crates/ui-egui/src/chrome.rs:153 (`W` 210)
        let _ = tokens;
        HistoryPanelStyle { snapshot_thumb: 28.0, undo_list_width: 210.0 }
    }
}
