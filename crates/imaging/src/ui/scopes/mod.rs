//! The scope panel: Photoshop's Histogram panel and Camera Raw's scope with tone zones,
//! clipping warnings and the vectorscope, plus the clipping overlay for a preview
//! (`ScopePanel`, `VectorscopePlot`, `ClippingOverlay`).

mod clipping;
mod labels;
mod menu;
mod panel;
mod plot;
pub(crate) mod stories;
mod style;
mod vectorscope;
mod zones;
