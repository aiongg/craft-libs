//! The row and panel menus (`ChannelsCommand`, `ChannelsPanel::panel_menu`,
//! `ChannelsPanel::run_command`).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::fmt::Debug;
use std::hash::Hash;

use craft_ui::docking::panel_menu::PanelCommand;
use craft_ui::menu::{MenuEntry, OpenAt};
use egui::{AsIdSalt, Context, Response, Ui};

use super::labels::ChannelsLabels;
use super::model::{ChannelRow, ChannelsEvent, OverlayPreset};
use super::panel::ChannelsPanel;

/// A command of a channel's context menu or the panel menu. Channel Options… (and Quick Mask
/// Options…, Layer Mask Display Options…) report [`ChannelsEvent::Options`]; Rename Channel…
/// opens the row's rename field in the panel; neither is a command.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ChannelsCommand {
    /// New Channel….
    NewChannel,
    /// New Spot Channel….
    NewSpotChannel,
    /// Duplicate Channel.
    Duplicate,
    /// Delete Channel.
    Delete,
    /// Merge Spot Channel (spot rows).
    MergeSpot,
    /// Convert to Alpha Channel (spot rows).
    ConvertToAlpha,
    /// Exit Quick Mask.
    ExitQuickMask,
    /// Color Indicates Masked Areas (a radio item).
    ColorIndicatesMasked,
    /// Color Indicates Selected Areas (a radio item).
    ColorIndicatesSelected,
    /// Overlay Color › a preset.
    OverlayColor(OverlayPreset),
    /// Overlay Opacity › a percentage of [`OVERLAY_OPACITIES`](super::OVERLAY_OPACITIES).
    OverlayOpacity(u8),
    /// Enable or Disable Layer Mask (the layer-mask row).
    ToggleLayerMask,
    /// Delete Layer Mask.
    DeleteLayerMask,
    /// Split Channels.
    SplitChannels,
    /// Merge Channels….
    MergeChannels,
}

impl ChannelsCommand {
    /// The command's panel-menu id: `channels.new`, `channels.newSpot`, `channels.duplicate`,
    /// `channels.delete`, `channels.mergeSpot`, `channels.convertToAlpha`,
    /// `channels.exitQuickMask`, `channels.indicatesMasked`, `channels.indicatesSelected`,
    /// `channels.overlay.<red|green|blue|cyan|magenta|yellow>`, `channels.opacity.<n>`,
    /// `channels.toggleLayerMask`, `channels.deleteLayerMask`, `channels.split`, `channels.merge`.
    pub fn id(self) -> String {
        // new: ids for the docking host's `PanelCommand`.
        todo!()
    }

    /// The command of an [`id`](Self::id); `None` for any other text.
    pub fn from_id(id: &str) -> Option<ChannelsCommand> {
        // new: the inverse of `id`.
        todo!()
    }
}

/// The context-menu entries of `row`, by its kind, then the document's commands:
///
/// - alpha: Duplicate Channel, Delete Channel, Rename Channel…, Channel Options…, a separator,
///   Color Indicates Masked Areas and Selected Areas (radio, from `indicates`), Overlay Color ›
///   the six presets, Overlay Opacity › 25 %, 50 %, 75 %, 100 %;
/// - spot: Duplicate, Delete, Rename…, Channel Options…, a separator, Merge Spot Channel, Convert
///   to Alpha Channel;
/// - Quick Mask: Exit Quick Mask, Quick Mask Options…, the Color Indicates radios, Overlay Color
///   and Overlay Opacity;
/// - layer mask: Disable (or Enable) Layer Mask, Delete Layer Mask, Layer Mask Display Options…;
/// - composite and colour: Duplicate Channel;
/// - then a separator, New Channel…, New Spot Channel…, a separator, Split Channels, Merge
///   Channels….
///
/// Commands are [`ChannelsCommand::id`]s, plus `channels.rename` and `channels.options`.
pub(super) fn row_menu_entries<K>(row: &ChannelRow<K>, labels: &ChannelsLabels) -> Vec<MenuEntry<PanelCommand>> {
    // pc: crates/ui-egui/src/panels/channels_panel.rs:159-204 `draw_row_menu`, `draw_alpha_menu`,
    // `draw_quick_mask_menu`, `draw_mask_menu`; :572-605 `indicates_items`, `overlay_colors`.
    todo!()
}

/// The event of a picked menu `command` on the row `key`: [`ChannelsEvent::Command`] for a
/// [`ChannelsCommand`] (`key` `None` for New Channel…, New Spot Channel…, Split Channels and
/// Merge Channels…), [`ChannelsEvent::Options`] for `channels.options`. `channels.rename` starts
/// the row's rename (kept in egui memory under the panel's id, read by the panel's list) and
/// reports nothing. `None` for an unknown command.
pub(super) fn command_event<K: Clone + Send + Sync + 'static>(
    ctx: &Context,
    panel: egui::Id,
    key: Option<&K>,
    command: &str,
) -> Option<ChannelsEvent<K>> {
    // pc: crates/ui-egui/src/panels/channels_panel.rs:493-509 (`ui.renameChannel`, `ui.channelOptions`, the rest run).
    todo!()
}

/// Shows the row context menu on the list's response: opens it for `request` (the row's key and
/// where), keeps the row in egui memory under the panel's id while it is open, rebuilds
/// [`row_menu_entries`] each pass and reports a pick through [`command_event`].
pub(super) fn show_row_menu<K: Clone + Eq + Hash + Debug + Send + Sync + 'static>(
    ui: &Ui,
    list: &Response,
    rows: &[ChannelRow<K>],
    labels: &ChannelsLabels,
    request: Option<(K, OpenAt)>,
) -> Vec<ChannelsEvent<K>> {
    // pc: crates/ui-egui/src/panels/channels_panel.rs:88 (the row's `context_menu`).
    todo!()
}

impl<K: Clone + Eq + Hash + Debug + Send + Sync + 'static> ChannelsPanel<'_, K> {
    /// The panel menu, for the docking host's `panel_menu_entries` hook: the context-menu
    /// entries of [`targeted_row`](super::targeted_row) of `rows` (the document's commands only
    /// without one), as Photoshop's Channels panel menu (lead call 21).
    pub fn panel_menu(ctx: &Context, id_salt: impl AsIdSalt, rows: &[ChannelRow<K>]) -> Vec<MenuEntry<PanelCommand>> {
        // new: `row_menu_entries` of the targeted row, with `ChannelsLabels::of_ctx`.
        todo!()
    }

    /// Carries out a command of [`panel_menu`](Self::panel_menu) on the targeted row of `rows`,
    /// as a context-menu pick does (`menu::command_event`). `id_salt` is the panel's.
    pub fn run_command(ctx: &Context, id_salt: impl AsIdSalt, rows: &[ChannelRow<K>], command: &PanelCommand) -> Option<ChannelsEvent<K>> {
        // new: `command_event` with the targeted row's key.
        todo!()
    }
}
