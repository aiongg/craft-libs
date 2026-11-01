//! [`FilterGalleryLabels`]: Filter Gallery's words. The filters' and parameters' names are the
//! app's (`GalleryFilter::label`, `Param` labels through its `I18n`); OK and Cancel are the
//! dialog's.

craft_ui::words! {
    /// Filter Gallery's words, English by default (Photoshop's names).
    #[non_exhaustive]
    pub struct FilterGalleryLabels {
        /// The title's name.
        name: "gallery.name" = "Filter Gallery";
        /// The middle column's accessible name.
        filters: "gallery.filters" = "Filters";
        /// The settings column's accessible name.
        settings: "gallery.settings" = "Settings";
        filter: "gallery.filter" = "Filter";
        effect_layers: "gallery.effectLayers" = "Effect Layers";
        move_up: "gallery.moveUp" = "Move effect layer up";
        move_down: "gallery.moveDown" = "Move effect layer down";
        new_layer: "gallery.newLayer" = "New effect layer";
        delete_layer: "gallery.deleteLayer" = "Delete effect layer";
        visibility: "gallery.visibility" = "Visibility";
    }
}
