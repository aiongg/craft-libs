//! `HistoryPanelStyle` from the semantic tokens.

use super::panel::HistoryPanel;
use crate::theme::UiTokens;

/// The History panel's own sizes; every other part draws with the `list.*` and `panel_footer.*` styles.
/// Painting code reads these sizes from here and nowhere else; no body names a number for them.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct HistoryPanelStyle {
    /// The side of a snapshot thumbnail, in points; the row is this plus `list.thumb_pad`.
    pub snapshot_thumb: f32,
}

impl HistoryPanel<'_> {
    /// The panel's sizes: the snapshot thumbnail is `size.thumb.sm`.
    pub fn style(tokens: &UiTokens) -> HistoryPanelStyle {
        HistoryPanelStyle { snapshot_thumb: tokens.size.thumb.sm }
    }
}
