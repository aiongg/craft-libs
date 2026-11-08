//! The draw tools' widgets, for egui.
//!
//! - [`gradient`]: the gradient stop bar ([`GradientEditor`]), its palette and swatch, the stop
//!   rules ([`gradient::stops`]), the on-canvas [`GradientOverlay`] with the geometry of its
//!   line ([`gradient::gradient_line`]) and the [`GradientMapEditor`].
//! - [`pen`]: the [`PenDraft`] of the Pen tool and the [`BezierOverlay`] that edits paths, with the
//!   path model they share ([`EditPath`]), and the [`PathOptionsPopover`].
//! - [`stroke`]: the [`StrokeEditor`], the [`StrokeButton`] that opens it and the [`StrokePreview`].
//! - [`corner`]: the [`CornerWidgets`] for corner radii.
//! - [`crop`]: the [`PerspectiveCropQuad`], the [`CropTool`] with its [`CropSession`], and the [`CropBar`].
//! - [`transform`]: the [`TransformBox`] of Free Transform, the options bar's [`TransformFields`],
//!   the [`TransformSession`] with its [`TransformOverlay`] and the [`TransformBar`].
//! - [`measure`]: the [`RulerTool`] with its [`RulerBar`], and [`box_distances`] between boxes.
//! - [`symmetry`]: the [`Symmetry`] guides with their [`SymmetryOverlay`], [`SymmetryMenu`] and
//!   the [`SymmetryTransform`] that mirrors a stroke.
//! - [`shape`] and [`shape_drag`]: the shape layer's [`ShapePanel`] and [`ShapeBar`]; the
//!   [`ShapeDragTool`] and its [`CreateShapeDialog`].
//! - [`mod@slice`]: the [`SliceTool`], its [`SliceOverlay`], [`SliceBar`] and dialogs.
//! - [`artboard`] and [`add_beside`]: the [`ArtboardTool`], its [`ArtboardBar`] and
//!   [`ArtboardProperties`]; the add-beside buttons ([`AddBesideHandles`], a canvas control).
//! - [`connector`]: the [`ConnectorTool`], its [`ConnectorOverlay`] and [`ConnectorBar`].
//! - [`pattern`] and [`picture_fill`]: the [`PatternFillEditor`] and [`PictureFillEditor`].
//! - [`brush_presets`] and [`brush_settings`]: the [`BrushPresetPicker`], [`BrushPresetChip`] and
//!   [`BrushSettingsPanel`].
//!
//! The main types of each module are also re-exported here.

pub mod add_beside;
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

pub use add_beside::{AddBesideHandles, AddBesideOutput, AddBesideWords, AddSide, add_beside_places};
pub use artboard::{
    ArtboardBackground, ArtboardBar, ArtboardBarOutput, ArtboardEvent, ArtboardItem, ArtboardLabels, ArtboardPreset, ArtboardProperties,
    ArtboardPropertiesOutput, ArtboardRules, ArtboardStyle, ArtboardTool, ArtboardToolLabels,
};
pub use brush_presets::{
    BrushCardLabels, BrushCardStyle, BrushPick, BrushPickerLabels, BrushPickerSettings, BrushPickerState, BrushPickerStyle, BrushPickerValues,
    BrushPresetCards, BrushPresetChip, BrushPresetChipResponse, BrushPresetChipStyle, BrushPresetPicker, BrushPresetPickerOutput,
};
pub use brush_settings::{
    BrushPanelAction, BrushSection, BrushSectionEvent, BrushSectionKind, BrushSectionList, BrushSettingsLabels, BrushSettingsPanel,
    BrushSettingsPanelOutput, BrushSettingsStyle, BrushSizeField, BrushStrokeStrip, DynamicControl, TipEllipse,
};
pub use connector::{
    ConnectionSite, ConnectionTarget, ConnectorBar, ConnectorBarLabels, ConnectorBarOutput, ConnectorDraft, ConnectorEnd, ConnectorEndKind,
    ConnectorEvent, ConnectorItem, ConnectorLine, ConnectorOverlay, ConnectorStyle, ConnectorTool, ConnectorToolLabels,
};
pub use corner::{CornerEdit, CornerKind, CornerRadii, CornerStyle, CornerWidgets, CornerWidgetsLabels, CornerWidgetsResponse};
pub use crop::{
    CropBar, CropBarOutput, CropBarStyle, CropBarWords, CropCommit, CropDocument, CropEvent, CropFill, CropInput, CropOptionsMenu,
    CropOptionsMenuResponse, CropOutput, CropOverlayMenu, CropPresetChoice, CropRatio, CropRatioChange, CropSession, CropSettings, CropSizePreset,
    CropTarget, CropTool, CropToolPass, CropUserPreset, CropWords, PerspectiveCropOutput, PerspectiveCropTool, ResolutionUnit,
};
pub use crop::{PerspectiveCropQuad, PerspectiveQuad, Polyline, ShieldColor};
pub use gradient::{
    ColorRequest, ColorStop, GradientEditor, GradientEditorResponse, GradientEditorStyle, GradientExtent, GradientGroup, GradientItem,
    GradientLabels, GradientLine, GradientLineNames, GradientLineStyle, GradientMap, GradientMapEditor, GradientMapLabels, GradientMapOutput,
    GradientOverlay, GradientOverlayResponse, GradientPalette, GradientRamp, GradientRampStyle, GradientRef, GradientStops, GradientSwatch,
    LineCursor, LineGradient, LinePart, LinePress, LineRelease, LineTrack, OpacityStop, PaletteLayout, Sampler as StopSampler, StopDrop, StopRef,
    StopRow, StopRows, StopRules, TaggedStop, default_sampler,
};
pub use measure::{MIN_LINE, RulerBar, RulerBarOutput, RulerLine, RulerOverlay, RulerPass, RulerTool, RulerWords, box_distances, ruler_line_form};
pub use pattern::{
    PatternColors, PatternFillDialog, PatternFillEditor, PatternFillOptions, PatternFillOutput, PatternSwatch, PatternWords, SwatchPainter,
};
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
    ChipSetup, LiveShape, ShapeAppearance, ShapeBar, ShapeBarOutput, ShapeChange, ShapePaint, ShapePanel, ShapePanelOutput, ShapeToolSettings,
    ShapeWords,
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
