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
//! use craft_theme::{Appearance, Contrast, Mode, Size, TextSize, builtins, resolve};
//!
//! let theme = builtins().iter().find(|t| t.id.as_str() == "craft.graphite").unwrap();
//! let appearance = Appearance { mode: Mode::Dark, contrast: Contrast::Standard, size: Size::Compact, text_size: TextSize::Default, reduced_motion: false };
//! let resolved = resolve(theme, appearance, None);
//! assert!(resolved.warnings.is_empty());
//! ```

mod builtins;
pub mod color;
mod color_model;
mod components;
pub mod contrast;
mod derive;
mod explain;
pub mod format;
mod id;
pub mod ramp;
mod resolve;
pub mod scale;
pub mod selection;
mod theme;
mod tokens;
mod values;

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
mod selection_tests;
#[cfg(test)]
mod semantic_tests;
#[cfg(test)]
mod tests;

pub use builtins::builtins;
pub use color::{Hsb, Lab, Oklab, Oklch, Rgba, Srgba};
pub use color_model::{
    Channel, ColorModel, DefaultOn, HsbModel, LabModel, MAX_CHANNELS, ModelValues, OklchModel, RgbModel, builtin_models, kept_or_converted,
};
pub use components::{
    BadgeColors, BadgeField, BadgePart, BadgeStyle, BadgeTokens, BadgeTone, BorderColors, BorderState, ButtonColors, ButtonField, ButtonPart,
    ButtonTokens, ButtonVariant, BySize, CheckField, CheckOffColors, CheckOffPart, CheckOnColors, CheckOnPart, CheckTokens, ColorPickerField,
    ColorPickerTokens, ComponentField, ComponentTokens, ControlSize, CurveField, CurveTokens, Dimmed, EmptyStateField, EmptyStateTokens, FieldField,
    FieldKind, FieldPart, FieldTokens, GradientEditorField, GradientEditorTokens, GuideField, GuideTokens, HandleField, HandleTokens, HistogramField,
    HistogramTokens, IconButtonField, IconButtonTokens, ItemColors, ItemPart, KbdField, KbdTokens, ListField, ListTokens, ListboxField,
    ListboxTokens, MarqueeField, MarqueeTokens, MenuField, MenuTokens, NoticeField, NoticeTokens, OverrideError, OverrideValue, PanelHeaderField,
    PanelHeaderTokens, ProgressField, ProgressTokens, PropertyField, PropertyTokens, ReadoutField, ReadoutTokens, RulerField, RulerTokens,
    SectionField, SectionTokens, SegmentedField, SegmentedTokens, SliderField, SliderTokens, State, States, SwatchColor, SwatchField, SwatchTokens,
    ThumbColors, ThumbPart, ToastField, ToastTokens, ToggleButtonField, ToggleButtonTokens, Tone, TooltipField, TooltipTokens,
};
pub use contrast::{contrast_ratio, ensure_contrast, relative_luminance};
pub use derive::{Anchoring, ContrastCheck, PassMove, Ramps};
pub use explain::{ColorOrigin, Explained, FieldOrigin, PairReport, explain};
pub use id::{APP_DEFAULT, AppId, IdError, RESERVED_PREFIX, SUITE_DEFAULT, ThemeId};
pub use ramp::{Mode, Ramp, RampKind};
pub use resolve::{Resolved, resolve};
pub use scale::{Duration, Easing, Elevation, Opacity, Radius, Space, Stroke};
pub use selection::{
    AppChoice, AppDefaults, ContrastChoice, Fallback, ModeChoice, Selection, Shown, SuiteChoice, SystemAppearance, ThemePair, resolve_selection,
};
pub use theme::{
    AppOverrides, Appearance, Contrast, ContrastIntent, EdgeSeed, InterfaceFont, ModePins, ModeSeeds, PinLayer, Pins, SeedOverrides, Seeds,
    ShapeSeed, Size, TextSize, ThemeDef, Variants,
};
pub use tokens::{
    AccentColors, BarSizes, CaptionColors, ColorToken, ColorTokens, ControlSizes, DataColors, Edge, EdgeColors, ElevationTokens, FontSlot, Gaps,
    IconSizes, LineColors, ListSizes, Motion, MotionTokens, OverlayColors, Pads, RadiusSet, RailSizes, ShapeTokens, SizeTokens, SpaceTokens,
    StateColors, Status, StatusColors, StatusKind, SurfaceColors, TextColors, ThumbSizes, TokenRef, Tokens, TypeRole, TypeStyle, TypeTokens,
};
pub use values::ColorSource;
