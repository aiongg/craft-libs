//! The raster image editing widgets, for egui.
//!
//! - [`warp`]: the [`WarpGrid`] on a canvas, the mesh of a mesh warp, its anchors and handles
//!   dragged and nudged; the pin warp ([`PinWarp`]) and perspective warp ([`PerspectiveWarp`]) with
//!   their bars, and the [`WarpFields`] of the options bar.
//! - [`transform_preview`]: the [`TransformPreview`] of a transform or warp in progress.
//! - [`curve`]: the [`CurveEditor`] and [`CurvePreview`]; [`histogram`]: the [`Histogram`] plot.
//! - [`adjust`]: the [`LevelsEditor`] and the editors of the other colour adjustments (curves,
//!   hue and saturation, color balance, channel mixer, selective color and more).
//! - [`live_preview`]: the [`LivePreview`] of an adjustment on a proxy of the image.
//! - [`layer_style`]: the [`LayerStyleDialog`], the [`BlendIfBar`] and the [`ContourField`].
//! - [`blend_keys`]: the [`BlendKeys`] that pick a blend mode from the keyboard.
//! - [`select`]: the [`MarqueeTool`] and the lasso tools; [`selection`]: the [`SelectionOverlay`];
//!   [`mask_view`]: the [`MaskViewControls`] for a mask's views.
//! - [`select_mask`], [`fill`], [`filter_gallery`], [`liquify`], [`wide_angle`] and
//!   [`vanishing_point`]: the full-window editors ([`SelectMaskWorkspace`],
//!   [`ContentAwareFillWorkspace`], [`FilterGalleryWorkspace`], [`LiquifyWorkspace`],
//!   [`WideAngleWorkspace`], [`VanishingPointWorkspace`]), each with its session, panel and options bar.
//! - [`blur_pins`]: the [`BlurPinsOverlay`] and [`BlurPinsPanel`] of the blur effects.
//! - [`channels`]: the Channels panel.
//! - [`scopes`]: the scope panel (histogram, statistics, tone zones, vectorscope) and the
//!   clipping overlay.
//!
//! The main types of each module are also re-exported here.

pub mod blend_keys;
pub mod histogram;
#[cfg(feature = "screen-testing")]
pub(crate) mod mock_image;
pub mod scopes;

pub use blend_keys::{BlendKeyEntry, BlendKeyPick, BlendKeys, BlendShortcut, BlendStep};
pub use scopes::*;
