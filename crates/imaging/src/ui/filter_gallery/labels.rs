//! [`FilterGalleryLabels`]: Filter Gallery's words. The filters' and parameters' names are the
//! app's (`GalleryFilter::label`, `Param` labels through its `I18n`); OK and Cancel are the
//! dialog's.

craft_ui::words! {
    /// Filter Gallery's words, English by default (Photoshop's names).
    #[non_exhaustive]
    pub struct FilterGalleryLabels {
        /// The title's name.
        name: "filterGallery.name" = "Filter Gallery";
        /// The middle column's accessible name.
        filters: "filterGallery.filters" = "Filters";
        /// The settings column's accessible name.
        settings: "filterGallery.settings" = "Settings";
        filter: "filterGallery.filter" = "Filter";
        effect_layers: "filterGallery.effectLayers" = "Effect Layers";
        move_up: "filterGallery.moveUp" = "Move effect layer up";
        move_down: "filterGallery.moveDown" = "Move effect layer down";
        new_layer: "filterGallery.newLayer" = "New effect layer";
        delete_layer: "filterGallery.deleteLayer" = "Delete effect layer";
        visibility: "filterGallery.visibility" = "Visibility";
    }
}
