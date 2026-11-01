//! `LayersLabels`: every user-visible word of the Layers panel's rows, footer and menus.
//!
//! Patterns with `{row}`, `{layer}` and `{mask}` are whole phrases, so a language puts the name
//! where its grammar wants it (`craft_i18n::fmt`).

craft_ui::words! {
    /// The words of the Layers panel. The mask and Delete buttons' words come from the app
    /// ([`LayersFooterState`](crate::ui::tree::LayersFooterState)), since they follow the target.
    pub struct LayersLabels {
        /// The list's name for a screen reader.
        list: "layers.list" = "Layers";
        /// The eye cell's name: `{row}` is the layer name.
        visibility: "layers.visibility" = "Visibility {row}";
        /// The lock cell's name.
        lock_cell: "layers.lockCell" = "Lock {row}";
        /// The lock indicator's tooltip: a click unlocks.
        unlock: "layers.unlock" = "Unlock {row}";
        /// The mask chain's names: `{mask}` is "layer mask" or "vector mask".
        link_mask: "layers.linkMask" = "Link {mask} of {layer}";
        unlink_mask: "layers.unlinkMask" = "Unlink {mask} of {layer}";
        /// The accessible name of the rename field.
        name: "layers.name" = "Name";
        /// The effects triangle's names.
        expand_effects: "layers.expandEffects" = "Expand effects {layer}";
        collapse_effects: "layers.collapseEffects" = "Collapse effects {layer}";
        /// The effects triangle's tooltip: `{key}` is the platform's Alt.
        effects_tip: "layers.effectsTip" = "Show or hide the layer's effects  ({key}-click: all layers)";
        /// The sub-row headers.
        effects: "layers.effects" = "Effects";
        smart_filters: "layers.smartFilters" = "Smart Filters";
        /// The sub-rows' descriptions.
        effect_of: "layers.effectOf" = "effect of {layer}";
        smart_filter_of: "layers.smartFilterOf" = "Smart Filter of {layer}";
        /// The kind words of a row's description.
        kind_pixel: "layers.kindPixel" = "pixel layer";
        kind_group: "layers.kindGroup" = "group";
        kind_adjustment: "layers.kindAdjustment" = "adjustment layer";
        kind_fill: "layers.kindFill" = "fill layer";
        kind_text: "layers.kindText" = "type layer";
        kind_shape: "layers.kindShape" = "shape layer";
        kind_smart_object: "layers.kindSmartObject" = "smart object";
        kind_artboard: "layers.kindArtboard" = "artboard";
        kind_path: "layers.kindPath" = "path";
        kind_image: "layers.kindImage" = "image";
        kind_object: "layers.kindObject" = "object";
        /// The flag words of a row's description, joined with ", ".
        clipped: "layers.clipped" = "clipped";
        locked: "layers.locked" = "locked";
        linked: "layers.linked" = "linked";
        layer_mask: "layers.layerMask" = "layer mask";
        vector_mask: "layers.vectorMask" = "vector mask";
        /// The mask names as titles (the drag ghost of a mask row), capitalised.
        layer_mask_title: "layers.layerMaskTitle" = "Layer Mask";
        vector_mask_title: "layers.vectorMaskTitle" = "Vector Mask";
        /// The effects mark beside a row's triangle, "fx" in every language.
        fx: "layers.fx" = "fx";
        disabled_mask: "layers.disabledMask" = "disabled mask";
        has_effects: "layers.hasEffects" = "effects";
        has_smart_filters: "layers.hasSmartFilters" = "Smart Filters";
        hidden: "layers.hidden" = "hidden";
        /// The target words of the active row's description.
        pixels_targeted: "layers.pixelsTargeted" = "pixels targeted";
        layer_mask_targeted: "layers.layerMaskTargeted" = "layer mask targeted";
        vector_mask_targeted: "layers.vectorMaskTargeted" = "vector mask targeted";
        filter_mask_targeted: "layers.filterMaskTargeted" = "filter mask targeted";
        /// The footer buttons' tooltips and names, from the reading start.
        link_layers: "layers.linkLayers" = "Link layers";
        /// The mask button's name while the app offers no mask to add (the button is greyed).
        add_mask: "layers.addMask" = "Add a mask";
        add_style: "layers.addStyle" = "Add a layer style";
        add_adjustment: "layers.addAdjustment" = "Create new fill or adjustment layer";
        new_group: "layers.newGroup" = "Create a new group";
        new_layer: "layers.newLayer" = "Create a new layer";
        /// The panel menu's library rows.
        filter_options: "layers.menu.filterOptions" = "Filter Options";
        show_filters: "layers.menu.showFilters" = "Show";
        panel_options: "layers.menu.panelOptions" = "Panel Options…";
        collapse_all: "layers.menu.collapseAll" = "Collapse All Groups";
        clip_thumbnails: "layers.menu.clipThumbnails" = "Clip Thumbnails to Layer Bounds";
    }
}
