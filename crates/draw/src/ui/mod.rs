//! The draw tools' widgets, for egui.
//!
//! - [`gradient`]: the gradient stop bar ([`GradientEditor`]), its palette and swatch, the stop
//!   rules ([`gradient::stops`]) and the on-canvas [`GradientOverlay`] with the geometry of its
//!   line ([`gradient::gradient_line`]).
//! - [`pen`]: the [`PenDraft`] of the Pen tool and the [`BezierOverlay`] that edits paths, with the
//!   path model they share ([`EditPath`]).
//! - [`crop`]: the [`CropFrame`] of the Crop tool, its composition guides, and the
//!   [`PerspectiveCropQuad`].
//! - [`transform`]: the [`TransformBox`] of Free Transform and the options bar's
//!   [`TransformFields`].

pub mod add_beside;
pub(crate) mod add_beside_stories;
#[cfg(test)]
mod add_beside_tests;
pub mod artboard;
pub mod brush_presets;
pub mod brush_settings;
pub mod connector;
pub mod corner;
pub mod crop;
pub mod gradient;
pub mod measure;
pub mod pattern;
pub mod pen;
pub mod picture_fill;
pub mod shape;
pub mod shape_drag;
pub mod slice;
pub mod stroke;
pub mod symmetry;
pub mod transform;

#[cfg(test)]
mod words_tests;

pub use crop::{
    CropBox, CropFrame, CropGuide, CropLabels, CropNames, CropPress, CropPressKind, CropReadout, CropRelease, CropResponse, CropShield, CropStyle,
    CropTrack, GuideShow, PerspectiveCropNames, PerspectiveCropQuad, PerspectiveCropResponse, PerspectiveCropStyle, PerspectiveHit, PerspectiveQuad,
    Polyline, ShieldColor,
};
pub use gradient::{
    ColorRequest, ColorStop, GradientEditor, GradientEditorResponse, GradientEditorStyle, GradientExtent, GradientGroup, GradientItem,
    GradientLabels, GradientLine, GradientLineNames, GradientLineStyle, GradientOverlay, GradientOverlayResponse, GradientPalette, GradientRamp,
    GradientRampStyle, GradientRef, GradientStops, GradientSwatch, LineCursor, LineGradient, LinePart, LinePress, LineRelease, LineTrack,
    OpacityStop, PaletteLayout, Sampler as StopSampler, StopDrop, StopRef, StopRow, StopRows, StopRules, TaggedStop, default_sampler,
};
pub use pen::{
    BezierHit, BezierNames, BezierOverlay, BezierResponse, BezierStyle, BezierTool, EditPath, EditSubpath, HandleSide, HandlesShown, Knot, KnotRef,
    MAX_ANCHORS, PathEdit, PathSelection, PenDraft, PenNames, PenResponse, PenStyle,
};
pub use transform::{
    BarGroups, BoxPress, BoxRelease, BoxTrack, FieldLabel, ParseContext, RotateZone, SizeUnit, TransformBox, TransformBoxResponse, TransformField,
    TransformFieldLabels, TransformFields, TransformLabels, TransformNames, TransformRow, TransformStyle, paint_pivot,
};

craft_ui::story::register! {
    gradient,
    gradient::gradient_overlay::story,
    transform::transform_box,
    crop::perspective_crop,
    crop::crop_frame::story,
    pen::bezier::story,
    pen::story,
    transform::transform_fields,
}

/// The stories of the draw tools.
pub fn stories() -> Vec<craft_ui::story::Story> {
    registry()
}

/// The count words of the draw tools: their plural forms are in [`WORDS`], these give the singular
/// forms a catalog extraction needs.
pub const COUNTS: &[craft_ui::localized::words::Counted] = gradient::counts::ALL;

/// The labels tables of the draw tools, for the words registry.
pub const WORDS: &[&[craft_ui::localized::Word]] = &[
    <GradientLineNames as craft_ui::localized::Labels>::WORDS,
    <GradientLabels as craft_ui::localized::Labels>::WORDS,
    <CropNames as craft_ui::localized::Labels>::WORDS,
    <CropLabels as craft_ui::localized::Labels>::WORDS,
    <PerspectiveCropNames as craft_ui::localized::Labels>::WORDS,
    <TransformNames as craft_ui::localized::Labels>::WORDS,
    <TransformLabels as craft_ui::localized::Labels>::WORDS,
    <TransformFieldLabels as craft_ui::localized::Labels>::WORDS,
    <BezierNames<'static> as craft_ui::localized::Labels>::WORDS,
    <PenNames<'static> as craft_ui::localized::Labels>::WORDS,
    <picture_fill::PictureFillLabels as craft_ui::localized::Labels>::WORDS,
    <picture_fill::stories::PictureStoryLabels as craft_ui::localized::Labels>::WORDS,
    <gradient::map::GradientMapLabels as craft_ui::localized::Labels>::WORDS,
    <connector::ConnectorToolLabels as craft_ui::localized::Labels>::WORDS,
    <connector::ConnectorBarLabels as craft_ui::localized::Labels>::WORDS,
    <connector::stories::ConnectorStoryLabels as craft_ui::localized::Labels>::WORDS,
    <slice::SliceToolLabels as craft_ui::localized::Labels>::WORDS,
    <slice::SliceLabels as craft_ui::localized::Labels>::WORDS,
    <slice::stories::SliceStoryLabels as craft_ui::localized::Labels>::WORDS,
    <pattern::PatternFillLabels as craft_ui::localized::Labels>::WORDS,
    <pattern::stories::PatternStoryLabels as craft_ui::localized::Labels>::WORDS,
    <brush_presets::BrushPickerLabels as craft_ui::localized::Labels>::WORDS,
    <brush_presets::BrushCardLabels as craft_ui::localized::Labels>::WORDS,
    <brush_presets::stories::BrushPresetStoryLabels as craft_ui::localized::Labels>::WORDS,
    <symmetry::SymmetryLabels as craft_ui::localized::Labels>::WORDS,
    <transform::TransformBarLabels as craft_ui::localized::Labels>::WORDS,
    <transform::TransformMenuLabels as craft_ui::localized::Labels>::WORDS,
    <transform::session_stories::TransformSessionStoryLabels as craft_ui::localized::Labels>::WORDS,
    <crop::CropBarLabels as craft_ui::localized::Labels>::WORDS,
    <stroke::StrokeLabels as craft_ui::localized::Labels>::WORDS,
    <brush_settings::BrushSettingsLabels as craft_ui::localized::Labels>::WORDS,
    <brush_settings::stories::BrushSettingsStoryLabels as craft_ui::localized::Labels>::WORDS,
    <pen::PathOptionsLabels as craft_ui::localized::Labels>::WORDS,
    <add_beside_stories::AddBesideStoryLabels as craft_ui::localized::Labels>::WORDS,
    <measure::RulerToolLabels as craft_ui::localized::Labels>::WORDS,
    <measure::RulerBarLabels as craft_ui::localized::Labels>::WORDS,
    <measure::stories::MeasureStoryLabels as craft_ui::localized::Labels>::WORDS,
    <shape_drag::ShapeDragToolLabels as craft_ui::localized::Labels>::WORDS,
    <shape_drag::CreateShapeLabels as craft_ui::localized::Labels>::WORDS,
    <shape_drag::stories::ShapeDragStoryLabels as craft_ui::localized::Labels>::WORDS,
    <artboard::ArtboardToolLabels as craft_ui::localized::Labels>::WORDS,
    <artboard::ArtboardLabels as craft_ui::localized::Labels>::WORDS,
    <artboard::stories::ArtboardStoryLabels as craft_ui::localized::Labels>::WORDS,
    <shape::ShapeLabels as craft_ui::localized::Labels>::WORDS,
    <shape::stories::ShapeStoryLabels as craft_ui::localized::Labels>::WORDS,
    <add_beside::AddBesideLabels as craft_ui::localized::Labels>::WORDS,
    <corner::CornerWidgetsLabels as craft_ui::localized::Labels>::WORDS,
];
