#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]
//! The Craft design system's theme model: primitives (sRGB and OKLCH colour, WCAG contrast,
//! 12-step colour ramps, fixed scales), the semantic layer built on them (themes as seeds and
//! pins, derivation with a contrast pass, Size presets, type roles and [`resolve`]), the
//! component layer built on that (one token struct per component family, with curated
//! overrides), the user's selection with its resolution ([`resolve_selection`]), and
//! [`explain`], which reports how each colour got its value and every contrast pair measured.
//!
//! Pure data and functions: no egui, no I/O, no threads. Theme files on disk, the suite folder
//! and change watching belong to `craft-theme-store`; `craft_ui::theme` turns tokens into egui
//! ones.
//!
//! ```
//! use craft_theme::{Appearance, Contrast, Mode, Size, TextSize, format, resolve};
//!
//! let file = br##"{"schema": "craft-design-theme", "schema_version": 1, "id": "example",
//!     "seeds": {"neutral": "#8b8d98", "accent": "#3e63dd",
//!               "status": {"good": "#30a46c", "warn": "#ffc53d", "bad": "#e5484d", "info": "#0090ff"}}}"##;
//! let theme = format::parse(file).unwrap().theme;
//! let appearance = Appearance { mode: Mode::Dark, contrast: Contrast::Standard, size: Size::Compact, text_size: TextSize::Small, reduced_motion: false };
//! let resolved = resolve(&theme, appearance, None);
//! assert!(resolved.warnings.is_empty());
//! ```

mod app_sizes;
mod builtins;
pub mod color;
mod color_model;
mod components;
pub mod contrast;
mod derive;
pub mod dump;
mod explain;
pub mod format;
mod id;
pub mod ramp;
pub mod report;
mod resolve;
pub mod scale;
pub mod selection;
pub mod sizes;
mod theme;
mod tokens;
mod values;

#[cfg(test)]
mod app_sizes_tests;
#[cfg(test)]
mod builtin_tests;
#[cfg(test)]
mod color_model_tests;
#[cfg(test)]
mod component_tests;
#[cfg(test)]
mod explain_tests;
#[cfg(test)]
mod format_tests;
#[cfg(test)]
mod report_tests;
#[cfg(test)]
mod selection_tests;
#[cfg(test)]
mod semantic_tests;
#[cfg(test)]
mod sizes_tests;
#[cfg(test)]
mod tests;

pub use app_sizes::{AppSizes, SizeMap, SizeValue};
pub use builtins::builtins;
pub use color::{Hsb, Lab, Oklab, Oklch, Rgba, Srgba};
pub use color_model::{
    Channel, ColorModel, DefaultOn, HsbModel, LabModel, MAX_CHANNELS, ModelValues, OklchModel, RgbModel, builtin_models, kept_or_converted,
};
pub use components::{
    AboutField, AboutTokens, ActionsField, ActionsTokens, AddBesideField, AddBesideTokens, AdjustField, AdjustTokens, AlignField, AlignTokens,
    AltTextField, AltTextTokens, AngleDialField, AngleDialTokens, AppShellField, AppShellTokens, ArtboardField, ArtboardTokens, AutocompleteField,
    AutocompleteTokens, BadgeColors, BadgeField, BadgePart, BadgeStyle, BadgeTokens, BadgeTone, BeforeAfterField, BeforeAfterTokens, BezierField,
    BezierTokens, BlendIfField, BlendIfTokens, BlurPinsField, BlurPinsTokens, BorderColors, BorderState, BreadcrumbField, BreadcrumbTokens,
    BrushField, BrushTokens, ButtonColors, ButtonField, ButtonPart, ButtonTokens, ButtonVariant, BySize, CanvasField, CanvasTokens, ChannelsField,
    ChannelsTokens, CheckField, CheckOffColors, CheckOffPart, CheckOnColors, CheckOnPart, CheckSpellingField, CheckSpellingTokens, CheckSwitchColors,
    CheckSwitchPart, CheckTokens, ChipField, ChipTokens, ColorPickerField, ColorPickerTokens, ColorRampField, ColorRampTokens, ColorWheelField,
    ColorWheelTokens, CommandPaletteField, CommandPaletteTokens, ComponentField, ComponentTokens, ConnectorField, ConnectorTokens, ConstrainField,
    ConstrainTokens, ControlSize, CornerField, CornerTokens, CropField, CropTokens, CursorField, CursorTokens, CurveField, CurveTokens, DateField,
    DateTokens, DialogField, DialogTokens, Dimmed, DockField, DockTokens, DragField, DragTokens, DropVeilField, DropVeilTokens, EmptyStateField,
    EmptyStateTokens, EventLogField, EventLogTokens, ExportField, ExportTokens, EyedropperField, EyedropperTokens, FieldField, FieldKind, FieldPart,
    FieldTokens, FileListField, FileListTokens, FilterGalleryField, FilterGalleryTokens, FindingsField, FindingsTokens, FloatField, FloatTokens,
    FontPickerField, FontPickerTokens, FormField, FormSpacing, FormTokens, FullWindowField, FullWindowTokens, GlyphsField, GlyphsTokens,
    GradientEditorField, GradientEditorTokens, GradientLineField, GradientLineTokens, GradientPaletteField, GradientPaletteTokens, GridField,
    GridTokens, GripField, GripTokens, GroupBoxField, GroupBoxTokens, GuideField, GuideTokens, HandleField, HandleTokens, HarmonyField,
    HarmonyTokens, HistogramField, HistogramTokens, HistoryField, HistoryTokens, HoverField, HoverTokens, IconButtonField, IconButtonTokens,
    IconPickerField, IconPickerTokens, InspectorField, InspectorTokens, ItemColors, ItemPart, KbdField, KbdTokens, LabelColor, LabelPart, LabelTint,
    LayerStyleField, LayerStyleTokens, LayersField, LayersTokens, LevelsField, LevelsTokens, LineGuideField, LineGuideTokens, LinkField, LinkTokens,
    LiquifyField, LiquifyTokens, ListField, ListManagerField, ListManagerTokens, ListTokens, ListboxField, ListboxTokens, MarqueeField,
    MarqueeTokens, MeasureField, MeasureTokens, MenuField, MenuHue, MenuTint, MenuTintPart, MenuTokens, MetadataField, MetadataTokens,
    NavigatorField, NavigatorTokens, NewDocumentField, NewDocumentTokens, NoticeField, NoticeTokens, OptionsBarField, OptionsBarTokens,
    OutlineDraftField, OutlineDraftTokens, OverrideError, OverrideValue, PaintChipField, PaintChipTokens, PanelField, PanelFooterField,
    PanelFooterTokens, PanelHeaderField, PanelHeaderTokens, PanelTokens, PasswordField, PasswordTokens, PasteboardField, PasteboardTokens,
    PopoverField, PopoverTokens, PopupField, PopupTokens, PreferencesField, PreferencesTokens, PresetField, PresetTokens, ProgressField,
    ProgressTokens, PropertyField, PropertyTokens, RailField, RailTokens, RatingField, RatingTokens, ReadoutField, ReadoutTokens, RefPointField,
    RefPointTokens, ResetDotField, ResetDotTokens, RulerField, RulerTokens, SamplerField, SamplerTokens, ScopesField, ScopesTokens,
    ScreenPickerField, ScreenPickerTokens, ScrollField, ScrollTokens, SectionField, SectionTokens, SegmentedField, SegmentedTokens, SelectField,
    SelectMaskField, SelectMaskTokens, SelectTokens, SelectionBarField, SelectionBarTokens, SelectionFrameField, SelectionFrameTokens,
    ShapeDragField, ShapeDragTokens, SizeField, SliceField, SliceTokens, SliderField, SliderRowField, SliderRowTokens, SliderTokens, SpinnerField,
    SpinnerTokens, SplitButtonField, SplitButtonTokens, SplitField, SplitTokens, StartField, StartTokens, State, States, StatusBarField,
    StatusBarTokens, StrokeEditorField, StrokeEditorTokens, SwatchColor, SwatchField, SwatchPaletteField, SwatchPaletteTokens, SwatchTokens,
    SymmetryField, SymmetryTokens, TabField, TabTokens, TableField, TableTokens, TextEditorField, TextEditorTokens, TextStylesField,
    TextStylesTokens, ThemeChipField, ThemeChipTokens, ThumbColors, ThumbPart, ThumbnailField, ThumbnailTokens, TileField, TileTokens, TitleBarField,
    TitleBarTokens, ToastField, ToastTokens, ToggleButtonField, ToggleButtonTokens, Tone, ToolPillField, ToolPillTokens, ToolbarField, ToolbarTokens,
    TooltipField, TooltipTokens, TourField, TourTokens, TransformField, TransformTokens, TreePickerField, TreePickerTokens, VanishingPointField,
    VanishingPointTokens, WideAngleField, WideAngleTokens,
};
pub use contrast::{contrast_ratio, ensure_contrast, relative_luminance};
pub use derive::{Anchoring, ContrastCheck, PassMove, Ramps};
pub use explain::{ColorOrigin, Explained, FieldOrigin, PairReport, SizeLayer, SizeOrigin, explain, explain_sized};
pub use id::{APP_DEFAULT, AppId, IdError, RESERVED_PREFIX, SUITE_DEFAULT, ThemeId};
pub use ramp::{Mode, Ramp, RampKind};
pub use resolve::{Resolved, resolve, resolve_sized};
pub use scale::{Duration, Easing, Elevation, Opacity, Radius, Space, Stroke};
pub use selection::{
    AppChoice, AppDefaults, ContrastChoice, Fallback, ModeChoice, MotionChoice, Selection, Shown, SuiteChoice, SystemAppearance, ThemePair,
    resolve_selection,
};
pub use theme::{
    AppOverrides, Appearance, Contrast, ContrastIntent, EdgeSeed, InterfaceFont, ModePins, ModeSeeds, PinLayer, Pins, RadiusOffset, SeedOverrides,
    Seeds, ShapeSeed, Size, TextSize, TextSizeSteps, ThemeDef, Variants,
};
pub use tokens::{
    AccentColors, BarSizes, CaptionColors, ColorToken, ColorTokens, ControlSizes, DataColors, Edge, EdgeColors, ElevationTokens, FontSlot, Gaps,
    IconSizes, LineColors, ListSizes, Motion, MotionTokens, OverlayColors, Pads, PresetSizes, RadiusSet, RailSizes, ShapeTokens, SizeTokens,
    SpaceTokens, StateColors, Status, StatusColors, StatusKind, SurfaceColors, TabSizes, TextColors, ThumbSizes, TokenRef, Tokens, TypeRole,
    TypeStyle, TypeTokens,
};
pub use values::ColorSource;
