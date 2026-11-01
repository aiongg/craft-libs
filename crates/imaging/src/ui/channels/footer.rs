//! The footer buttons and their drops (`ChannelsFooterButton`).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::fmt::Debug;
use std::hash::Hash;

use egui::Ui;

use super::labels::ChannelsLabels;
use super::model::{ChannelKind, ChannelRow, ChannelsEvent};

/// A button of the Channels footer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChannelsFooterButton {
    /// Delete current channel (`Icon::Trash`): disabled while the target is the composite, a
    /// colour channel or Quick Mask.
    Delete,
    /// Create new channel (`Icon::Plus`); a row dropped on it is duplicated.
    New,
    /// Save selection as channel (`Icon::RectDashed`); takes no drops.
    SaveSelection,
    /// Load channel as selection (`Icon::EllipseDashed`); a row dropped on it is loaded.
    LoadSelection,
}

/// Draws the footer with [`PanelFooter`](craft_ui::composites::PanelFooter) and returns its
/// events in order.
///
/// - Buttons from the reading end: Delete, New, Save selection as channel, Load channel as
///   selection (so Photoshop's order from the left is Load, Save, New, Delete), named and tipped
///   by the `ChannelsLabels` footer words. Delete is disabled while [`targeted_row`](super::targeted_row)
///   is the composite, a colour channel or Quick Mask, or there is none.
/// - A click reports [`ChannelsEvent::Footer`] with `key` = [`targeted_row`](super::targeted_row).
/// - Right after Delete, New and Load,
///   `PanelFooter::drop_target::<ListDrag<K>>(ui, &button, |drag| accepts_footer_drop(..))`; a
///   drop reports [`ChannelsEvent::FooterDrop`] with the dragged row's key. The button shows the
///   list's drop outline while it accepts and nothing when it refuses.
/// - Keyboard: each button is a Tab stop after the list.
pub(super) fn show_footer<K: Clone + Eq + Hash + Debug + Send + Sync + 'static>(
    ui: &mut Ui,
    rows: &[ChannelRow<K>],
    labels: &ChannelsLabels,
) -> Vec<ChannelsEvent<K>> {
    // pc: crates/ui-egui/src/panels/channels_panel.rs:456-487 (the footer, Photoshop's order from the left);
    // pc: crates/ui-egui/src/panels/channels_panel.rs:528-540 `drop_action`.
    todo!()
}

/// Whether `button` takes a drop of a row of `kind`: Delete refuses the composite, colour
/// channels and Quick Mask (PhotoCraft does nothing for them); New and Load take every row; Save
/// takes none.
pub(super) fn accepts_footer_drop(button: ChannelsFooterButton, kind: ChannelKind) -> bool {
    // pc: crates/ui-egui/src/panels/channels_panel.rs:528-540 `drop_action` (`None` for Delete on other rows).
    todo!()
}
