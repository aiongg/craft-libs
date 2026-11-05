//! The view modes of a mask and their controls.
//!
//! Select and Mask's View Mode: how the preview shows the refined selection. Photoshop's seven
//! views ([`MaskViewMode`]: Onion Skin, Marching Ants, Overlay, On Black, On White, Black & White,
//! On Layers), each with its key; an Opacity for the four that veil the unselected area; Show Edge
//! and Show Original; hold X to see the image without a view.
//!
//! - [`MaskViewState`]: the dialog's view state, a plain value the workspace owns. Not undoable.
//! - [`MaskViewState::recipe`]: what the engine builds for the current view ([`MaskViewRecipe`]:
//!   which coverage, the veils, the active layer's mask, whether the ants show). The library cannot
//!   composite the view itself: it has no document or layer type.
//! - [`MaskViewKeys`]: O M V A T K Y pick a view, F the next, X held hides the view, J Show Edge, P
//!   Show Original.
//! - [`MaskViewControls`]: the View Mode block of the Properties panel.
//!
//! ```ignore
//! MaskViewKeys::read(ui, &mut state.view);
//! let out = MaskViewControls::new("view", &mut state.view).show(ui);
//! if out.changed.is_some() { app.request_preview(state.view.recipe()) }   // the engine re-renders
//! SelectionOverlay::new(("sam", id), &proxy_mask).visible(state.view.recipe().ants).reduction(k).show(ui, &canvas, &view);
//! ```

mod controls;
mod keys;
mod state;
pub(crate) mod stories;
#[cfg(test)]
mod tests;

pub use controls::{MaskViewControls, MaskViewResponse};
pub use keys::MaskViewKeys;
pub use state::{MaskCoverage, MaskViewChange, MaskViewMode, MaskViewRecipe, MaskViewState, Veil, VeilMask};

craft_ui::words! {
    /// The words of the mask views, English by default.
    // Every field is one self-evident word; its English text is the default.
    #[allow(missing_docs)]
    #[non_exhaustive]
    pub struct MaskViewLabels {
        /// The row label beside the view drop-down.
        view: "maskView.view" = "View";
        /// The view drop-down's accessible name.
        view_mode: "maskView.viewMode" = "View Mode";
        /// The opacity of the current view.
        opacity: "maskView.opacity" = "Opacity";
        show_edge: "maskView.showEdge" = "Show Edge";
        show_original: "maskView.showOriginal" = "Show Original";
        /// The hint under the controls.
        hint: "maskView.hint" = "F cycles the views; hold X to see the image";
        onion_skin: "maskView.onionSkin" = "Onion Skin";
        marching_ants: "maskView.marchingAnts" = "Marching Ants";
        overlay: "maskView.overlay" = "Overlay";
        on_black: "maskView.onBlack" = "On Black";
        on_white: "maskView.onWhite" = "On White";
        black_and_white: "maskView.blackAndWhite" = "Black & White";
        on_layers: "maskView.onLayers" = "On Layers";
    }
}
