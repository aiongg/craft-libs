//! `ChannelsLabels`: every user-visible word of the Channels panel, and the shortcut chords
//! the rows display.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::Context;

use super::model::ChannelRow;

craft_ui::words! {
    /// The words of the Channels panel. Channel names are the app's ([`ChannelRow::name`]).
    pub struct ChannelsLabels {
        /// The list's name for a screen reader.
        list: "channels.list" = "Channels";
        /// The eye cell's name: `{row}` is the channel name.
        visibility: "channels.visibility" = "Visibility {row}";
        /// The eye's tooltip.
        toggle_visibility: "channels.toggleVisibility" = "Toggle visibility";
        /// The layer-mask row's eye tooltip.
        mask_overlay: "channels.maskOverlay" = "Show the layer mask as an overlay";
        /// A hidden row's description.
        hidden: "channels.hidden" = "Hidden";
        /// The footer buttons' tooltips and names.
        delete: "channels.delete" = "Delete current channel";
        new: "channels.new" = "Create new channel";
        save_selection: "channels.saveSelection" = "Save selection as channel";
        load_selection: "channels.loadSelection" = "Load channel as selection";
        /// The context menu.
        duplicate_channel: "channels.menu.duplicate" = "Duplicate Channel";
        delete_channel: "channels.menu.delete" = "Delete Channel";
        rename_channel: "channels.menu.rename" = "Rename Channel…";
        channel_options: "channels.menu.options" = "Channel Options…";
        merge_spot: "channels.menu.mergeSpot" = "Merge Spot Channel";
        convert_to_alpha: "channels.menu.convertToAlpha" = "Convert to Alpha Channel";
        indicates_masked: "channels.menu.indicatesMasked" = "Color Indicates Masked Areas";
        indicates_selected: "channels.menu.indicatesSelected" = "Color Indicates Selected Areas";
        overlay_color: "channels.menu.overlayColor" = "Overlay Color";
        red: "channels.menu.red" = "Red";
        green: "channels.menu.green" = "Green";
        blue: "channels.menu.blue" = "Blue";
        cyan: "channels.menu.cyan" = "Cyan";
        magenta: "channels.menu.magenta" = "Magenta";
        yellow: "channels.menu.yellow" = "Yellow";
        overlay_opacity: "channels.menu.overlayOpacity" = "Overlay Opacity";
        /// An opacity entry: `{value}` is the percentage in the locale's digits.
        percent: "channels.menu.percent" = "{value}%";
        exit_quick_mask: "channels.menu.exitQuickMask" = "Exit Quick Mask";
        quick_mask_options: "channels.menu.quickMaskOptions" = "Quick Mask Options…";
        enable_layer_mask: "channels.menu.enableLayerMask" = "Enable Layer Mask";
        disable_layer_mask: "channels.menu.disableLayerMask" = "Disable Layer Mask";
        delete_layer_mask: "channels.menu.deleteLayerMask" = "Delete Layer Mask";
        mask_display_options: "channels.menu.maskDisplayOptions" = "Layer Mask Display Options…";
        new_channel: "channels.menu.newChannel" = "New Channel…";
        new_spot_channel: "channels.menu.newSpotChannel" = "New Spot Channel…";
        split_channels: "channels.menu.split" = "Split Channels";
        merge_channels: "channels.menu.merge" = "Merge Channels…";
    }
}

/// The shortcut number each row displays, in row order: the composite 2, then each colour, alpha
/// and spot row the next number while it is at most 9; Quick Mask and layer-mask rows `None`.
/// The panel only displays them; the app's keymap owns the keys.
pub(super) fn shortcut_numbers<K>(rows: &[ChannelRow<K>]) -> Vec<Option<u8>> {
    // pc: crates/ui-egui/src/panels/channels_panel.rs:249-284 `row_presentation` (`slot`: Composite 2,
    // colour 3 + k, alpha 3 + colours + i while at most 9).
    todo!()
}

/// The chord text of shortcut `number`: Cmd+n on macOS, Ctrl+n elsewhere, in the library's
/// [`Kbd`](craft_ui::controls::Kbd) form for the context's platform.
pub(super) fn chord_text(ctx: &Context, number: u8) -> String {
    // pc: crates/ui-egui/src/panels/channels_panel.rs:403-411 (`shortcuts::pretty("Cmd+{slot}")`, mono 11, unified to `Kbd`).
    todo!()
}
