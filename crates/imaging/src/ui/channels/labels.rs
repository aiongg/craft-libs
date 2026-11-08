//! `ChannelsWords`: every user-visible word of the Channels panel, and the shortcut chords the rows display.

use craft_keymap::{Chord, Key, Mods, Platform};

use super::model::{ChannelKind, ChannelRow};

craft_ui::words! {
    /// The words of the Channels panel. Channel names are the app's ([`ChannelRow::name`]).
    pub struct ChannelsWords {
        /// The list's name for a screen reader.
        list: "channelsPanel.list" = "Channels";
        /// The eye cell's name: `{row}` is the channel name.
        visibility: "channelsPanel.visibility" = "Visibility {row}";
        /// The eye's tooltip.
        toggle_visibility: "channelsPanel.toggleVisibility" = "Toggle visibility";
        /// The layer-mask row's eye tooltip.
        mask_overlay: "channelsPanel.maskOverlay" = "Show the layer mask as an overlay";
        /// A hidden row's description.
        hidden: "channelsPanel.hidden" = "Hidden";
        /// The footer buttons' tooltips and names.
        delete: "channelsPanel.delete" = "Delete current channel";
        new: "channelsPanel.new" = "Create new channel";
        save_selection: "channelsPanel.saveSelection" = "Save selection as channel";
        load_selection: "channelsPanel.loadSelection" = "Load channel as selection";
        /// The row menu's name.
        row_menu: "channelsPanel.rowMenu" = "Channel";
        /// The context menu.
        duplicate_channel: "channelsPanel.menu.duplicate" = "Duplicate Channel";
        delete_channel: "channelsPanel.menu.delete" = "Delete Channel";
        rename_channel: "channelsPanel.menu.rename" = "Rename Channel…";
        channel_options: "channelsPanel.menu.options" = "Channel Options…";
        merge_spot: "channelsPanel.menu.mergeSpot" = "Merge Spot Channel";
        convert_to_alpha: "channelsPanel.menu.convertToAlpha" = "Convert to Alpha Channel";
        indicates_masked: "channelsPanel.menu.indicatesMasked" = "Color Indicates Masked Areas";
        indicates_selected: "channelsPanel.menu.indicatesSelected" = "Color Indicates Selected Areas";
        overlay_color: "channelsPanel.menu.overlayColor" = "Overlay Color";
        red: "channelsPanel.menu.red" = "Red";
        green: "channelsPanel.menu.green" = "Green";
        blue: "channelsPanel.menu.blue" = "Blue";
        cyan: "channelsPanel.menu.cyan" = "Cyan";
        magenta: "channelsPanel.menu.magenta" = "Magenta";
        yellow: "channelsPanel.menu.yellow" = "Yellow";
        overlay_opacity: "channelsPanel.menu.overlayOpacity" = "Overlay Opacity";
        /// An opacity entry: `{value}` is the percentage in the locale's digits.
        percent: "channelsPanel.menu.percent" = "{value}%";
        exit_quick_mask: "channelsPanel.menu.exitQuickMask" = "Exit Quick Mask";
        quick_mask_options: "channelsPanel.menu.quickMaskOptions" = "Quick Mask Options…";
        enable_layer_mask: "channelsPanel.menu.enableLayerMask" = "Enable Layer Mask";
        disable_layer_mask: "channelsPanel.menu.disableLayerMask" = "Disable Layer Mask";
        delete_layer_mask: "channelsPanel.menu.deleteLayerMask" = "Delete Layer Mask";
        mask_display_options: "channelsPanel.menu.maskDisplayOptions" = "Layer Mask Display Options…";
        new_channel: "channelsPanel.menu.newChannel" = "New Channel…";
        new_spot_channel: "channelsPanel.menu.newSpotChannel" = "New Spot Channel…";
        split_channels: "channelsPanel.menu.split" = "Split Channels";
        merge_channels: "channelsPanel.menu.merge" = "Merge Channels…";
    }
}

/// The shortcut number each row displays, in row order: the composite 2, then each colour, alpha and spot row the
/// next number while it is at most 9; Quick Mask and layer-mask rows `None`. The panel only displays them; the app's
/// keymap owns the keys.
pub(super) fn shortcut_numbers<K>(rows: &[ChannelRow<K>]) -> Vec<Option<u8>> {
    let mut next = 2u8;
    rows.iter()
        .map(|row| match row.kind {
            ChannelKind::Composite | ChannelKind::Color | ChannelKind::Alpha { .. } | ChannelKind::Spot { .. } => {
                let number = next;
                next = next.saturating_add(1);
                (number <= 9).then_some(number)
            }
            ChannelKind::QuickMask { .. } | ChannelKind::LayerMask { .. } => None,
        })
        .collect()
}

/// The chord text of shortcut `number`: Cmd+n on macOS, Ctrl+n elsewhere.
pub(super) fn chord_text(number: u8) -> Option<String> {
    let key = Key::parse(&number.to_string())?;
    Some(Chord::new(Mods::CMD, key).display(Platform::current()))
}
