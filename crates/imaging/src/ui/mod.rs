//! The raster image editing widgets, for egui.
//!
//! - [`warp`]: the [`WarpGrid`] on a canvas, the mesh of PhotoCraft's Edit › Transform › Warp, its
//!   anchors and handles dragged and nudged.
//! - [`adjust`]: the [`LevelsEditor`], a channel's levels over its histogram.
//! - [`channels`]: the Channels panel.
//! - [`scopes`]: the scope panel (histogram, statistics, tone zones, vectorscope) and the
//!   clipping overlay.

pub mod adjust;
pub mod blend_keys;
pub(crate) mod blend_keys_stories;
#[cfg(test)]
mod blend_keys_tests;
pub mod blur_pins;
pub mod channels;
pub mod fill;
pub mod filter_gallery;
pub mod layer_style;
pub mod liquify;
pub mod live_preview;
pub mod mask_view;
pub(crate) mod mock_image;
pub mod scopes;
pub mod select;
pub mod select_mask;
pub mod selection;
pub mod transform_preview;
pub mod vanishing_point;
pub mod warp;
pub mod wide_angle;

#[cfg(test)]
mod id_tests;
#[cfg(test)]
mod words_tests;

pub use adjust::{Levels, LevelsClip, LevelsEditor, LevelsLabels, LevelsStyle};
pub use channels::{
    ChannelKind, ChannelRow, ChannelsCommand, ChannelsEvent, ChannelsFooterButton, ChannelsLabels, ChannelsOutput, ChannelsPanel, ChannelsStyle,
    ColorIndicates, OVERLAY_OPACITIES, OverlayPreset, targeted_row,
};
pub use scopes::{
    COLORS_CHANNEL, ChannelOverlay, ClippingDiagnostic, ClippingFrame, ClippingMode, ClippingOverlay, ClippingWarnings, HistogramSet, ScopeChannel,
    ScopeFrame, ScopeMenuAction, ScopePanel, ScopePanelLabels, ScopePanelOutput, ScopePanelState, ScopePanelStyle, ScopeReadout, ScopeSettings,
    ScopeSource, ScopeView, ScopeZoneEdit, ToneZone, VectorscopeFrame, VectorscopePlot, ZoneChange, scope_point,
};
pub use warp::{
    WarpEdit, WarpGrid, WarpMesh, WarpNames, WarpPress, WarpRelease, WarpResponse, WarpSide, WarpSplit, WarpSplitAt, WarpSplitPress, WarpStyle,
    WarpTarget, WarpTrack,
};

craft_ui::story::register! {
    warp::story,
    adjust,
    channels::stories,
    scopes::stories,
}

/// The stories of the imaging widgets.
pub fn stories() -> Vec<craft_ui::story::Story> {
    registry()
}

/// The count words of the imaging widgets: their plural forms are in [`WORDS`], these give the
/// singular forms a catalog extraction needs.
pub const COUNTS: &[craft_ui::localized::words::Counted] = &[select::OUTLINE_POINTS, warp::perspective::PLANES, wide_angle::CONSTRAINTS];

/// The plural forms of [`COUNTS`], as a words table.
const COUNT_WORDS: &[craft_ui::localized::Word] =
    &[select::OUTLINE_POINTS.word, warp::perspective::PLANES.word, wide_angle::CONSTRAINTS.word];

/// The labels tables of the imaging widgets, for the words registry.
pub const WORDS: &[&[craft_ui::localized::Word]] = &[
    COUNT_WORDS,
    <WarpNames<'static> as craft_ui::localized::Labels>::WORDS,
    <LevelsLabels as craft_ui::localized::Labels>::WORDS,
    <ChannelsLabels as craft_ui::localized::Labels>::WORDS,
    <ScopePanelLabels as craft_ui::localized::Labels>::WORDS,
    <channels::stories::ChannelsStoryLabels as craft_ui::localized::Labels>::WORDS,
    <scopes::stories::ScopeStoryLabels as craft_ui::localized::Labels>::WORDS,
    <adjust::AdjustLabels as craft_ui::localized::Labels>::WORDS,
    <adjust::stories::AdjustStoryLabels as craft_ui::localized::Labels>::WORDS,
    <liquify::LiquifyLabels as craft_ui::localized::Labels>::WORDS,
    <liquify::stories::LiquifyStoryLabels as craft_ui::localized::Labels>::WORDS,
    <filter_gallery::FilterGalleryLabels as craft_ui::localized::Labels>::WORDS,
    <filter_gallery::stories::GalleryStoryLabels as craft_ui::localized::Labels>::WORDS,
    <wide_angle::WideAngleLabels as craft_ui::localized::Labels>::WORDS,
    <layer_style::LayerStyleLabels as craft_ui::localized::Labels>::WORDS,
    <layer_style::BlendIfBarLabels as craft_ui::localized::Labels>::WORDS,
    <layer_style::ContourFieldLabels as craft_ui::localized::Labels>::WORDS,
    <layer_style::stories::LayerStyleStoryLabels as craft_ui::localized::Labels>::WORDS,
    <blur_pins::BlurPinsLabels as craft_ui::localized::Labels>::WORDS,
    <blur_pins::stories::BlurPinsStoryLabels as craft_ui::localized::Labels>::WORDS,
    <fill::FillLabels as craft_ui::localized::Labels>::WORDS,
    <fill::stories::FillStoryLabels as craft_ui::localized::Labels>::WORDS,
    <live_preview::stories::PreviewStoryLabels as craft_ui::localized::Labels>::WORDS,
    <warp::perspective::stories::PerspectiveStoryLabels as craft_ui::localized::Labels>::WORDS,
    <warp::perspective::PerspectiveWarpLabels as craft_ui::localized::Labels>::WORDS,
    <warp::perspective::PerspectiveBarLabels as craft_ui::localized::Labels>::WORDS,
    <warp::fields_stories::WarpFieldsStoryLabels as craft_ui::localized::Labels>::WORDS,
    <warp::WarpFieldsLabels as craft_ui::localized::Labels>::WORDS,
    <warp::pin::stories::PinWarpStoryLabels as craft_ui::localized::Labels>::WORDS,
    <warp::pin::PinWarpLabels as craft_ui::localized::Labels>::WORDS,
    <warp::pin::PinWarpBarLabels as craft_ui::localized::Labels>::WORDS,
    <blend_keys_stories::BlendKeysStoryLabels as craft_ui::localized::Labels>::WORDS,
    <mask_view::MaskViewLabels as craft_ui::localized::Labels>::WORDS,
    <select::SelectLabels as craft_ui::localized::Labels>::WORDS,
    <vanishing_point::VanishingPointLabels as craft_ui::localized::Labels>::WORDS,
    <vanishing_point::stories::VanishingPointStoryLabels as craft_ui::localized::Labels>::WORDS,
    <select_mask::SelectMaskLabels as craft_ui::localized::Labels>::WORDS,
    <select_mask::stories::SelectMaskStoryLabels as craft_ui::localized::Labels>::WORDS,
    <selection::stories::SelectionStoryLabels as craft_ui::localized::Labels>::WORDS,
    <selection::SelectionOverlayLabels as craft_ui::localized::Labels>::WORDS,
    <transform_preview::stories::TransformPreviewStoryLabels as craft_ui::localized::Labels>::WORDS,
];
