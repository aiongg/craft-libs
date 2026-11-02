//! The raster image editing widgets, for egui.
//!
//! - [`warp`]: the [`WarpGrid`] on a canvas, the mesh of a mesh warp, its anchors and handles
//!   dragged and nudged; the pin warp ([`PinWarp`]) and perspective warp ([`PerspectiveWarp`]) with
//!   their bars, and the [`WarpFields`] of the options bar.
//! - [`transform_preview`]: the [`TransformPreview`] of a transform or warp in progress.
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

pub use adjust::{
    AdjustContext, AdjustLabels, AdjustSession, AdjustSliders, AdjustStyle, BlackWhite, BlackWhiteEditor, BrightnessContrast,
    BrightnessContrastEditor, ChannelMixer, ChannelMixerEditor, ColorBalance, ColorBalanceEditor, CurvesMode, CurvesPanel, CurvesPanelOutput,
    HueSaturation, HueSaturationEditor, LevelsPanel, LevelsPanelOutput, PhotoFilter, PhotoFilterEditor, SelectiveColor, SelectiveColorEditor,
    ThresholdEditor, ToneCurves, TonePickTool, TonePicker,
};
pub use adjust::{Levels, LevelsClip, LevelsEditor, LevelsLabels, LevelsStyle};
pub use blend_keys::{BlendKeyEntry, BlendKeyPick, BlendKeys, BlendShortcut};
pub use blur_pins::{
    BlurKind, BlurPin, BlurPins, BlurPinsLabels, BlurPinsOverlay, BlurPinsPanel, BlurPinsPanelOutput, BlurPinsResponse, BlurPinsStyle, IrisShape,
    TiltBand,
};
pub use channels::{
    ChannelKind, ChannelRow, ChannelsCommand, ChannelsEvent, ChannelsFooterButton, ChannelsLabels, ChannelsOutput, ChannelsPanel, ChannelsStyle,
    ColorIndicates, OVERLAY_OPACITIES, OverlayPreset, targeted_row,
};
pub use fill::{
    ContentAwareFillWorkspace, FillBar, FillBarOutput, FillEvent, FillLabels, FillOpen, FillOutput, FillOverlay, FillPanel, FillPanelOutput,
    FillSession, FillSettings, FillSource, FillTool,
};
pub use filter_gallery::{
    EffectParameters, EffectStack, FilterCatalog, FilterGalleryEvent, FilterGalleryLabels, FilterGalleryOpen, FilterGalleryOutput,
    FilterGallerySession, FilterGalleryWorkspace, FilterTree, GalleryFilter,
};
pub use layer_style::{
    BlendIfBar, BlendIfBarLabels, BlendIfBarStyle, ContourField, ContourFieldLabels, ContourFieldResponse, ContourFieldStyle, LayerStyleDialog,
    LayerStyleDialogStyle, LayerStyleLabels, LayerStyleOutput, StylePresets,
};
pub use liquify::{
    LiquifyEvent, LiquifyLabels, LiquifyOpen, LiquifyOutput, LiquifyPanel, LiquifyPanelOutput, LiquifySession, LiquifySettings, LiquifySource,
    LiquifyStyle, LiquifyTool, LiquifyWorkspace,
};
pub use live_preview::{DraftScale, LivePreview, PreviewKey, PreviewLane, PreviewSession, ProxyPlan};
pub use mask_view::{MaskViewControls, MaskViewKeys, MaskViewMode, MaskViewRecipe, MaskViewResponse, MaskViewState};
pub use scopes::{
    COLORS_CHANNEL, ChannelOverlay, ClippingDiagnostic, ClippingFrame, ClippingMode, ClippingOverlay, ClippingWarnings, HistogramSet, ScopeChannel,
    ScopeFrame, ScopeMenuAction, ScopePanel, ScopePanelLabels, ScopePanelOutput, ScopePanelState, ScopePanelStyle, ScopeReadout, ScopeSettings,
    ScopeSource, ScopeView, ScopeZoneEdit, ToneZone, VectorscopeFrame, VectorscopePlot, ZoneChange, scope_point,
};
pub use select::{
    LassoTool, MagneticLassoTool, MagneticSettings, MarqueeKind, MarqueeTool, PolygonLassoTool, SelectEvent, SelectToolStyle, SelectionShape,
};
pub use select_mask::{
    SelectMaskBar, SelectMaskBarOutput, SelectMaskEdit, SelectMaskEvent, SelectMaskLabels, SelectMaskOpen, SelectMaskOutput, SelectMaskPanel,
    SelectMaskPanelOutput, SelectMaskSession, SelectMaskSettings, SelectMaskSource, SelectMaskWorkspace,
};
pub use selection::{OutlineSource, SelectionOverlay, SelectionOverlayLabels, SelectionOverlayResponse};
pub use transform_preview::{PreviewMap, PreviewQuality, StyledPreview, TransformPreview};
pub use vanishing_point::{
    VanishingPlane, VanishingPointBar, VanishingPointBarOutput, VanishingPointEditor, VanishingPointEvent, VanishingPointLabels, VanishingPointOpen,
    VanishingPointOutput, VanishingPointSession, VanishingPointSettings, VanishingPointSource, VanishingPointStyle, VanishingPointWorkspace,
    VanishingScene,
};
pub use warp::perspective::{
    PerspectiveBar, PerspectiveBarLabels, PerspectiveBarOutput, PerspectiveEdit, PerspectiveSession, PerspectiveWarp, PerspectiveWarpLabels,
    PerspectiveWarpResponse,
};
pub use warp::pin::{Pin, PinWarp, PinWarpBar, PinWarpBarLabels, PinWarpBarOutput, PinWarpLabels, PinWarpResponse, PinWarpSession, PinWarpSettings};
pub use warp::{
    MeshStyle, PerspectiveWarpStyle, PinWarpStyle, WarpFieldValues, WarpFields, WarpFieldsEdit, WarpFieldsLabels, WarpFieldsOutput, WarpGridChoice,
    WarpStyleEntry, paint_mesh,
};
pub use warp::{
    WarpEdit, WarpGrid, WarpMesh, WarpNames, WarpPress, WarpRelease, WarpResponse, WarpSide, WarpSplit, WarpSplitAt, WarpSplitPress, WarpStyle,
    WarpTarget, WarpTrack,
};
pub use wide_angle::{
    WideAngleConstraint, WideAngleEvent, WideAngleLabels, WideAngleModel, WideAngleOpen, WideAngleOutput, WideAnglePanel, WideAnglePanelOutput,
    WideAngleSession, WideAngleSource, WideAngleStyle, WideAngleTool, WideAngleWorkspace,
};

craft_ui::story::register! {
    warp::story,
    adjust,
    channels::stories,
    scopes::stories,
    transform_preview::stories,
    warp::fields_stories,
    warp::pin::stories,
    warp::perspective::stories,
    select::stories,
    selection::stories,
    mask_view::stories,
    live_preview::stories,
    layer_style::stories,
    blend_keys_stories,
    liquify::stories,
    select_mask::stories,
    fill::stories,
    filter_gallery::stories,
    wide_angle::stories,
    vanishing_point::stories,
    blur_pins::stories,
}

/// The stories of the imaging widgets.
pub fn stories() -> Vec<craft_ui::story::Story> {
    registry()
}

/// The count words of the imaging widgets: their plural forms are in [`WORDS`], these give the
/// singular forms a catalog extraction needs.
pub const COUNTS: &[craft_ui::localized::words::Counted] = &[select::OUTLINE_POINTS, warp::perspective::PLANES, wide_angle::CONSTRAINTS];

/// The plural forms of [`COUNTS`], as a words table.
const COUNT_WORDS: &[craft_ui::localized::Word] = &[select::OUTLINE_POINTS.word, warp::perspective::PLANES.word, wide_angle::CONSTRAINTS.word];

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
    <select::stories::SelectToolStoryLabels as craft_ui::localized::Labels>::WORDS,
    <vanishing_point::VanishingPointLabels as craft_ui::localized::Labels>::WORDS,
    <vanishing_point::stories::VanishingPointStoryLabels as craft_ui::localized::Labels>::WORDS,
    <select_mask::SelectMaskLabels as craft_ui::localized::Labels>::WORDS,
    <select_mask::stories::SelectMaskStoryLabels as craft_ui::localized::Labels>::WORDS,
    <selection::stories::SelectionStoryLabels as craft_ui::localized::Labels>::WORDS,
    <selection::SelectionOverlayLabels as craft_ui::localized::Labels>::WORDS,
    <transform_preview::stories::TransformPreviewStoryLabels as craft_ui::localized::Labels>::WORDS,
];
