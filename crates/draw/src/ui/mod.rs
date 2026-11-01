//! The draw tools' widgets, for egui.
//!
//! - [`gradient`]: the gradient stop bar ([`GradientEditor`]), its palette and swatch, the stop
//!   rules ([`gradient::stops`]), the on-canvas [`GradientOverlay`] with the geometry of its
//!   line ([`gradient::gradient_line`]) and the [`GradientMapEditor`].
//! - [`pen`]: the [`PenDraft`] of the Pen tool and the [`BezierOverlay`] that edits paths, with the
//!   path model they share ([`EditPath`]), and the [`PathOptionsPopover`].
//! - [`stroke`]: the [`StrokeEditor`], the [`StrokeButton`] that opens it and the [`StrokePreview`].
//! - [`corner`]: the [`CornerWidgets`] for corner radii.
//! - [`crop`]: the [`CropFrame`] of the Crop tool, its composition guides, the [`PerspectiveCropQuad`],
//!   the [`CropTool`] with its [`CropSession`], and the [`CropOptionsBar`].
//! - [`transform`]: the [`TransformBox`] of Free Transform, the options bar's [`TransformFields`],
//!   the [`TransformSession`] with its [`TransformOverlay`] and the [`TransformBar`].
//! - [`measure`]: the [`RulerTool`] with its [`RulerBar`], and [`box_distances`] between boxes.
//! - [`symmetry`]: the [`Symmetry`] guides with their [`SymmetryOverlay`], [`SymmetryMenu`] and
//!   the [`SymmetryTransform`] that mirrors a stroke.
//! - [`shape`] and [`shape_drag`]: the shape layer's [`ShapePanel`] and [`ShapeOptionsBar`]; the
//!   [`ShapeDragTool`] and its [`CreateShapeDialog`].
//! - [`mod@slice`]: the [`SliceTool`], its [`SliceOverlay`], [`SliceBar`] and dialogs.
//! - [`artboard`] and [`add_beside`]: the [`ArtboardTool`], its [`ArtboardBar`] and
//!   [`ArtboardProperties`]; the add-beside buttons ([`AddBesideHandles`]).
//! - [`connector`]: the [`ConnectorTool`], its [`ConnectorOverlay`] and [`ConnectorBar`].
//! - [`pattern`] and [`picture_fill`]: the [`PatternFillEditor`] and [`PictureFillEditor`].
//! - [`brush_presets`] and [`brush_settings`]: the [`BrushPresetPicker`], [`BrushPresetChip`] and
//!   [`BrushSettingsPanel`].
//!
//! The main types of each module are also re-exported here.

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

pub use add_beside::{AddBesideHandles, AddBesideLabels, AddBesideResponse, AddBesideStyle, AddSide};
pub use artboard::{
    ArtboardBackground, ArtboardBar, ArtboardBarOutput, ArtboardEvent, ArtboardItem, ArtboardLabels, ArtboardPreset, ArtboardProperties,
    ArtboardPropertiesOutput, ArtboardRules, ArtboardStyle, ArtboardTool, ArtboardToolLabels,
};
pub use brush_presets::{
    BrushCardLabels, BrushCardStyle, BrushPick, BrushPickerLabels, BrushPickerSettings, BrushPickerState, BrushPickerStyle, BrushPickerValues,
    BrushPresetCards, BrushPresetChip, BrushPresetChipResponse, BrushPresetChipStyle, BrushPresetPicker, BrushPresetPickerOutput,
};
pub use brush_settings::{
    BrushPanelAction, BrushSection, BrushSectionEvent, BrushSectionKind, BrushSectionList, BrushSettingsLabels, BrushSettingsOutput,
    BrushSettingsPanel, BrushSettingsStyle, BrushSizeField, BrushStrokeStrip, DynamicControl, TipEllipse,
};
pub use connector::{
    ConnectionSite, ConnectionTarget, ConnectorBar, ConnectorBarEvent, ConnectorBarLabels, ConnectorBarOutput, ConnectorDraft, ConnectorEnd,
    ConnectorEndKind, ConnectorEvent, ConnectorItem, ConnectorLine, ConnectorOverlay, ConnectorStyle, ConnectorTool, ConnectorToolLabels,
};
pub use corner::{CornerEdit, CornerKind, CornerRadii, CornerStyle, CornerWidgets, CornerWidgetsLabels, CornerWidgetsResponse};
pub use crop::{
    CropBarEvent, CropBarLabels, CropBarOutput, CropBarStyle, CropCommit, CropDocument, CropEvent, CropFill, CropInput, CropOptionsBar,
    CropOptionsMenu, CropOptionsMenuResponse, CropOutput, CropOverlayMenu, CropPresetChoice, CropRatio, CropSession, CropSettings, CropSizePreset,
    CropTarget, CropTool, CropToolPass, CropUserPreset, ResolutionUnit,
};
pub use crop::{
    CropBox, CropFrame, CropGuide, CropLabels, CropNames, CropPress, CropPressKind, CropReadout, CropRelease, CropResponse, CropShield, CropStyle,
    CropTrack, GuideShow, PerspectiveCropNames, PerspectiveCropQuad, PerspectiveCropResponse, PerspectiveCropStyle, PerspectiveHit, PerspectiveQuad,
    Polyline, ShieldColor,
};
pub use gradient::{
    ColorRequest, ColorStop, GradientEditor, GradientEditorResponse, GradientEditorStyle, GradientExtent, GradientGroup, GradientItem,
    GradientLabels, GradientLine, GradientLineNames, GradientLineStyle, GradientMap, GradientMapEditor, GradientMapLabels, GradientMapOutput,
    GradientOverlay, GradientOverlayResponse, GradientPalette, GradientRamp, GradientRampStyle, GradientRef, GradientStops, GradientSwatch,
    LineCursor, LineGradient, LinePart, LinePress, LineRelease, LineTrack, OpacityStop, PaletteLayout, Sampler as StopSampler, StopDrop, StopRef,
    StopRow, StopRows, StopRules, TaggedStop, default_sampler,
};
pub use measure::{MIN_LINE, MeasureStyle, RulerBar, RulerBarLabels, RulerBarOutput, RulerLine, RulerTool, RulerToolLabels, box_distances};
pub use pattern::{PatternColors, PatternFillDialog, PatternFillEditor, PatternFillLabels, PatternFillOptions, PatternSwatch};
pub use pen::{
    AnchorsShown, HitMark, PATH_OPTIONS_COLOR_KEY, PathMode, PathOptionsLabels, PathOptionsPopover, PathOptionsPopoverResponse, PathOptionsSettings,
    PathTool, PenMode, SegmentMark,
};
pub use pen::{
    BezierHit, BezierNames, BezierOverlay, BezierResponse, BezierStyle, BezierTool, EditPath, EditSubpath, HandleSide, HandlesShown, Knot, KnotRef,
    MAX_ANCHORS, PathEdit, PathSelection, PenDraft, PenNames, PenResponse, PenStyle,
};
pub use picture_fill::{PictureFill, PictureFillEditor, PictureFillEvent, PictureFillLabels, PictureFillOutput, PictureMode};
pub use shape::{
    LiveShape, ShapeAppearance, ShapeChange, ShapeLabels, ShapeOptionsBar, ShapeOptionsOutput, ShapePaint, ShapePanel, ShapePanelOutput,
    ShapeToolSettings,
};
pub use shape_drag::{
    CreateShapeDialog, CreateShapeLabels, CreateShapeOptions, CreatedShape, ShapeDragEvent, ShapeDragOptions, ShapeDragStyle, ShapeDragTool,
    ShapeDragToolLabels, ShapeKind,
};
pub use slice::{
    DivideBy, DivideSlices, DivideSlicesDialog, SliceBar, SliceBarOutput, SliceCommand, SliceEvent, SliceItem, SliceKind, SliceLabels, SliceMode,
    SliceOptions, SliceOptionsDialog, SliceOverlay, SliceStyle, SliceTool, SliceToolLabels, SliceToolSettings,
};
pub use stroke::{
    DashUnit, LineCap, LineJoin, LineStyle, StrokeAlign, StrokeButton, StrokeButtonResponse, StrokeEditor, StrokeEditorResponse, StrokeEditorStyle,
    StrokeLabels, StrokePreview, StrokeStyle,
};
pub use symmetry::{
    Symmetry, SymmetryChoice, SymmetryCommand, SymmetryEvent, SymmetryLabels, SymmetryMenu, SymmetryMenuResponse, SymmetryMode, SymmetryOverlay,
    SymmetryStyle, SymmetryTransform,
};
pub use transform::{
    BarGroups, BoxPress, BoxRelease, BoxTrack, FieldLabel, ParseContext, RotateZone, SizeUnit, TransformBox, TransformBoxResponse, TransformField,
    TransformFieldLabels, TransformFields, TransformLabels, TransformNames, TransformRow, TransformStyle, paint_pivot,
};
pub use transform::{
    Interpolation, TransformBar, TransformBarLabels, TransformBarOutput, TransformChoice, TransformEdit, TransformMenuLabels, TransformOverlay,
    TransformOverlayResponse, TransformPhase, TransformPreset, TransformRequest, TransformResult, TransformSession, TransformSettings, TransformWarp,
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
    transform::session_stories,
    stroke::stories,
    corner::stories,
    crop::stories_tool,
    crop::stories_bar,
    measure::stories,
    symmetry::stories,
    brush_presets::stories,
    brush_settings::stories,
    shape::stories,
    shape_drag::stories,
    slice::stories,
    artboard::stories,
    add_beside_stories,
    pattern::stories,
    picture_fill::stories,
    connector::stories,
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
    <crop::stories_tool::CropStoryLabels as craft_ui::localized::Labels>::WORDS,
    <stroke::StrokeLabels as craft_ui::localized::Labels>::WORDS,
    <stroke::stories::StrokeStoryLabels as craft_ui::localized::Labels>::WORDS,
    <brush_settings::BrushSettingsLabels as craft_ui::localized::Labels>::WORDS,
    <brush_settings::stories::BrushSettingsStoryLabels as craft_ui::localized::Labels>::WORDS,
    <pen::PathOptionsLabels as craft_ui::localized::Labels>::WORDS,
    <add_beside_stories::AddBesideStoryLabels as craft_ui::localized::Labels>::WORDS,
    <measure::RulerToolLabels as craft_ui::localized::Labels>::WORDS,
    <measure::RulerBarLabels as craft_ui::localized::Labels>::WORDS,
    <measure::stories::MeasureStoryLabels as craft_ui::localized::Labels>::WORDS,
    <symmetry::stories::SymmetryStoryLabels as craft_ui::localized::Labels>::WORDS,
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
