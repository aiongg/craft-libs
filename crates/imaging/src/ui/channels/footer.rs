//! The footer buttons and their drops (`ChannelsFooterButton`).

use std::fmt::Debug;
use std::hash::Hash;

use craft_ui::behavior::ListDrag;
use craft_ui::composites::PanelFooter;
use craft_ui::icons::Icon;
use egui::{Id, Ui};

use super::labels::ChannelsLabels;
use super::model::{ChannelKind, ChannelRow, ChannelsEvent, targeted_row};

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
/// events in order. `list` is the id of the panel's list (`ListOutput::response.id`): only its
/// rows drop on the buttons.
///
/// - Buttons from the reading end: Delete, New, Save selection as channel, Load channel as
///   selection (so Photoshop's order from the left is Load, Save, New, Delete), named and tipped
///   by the `ChannelsLabels` footer words. Delete is disabled for a click while
///   [`targeted_row`](super::targeted_row) is the composite, a colour channel or Quick Mask, or
///   there is none.
/// - A click reports [`ChannelsEvent::Footer`] with `rows` = [`targeted_row`](super::targeted_row).
/// - Delete, New and Load are `PanelFooter::drop_button::<ListDrag<K>>`: a drop reports
///   [`ChannelsEvent::FooterDrop`] with the row the pointer grabbed (`ListDrag::grabbed`, not the
///   first of the selection). A button shows the list's drop outline, and Delete reads enabled,
///   while the grabbed row is one it accepts ([`accepts_footer_drop`]); otherwise it shows
///   nothing and reports nothing. Delete takes a dragged alpha or spot channel even while the
///   composite is targeted.
/// - Keyboard: each button is a Tab stop after the list.
pub(super) fn show_footer<K: Clone + Eq + Hash + Debug + Send + Sync + 'static>(
    ui: &mut Ui,
    rows: &[ChannelRow<K>],
    labels: &ChannelsLabels,
    list: Id,
) -> Vec<ChannelsEvent<K>> {
    // pc: crates/ui-egui/src/panels/channels_panel.rs:456-487 (the footer, Photoshop's order from the left);
    // pc: crates/ui-egui/src/panels/channels_panel.rs:528-540 `drop_action`.
    let target = targeted_row(rows);
    let delete_enabled = target.is_some_and(|row| accepts_footer_drop(ChannelsFooterButton::Delete, row.kind));
    let buttons = [
        (ChannelsFooterButton::Delete, Icon::Trash, labels.delete.as_str(), delete_enabled),
        (ChannelsFooterButton::New, Icon::Plus, labels.new.as_str(), true),
        (ChannelsFooterButton::SaveSelection, Icon::RectDashed, labels.save_selection.as_str(), true),
        (ChannelsFooterButton::LoadSelection, Icon::EllipseDashed, labels.load_selection.as_str(), true),
    ];
    let mut events = Vec::new();
    PanelFooter::show(ui, |ui| {
        for (button, icon, label, enabled) in buttons {
            let (response, dropped) = if button == ChannelsFooterButton::SaveSelection {
                (PanelFooter::button(ui, icon, label), None)
            } else {
                PanelFooter::drop_button::<ListDrag<K>>(ui, icon, label, enabled, |drag| {
                    drag.list == list && rows.iter().find(|row| row.key == drag.grabbed).is_some_and(|row| accepts_footer_drop(button, row.kind))
                })
            };
            if response.clicked() {
                events.push(ChannelsEvent::Footer { button, rows: target.map(|row| row.key.clone()).into_iter().collect() });
            }
            if let Some(drag) = dropped {
                events.push(ChannelsEvent::FooterDrop { button, rows: vec![drag.grabbed.clone()] });
            }
        }
    });
    events
}

/// Whether `button` takes a drop of a row of `kind`: Delete refuses the composite, colour
/// channels and Quick Mask (PhotoCraft does nothing for them); New and Load take every row; Save
/// takes none.
pub(super) fn accepts_footer_drop(button: ChannelsFooterButton, kind: ChannelKind) -> bool {
    // pc: crates/ui-egui/src/panels/channels_panel.rs:528-540 `drop_action` (`None` for Delete on other rows).
    match button {
        ChannelsFooterButton::Delete => !matches!(kind, ChannelKind::Composite | ChannelKind::Color | ChannelKind::QuickMask { .. }),
        ChannelsFooterButton::New | ChannelsFooterButton::LoadSelection => true,
        ChannelsFooterButton::SaveSelection => false,
    }
}
