//! The selection on the canvas: marching ants, preview and Quick Mask.
//!
//! [`SelectionOverlay`] draws a selection the app holds as a mask ([`MaskSource`]) or as segments
//! it traced itself: marching ants along a cached trace of the visible region, at the offset of a
//! floating piece or a selection drag, through a point map under a Free Transform, and under them
//! the Selection Preview or Quick Mask tint ([`PreviewMode`], [`QuickMaskLook`]).
//!
//! [`MaskSource`]: crate::trace::MaskSource
//! [`PreviewMode`]: crate::selection::PreviewMode
//! [`QuickMaskLook`]: crate::selection::QuickMaskLook

mod overlay;
pub(crate) mod stories;
#[cfg(test)]
mod tests;

pub use overlay::{OutlineSource, SelectionOverlay, SelectionOverlayLabels, SelectionOverlayResponse};
