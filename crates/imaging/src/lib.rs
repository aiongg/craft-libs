//! Raster image editing for Craft apps: warps, adjustments, layer styles, selection tools and masks, the full-window editors, channels and scopes.
//!
//! [`scopes`], [`selection`], [`trace`] and [`layer_style`] work without egui and build on every target. The widgets are behind the `ui` feature.

pub mod layer_style;
pub mod scopes;
pub mod selection;
pub mod trace;
#[cfg(feature = "ui")]
pub mod ui;

#[cfg(feature = "screen-testing")]
pub mod testing {
    craft_ui::testing_registered!();
}
