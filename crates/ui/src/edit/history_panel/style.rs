//! `HistoryPanelStyle` from the `history` tokens.

use super::panel::HistoryPanel;
use crate::theme::UiTokens;

/// The History panel's own sizes, from the `history` tokens (Size data an app overrides through
/// its sizes table); every other part draws with the `list.*` and `panel_footer.*` tokens.
/// Painting code reads these sizes from here and nowhere else; no body names a number for them.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct HistoryPanelStyle {
    /// The side of a snapshot thumbnail, in points; the row is this plus `list.thumb_pad`.
    pub snapshot_thumb: f32,
    /// The width of the [`UndoList`](super::UndoList) popup.
    pub undo_list_width: f32,
}

impl HistoryPanel<'_> {
    /// The panel's sizes: `history.snapshot_thumb` and `history.undo_list_width`.
    #[expect(clippy::todo, reason = "skeleton: task TK1 adds the `history` tokens and reads them here")]
    pub fn style(tokens: &UiTokens) -> HistoryPanelStyle {
        // pc: crates/ui-egui/src/panels/history.rs:284 (snapshot thumbnail); wordcraft: crates/ui-egui/src/chrome.rs:153 (`W`, the Undo list's width). Values in specs/step6a-history.md, "Proposed tokens".
        let _ = tokens;
        todo!()
    }
}
