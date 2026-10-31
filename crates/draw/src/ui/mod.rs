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

pub mod crop;
pub mod gradient;
pub mod pen;
pub mod transform;

#[cfg(test)]
mod words_tests;

pub use crop::{
    CropBox, CropFrame, CropGuide, CropLabels, CropNames, CropPress, CropPressKind, CropRelease, CropResponse, CropShield, CropStyle, CropTrack,
    GuideShow, PerspectiveCropNames, PerspectiveCropQuad, PerspectiveCropResponse, PerspectiveCropStyle, PerspectiveHit, PerspectiveQuad, Polyline,
    ShieldColor,
};
pub use gradient::{
    ColorStop, GradientEditor, GradientEditorResponse, GradientEditorStyle, GradientExtent, GradientGroup, GradientItem, GradientLabels,
    GradientLine, GradientLineNames, GradientLineStyle, GradientOverlay, GradientOverlayResponse, GradientPalette, GradientRamp, GradientRampStyle,
    GradientRef, GradientStops, GradientSwatch, LineCursor, LineGradient, LinePart, LinePress, LineRelease, LineTrack, OpacityStop, PaletteLayout,
    Sampler as StopSampler, StopDrop, StopRef, StopRow, StopRows, StopRules, default_sampler,
};
pub use pen::{
    BezierHit, BezierNames, BezierOverlay, BezierResponse, BezierStyle, BezierTool, EditPath, EditSubpath, HandleSide, HandlesShown, Knot, KnotRef,
    MAX_ANCHORS, PathEdit, PathSelection, PenDraft, PenNames, PenResponse, PenStyle,
};
pub use transform::{
    BoxPress, BoxRelease, BoxTrack, FieldLabel, RotateZone, SizeUnit, TransformBox, TransformBoxResponse, TransformFieldLabels, TransformFields,
    TransformLabels, TransformNames, TransformRow, TransformStyle, paint_pivot,
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
];
