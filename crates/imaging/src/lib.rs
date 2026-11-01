//! Raster image editing for Craft apps: warp, levels, channels and scopes now; masks, selection and the full-window editors later.
//!
//! [`scopes`] counts without egui and builds on every target. The widgets are behind the `ui` feature.

pub mod layer_style;
pub mod scopes;
pub mod selection;
pub mod trace;
#[cfg(feature = "ui")]
pub mod ui;
