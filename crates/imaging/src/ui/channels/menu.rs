//! The row and panel menus (`ChannelsCommand`, `ChannelsPanel::panel_menu`,
//! `ChannelsPanel::run_command`).


use std::fmt::Debug;
use std::hash::Hash;

use craft_theme::Srgba;
use craft_ui::docking::panel_menu::PanelCommand;
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::menu::{ContextMenu, MenuEntry, MenuItem, OpenAt};
use egui::{AsIdSalt, Context, Id, Response, Ui};

use super::labels::ChannelsLabels;
use super::model::{ChannelKind, ChannelRow, ChannelsEvent, ColorIndicates, OVERLAY_OPACITIES, OverlayPreset, targeted_row};
use super::panel::ChannelsPanel;

const RENAME: &str = "channels.rename";
const OPTIONS: &str = "channels.options";

/// The egui memory slot under `panel` where Rename Channel… leaves the key of the row to rename.
/// The panel's list reads it with [`take_pending_rename`] and opens that row's rename field.
fn pending_rename_id(panel: Id) -> Id {
    panel.with("pending-rename")
}

/// The panel id the panel menu's commands use: the one derived from the panel's `id_salt` alone.
pub(super) fn menu_panel_id(id_salt: impl AsIdSalt) -> Id {
    Id::NULL.with(id_salt)
}

/// The key of the row whose rename a menu asked for under `panel`, removed from memory.
pub(super) fn take_pending_rename<K: Clone + Send + Sync + 'static>(ctx: &Context, panel: Id) -> Option<K> {
    ctx.data_mut(|d| {
        let key = d.get_temp::<K>(pending_rename_id(panel));
        if key.is_some() {
            d.remove::<K>(pending_rename_id(panel));
        }
        key
    })
}

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
        match self {
            ChannelsCommand::NewChannel => "channels.new".to_owned(),
            ChannelsCommand::NewSpotChannel => "channels.newSpot".to_owned(),
            ChannelsCommand::Duplicate => "channels.duplicate".to_owned(),
            ChannelsCommand::Delete => "channels.delete".to_owned(),
            ChannelsCommand::MergeSpot => "channels.mergeSpot".to_owned(),
            ChannelsCommand::ConvertToAlpha => "channels.convertToAlpha".to_owned(),
            ChannelsCommand::ExitQuickMask => "channels.exitQuickMask".to_owned(),
            ChannelsCommand::ColorIndicatesMasked => "channels.indicatesMasked".to_owned(),
            ChannelsCommand::ColorIndicatesSelected => "channels.indicatesSelected".to_owned(),
            ChannelsCommand::OverlayColor(preset) => format!("channels.overlay.{}", preset_name(preset)),
            ChannelsCommand::OverlayOpacity(percent) => format!("channels.opacity.{percent}"),
            ChannelsCommand::ToggleLayerMask => "channels.toggleLayerMask".to_owned(),
            ChannelsCommand::DeleteLayerMask => "channels.deleteLayerMask".to_owned(),
            ChannelsCommand::SplitChannels => "channels.split".to_owned(),
            ChannelsCommand::MergeChannels => "channels.merge".to_owned(),
        }
    }

    /// The command of an [`id`](Self::id); `None` for any other text.
    pub fn from_id(id: &str) -> Option<ChannelsCommand> {
        // new: the inverse of `id`.
        let simple = match id {
            "channels.new" => ChannelsCommand::NewChannel,
            "channels.newSpot" => ChannelsCommand::NewSpotChannel,
            "channels.duplicate" => ChannelsCommand::Duplicate,
            "channels.delete" => ChannelsCommand::Delete,
            "channels.mergeSpot" => ChannelsCommand::MergeSpot,
            "channels.convertToAlpha" => ChannelsCommand::ConvertToAlpha,
            "channels.exitQuickMask" => ChannelsCommand::ExitQuickMask,
            "channels.indicatesMasked" => ChannelsCommand::ColorIndicatesMasked,
            "channels.indicatesSelected" => ChannelsCommand::ColorIndicatesSelected,
            "channels.toggleLayerMask" => ChannelsCommand::ToggleLayerMask,
            "channels.deleteLayerMask" => ChannelsCommand::DeleteLayerMask,
            "channels.split" => ChannelsCommand::SplitChannels,
            "channels.merge" => ChannelsCommand::MergeChannels,
            _ => {
                if let Some(name) = id.strip_prefix("channels.overlay.") {
                    return OverlayPreset::ALL.into_iter().find(|preset| preset_name(*preset) == name).map(ChannelsCommand::OverlayColor);
                }
                let percent = id.strip_prefix("channels.opacity.")?.parse::<u8>().ok()?;
                return OVERLAY_OPACITIES.contains(&percent).then_some(ChannelsCommand::OverlayOpacity(percent));
            }
        };
        Some(simple)
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
/// Commands are [`ChannelsCommand::id`]s, plus `channels.rename` and `channels.options`. `numbers`
/// writes the opacity percentages.
pub(super) fn row_menu_entries<K>(row: &ChannelRow<K>, labels: &ChannelsLabels, numbers: &LocaleNumbers) -> Vec<MenuEntry<PanelCommand>> {
    // pc: crates/ui-egui/src/panels/channels_panel.rs:159-204 `draw_row_menu`, `draw_alpha_menu`,
    // `draw_quick_mask_menu`, `draw_mask_menu`; :572-605 `indicates_items`, `overlay_colors`.
    let item = |label: &str, command: ChannelsCommand| MenuItem::new(label, command.id());
    let plain = |label: &str, id: &str| MenuItem::new(label, id.to_owned());
    let mut entries: Vec<MenuEntry<PanelCommand>> = Vec::new();
    match row.kind {
        ChannelKind::Alpha { indicates } => {
            entries.push(item(&labels.duplicate_channel, ChannelsCommand::Duplicate).into());
            entries.push(item(&labels.delete_channel, ChannelsCommand::Delete).into());
            entries.push(plain(&labels.rename_channel, RENAME).into());
            entries.push(plain(&labels.channel_options, OPTIONS).into());
            entries.push(MenuEntry::Separator);
            entries.extend(overlay_entries(labels, numbers, indicates));
        }
        ChannelKind::Spot { .. } => {
            entries.push(item(&labels.duplicate_channel, ChannelsCommand::Duplicate).into());
            entries.push(item(&labels.delete_channel, ChannelsCommand::Delete).into());
            entries.push(plain(&labels.rename_channel, RENAME).into());
            entries.push(plain(&labels.channel_options, OPTIONS).into());
            entries.push(MenuEntry::Separator);
            entries.push(item(&labels.merge_spot, ChannelsCommand::MergeSpot).into());
            entries.push(item(&labels.convert_to_alpha, ChannelsCommand::ConvertToAlpha).into());
        }
        ChannelKind::QuickMask { indicates } => {
            entries.push(item(&labels.exit_quick_mask, ChannelsCommand::ExitQuickMask).into());
            entries.push(plain(&labels.quick_mask_options, OPTIONS).into());
            entries.extend(overlay_entries(labels, numbers, indicates));
        }
        ChannelKind::LayerMask { enabled } => {
            let toggle = if enabled { &labels.disable_layer_mask } else { &labels.enable_layer_mask };
            entries.push(item(toggle, ChannelsCommand::ToggleLayerMask).into());
            entries.push(item(&labels.delete_layer_mask, ChannelsCommand::DeleteLayerMask).into());
            entries.push(plain(&labels.mask_display_options, OPTIONS).into());
        }
        ChannelKind::Composite | ChannelKind::Color => {
            entries.push(item(&labels.duplicate_channel, ChannelsCommand::Duplicate).into());
        }
    }
    entries.push(MenuEntry::Separator);
    entries.extend(document_entries(labels));
    entries
}

/// Document-level rows: New Channel…, New Spot Channel…, Split Channels, Merge Channels….
fn document_entries(labels: &ChannelsLabels) -> Vec<MenuEntry<PanelCommand>> {
    vec![
        MenuItem::new(labels.new_channel.as_str(), ChannelsCommand::NewChannel.id()).into(),
        MenuItem::new(labels.new_spot_channel.as_str(), ChannelsCommand::NewSpotChannel.id()).into(),
        MenuEntry::Separator,
        MenuItem::new(labels.split_channels.as_str(), ChannelsCommand::SplitChannels.id()).into(),
        MenuItem::new(labels.merge_channels.as_str(), ChannelsCommand::MergeChannels.id()).into(),
    ]
}

/// Color Indicates radios, then Overlay Color › and Overlay Opacity ›.
fn overlay_entries(labels: &ChannelsLabels, numbers: &LocaleNumbers, indicates: ColorIndicates) -> Vec<MenuEntry<PanelCommand>> {
    let colors = OverlayPreset::ALL
        .into_iter()
        .map(|preset| {
            let [r, g, b] = preset.rgb();
            MenuItem::new(preset_label(labels, preset), ChannelsCommand::OverlayColor(preset).id())
                .tint(Srgba::from(craft_theme::Rgba::rgb(r, g, b)))
                .into()
        })
        .collect();
    let opacities = OVERLAY_OPACITIES
        .into_iter()
        .map(|percent| MenuItem::new(percent_label(labels, numbers, percent), ChannelsCommand::OverlayOpacity(percent).id()).into())
        .collect();
    vec![
        MenuItem::new(labels.indicates_masked.as_str(), ChannelsCommand::ColorIndicatesMasked.id())
            .radio(indicates == ColorIndicates::MaskedAreas)
            .into(),
        MenuItem::new(labels.indicates_selected.as_str(), ChannelsCommand::ColorIndicatesSelected.id())
            .radio(indicates == ColorIndicates::SelectedAreas)
            .into(),
        MenuItem::submenu(labels.overlay_color.as_str(), colors).into(),
        MenuItem::submenu(labels.overlay_opacity.as_str(), opacities).into(),
    ]
}

fn preset_name(preset: OverlayPreset) -> &'static str {
    match preset {
        OverlayPreset::Red => "red",
        OverlayPreset::Green => "green",
        OverlayPreset::Blue => "blue",
        OverlayPreset::Cyan => "cyan",
        OverlayPreset::Magenta => "magenta",
        OverlayPreset::Yellow => "yellow",
    }
}

fn preset_label(labels: &ChannelsLabels, preset: OverlayPreset) -> &str {
    match preset {
        OverlayPreset::Red => &labels.red,
        OverlayPreset::Green => &labels.green,
        OverlayPreset::Blue => &labels.blue,
        OverlayPreset::Cyan => &labels.cyan,
        OverlayPreset::Magenta => &labels.magenta,
        OverlayPreset::Yellow => &labels.yellow,
    }
}

fn percent_label(labels: &ChannelsLabels, numbers: &LocaleNumbers, percent: u8) -> String {
    labels.percent.replace("{value}", &numbers.format(f64::from(percent), 0..=0))
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
    match command {
        RENAME => {
            if let Some(key) = key {
                ctx.data_mut(|d| d.insert_temp(pending_rename_id(panel), key.clone()));
                ctx.request_repaint();
            }
            None
        }
        OPTIONS => key.map(|key| ChannelsEvent::Options(key.clone())),
        _ => ChannelsCommand::from_id(command).map(|command| ChannelsEvent::Command { command, key: key.cloned() }),
    }
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
    let ctx = ui.ctx();
    let panel = list.id;
    let slot = panel.with("menu-row");
    if let Some((key, _)) = &request {
        ctx.data_mut(|d| d.insert_temp(slot, key.clone()));
    }
    let Some(row) = ctx.data(|d| d.get_temp::<K>(slot)).and_then(|key| rows.iter().find(|row| row.key == key)) else {
        return Vec::new();
    };
    let numbers = LocaleNumbers::of_ctx(ctx);
    let entries = row_menu_entries(row, labels, &numbers);
    let menu = ContextMenu::new(&entries).own_gestures(false).open_at(request.map(|(_, at)| at)).show(ui, list);
    let events = menu.chosen.and_then(|command| command_event(ctx, panel, Some(&row.key), &command)).into_iter().collect();
    if !menu.open {
        ctx.data_mut(|d| d.remove::<K>(slot));
    }
    events
}

impl<K: Clone + Eq + Hash + Debug + Send + Sync + 'static> ChannelsPanel<'_, K> {
    /// The panel menu, for the docking host's `panel_menu_entries` hook: the context-menu
    /// entries of [`targeted_row`](super::targeted_row) of `rows` (the document's commands only
    /// without one), as Photoshop's Channels panel menu (lead call 21).
    pub fn panel_menu(ctx: &Context, id_salt: impl AsIdSalt, rows: &[ChannelRow<K>]) -> Vec<MenuEntry<PanelCommand>> {
        // new: `row_menu_entries` of the targeted row, with `ChannelsLabels::of_ctx`.
        let _ = id_salt;
        let labels = ChannelsLabels::of_ctx(ctx);
        match targeted_row(rows) {
            Some(row) => row_menu_entries(row, &labels, &LocaleNumbers::of_ctx(ctx)),
            None => document_entries(&labels),
        }
    }

    /// Carries out a command of [`panel_menu`](Self::panel_menu) on the targeted row of `rows`,
    /// as a context-menu pick does (`menu::command_event`). `id_salt` is the panel's.
    pub fn run_command(ctx: &Context, id_salt: impl AsIdSalt, rows: &[ChannelRow<K>], command: &PanelCommand) -> Option<ChannelsEvent<K>> {
        // new: `command_event` with the targeted row's key.
        command_event(ctx, menu_panel_id(id_salt), targeted_row(rows).map(|row| &row.key), command)
    }
}
