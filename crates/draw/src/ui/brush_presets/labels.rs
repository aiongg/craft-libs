//! The words of the Brush Preset picker, its chip and its cards (`brushPresets.*`).

craft_ui::words! {
    /// The Brush Preset picker's and the options-bar chip's words, English by default.
    #[non_exhaustive]
    pub struct BrushPickerLabels {
        /// The picker's popover and the chip, for AccessKit and the chip's tooltip.
        picker: "brushPresets.picker" = "Brush Preset picker";
        /// The Size row's label and field name.
        size: "brushPresets.size" = "Size";
        /// The Hardness slider's label.
        hardness: "brushPresets.hardness" = "Hardness";
        /// The search field's placeholder and name.
        search: "brushPresets.search" = "Search Brushes";
        /// The Brush Settings button's name.
        settings: "brushPresets.settings" = "Brush Settings…";
        /// The Brush Settings button's tooltip.
        settings_tip: "brushPresets.settingsTip" = "Show every option of this brush";
        /// The New Brush Preset button's name.
        new_preset: "brushPresets.newPreset" = "New Brush Preset";
        /// The New Brush Preset button's tooltip.
        new_preset_tip: "brushPresets.newPresetTip" = "Create new brush preset from the current settings";
        /// The gear button's name and tooltip.
        options: "brushPresets.options" = "Brush Preset Options";
        /// Gear menu: New Brush Preset….
        menu_new: "brushPresets.menuNew" = "New Brush Preset…";
        /// Gear menu: rename the current preset.
        menu_rename: "brushPresets.menuRename" = "Rename Brush…";
        /// Gear menu: delete the current preset.
        menu_delete: "brushPresets.menuDelete" = "Delete Brush";
        /// Gear menu: the name part box.
        part_name: "brushPresets.partName" = "Brush Name";
        /// Gear menu: the stroke part box.
        part_stroke: "brushPresets.partStroke" = "Brush Stroke";
        /// Gear menu: the tip part box.
        part_tip: "brushPresets.partTip" = "Brush Tip";
        /// Gear menu: Import Brushes….
        menu_import: "brushPresets.menuImport" = "Import Brushes…";
        /// The footer's card scale slider, for AccessKit.
        card_size: "brushPresets.cardSize" = "Card size";
    }
}

craft_ui::words! {
    /// The words of the brush preset cards, English by default. Templates take `{name}` and
    /// `{size}`.
    #[non_exhaustive]
    pub struct BrushCardLabels {
        /// The card list, for AccessKit.
        presets: "brushPresets.presets" = "Brush presets";
        /// A card's accessible name: "{name}, {size} px".
        preset_size: "brushPresets.presetSize" = "{name}, {size} px";
    }
}
