//! The v2 token model: primitives (sRGB and OKLCH colour, WCAG contrast, 12-step colour ramps,
//! fixed scales), the semantic layer built on them (themes as seeds and pins, derivation with a
//! contrast pass, Size presets, type roles and [`resolve`]) and the component layer built on
//! that (one token struct per component family, with curated overrides).
//!
//! Pure data and functions, like the rest of the crate. This module replaces the pilot model at
//! the crate root once the rest of the design system lands.

pub mod color;
mod components;
pub mod contrast;
mod derive;
pub mod format;
pub mod ramp;
mod resolve;
pub mod scale;
mod theme;
mod tokens;
mod values;

#[cfg(test)]
mod component_tests;
#[cfg(test)]
mod format_tests;
#[cfg(test)]
mod semantic_tests;
#[cfg(test)]
mod tests;

pub use color::{Oklab, Oklch, Rgba};
pub use components::{
    ButtonColors, ButtonField, ButtonPart, ButtonTokens, ButtonVariant, BySize, ComponentField, ComponentTokens, ControlSize, FieldKind,
    OverrideError, OverrideValue, State, States,
};
pub use contrast::{contrast_ratio, ensure_contrast, relative_luminance};
pub use derive::ContrastWarning;
pub use ramp::{Mode, Ramp, RampKind};
pub use resolve::{Resolved, resolve};
pub use scale::{Duration, Easing, Elevation, Opacity, Radius, Space, Stroke};
pub use theme::{AppOverrides, Appearance, Contrast, ContrastIntent, EdgeSeed, ModePins, Pins, Seeds, ShapeSeed, Size, TextSize, ThemeDef, Variants};
pub use tokens::{
    AccentColors, BarSizes, CaptionColors, ColorToken, ColorTokens, ControlSizes, DataColors, Edge, EdgeColors, ElevationTokens, FontSlot, Gaps,
    IconSizes, LineColors, Motion, MotionTokens, OverlayColors, Pads, RadiusSet, RailSizes, ShapeTokens, SizeTokens, SpaceTokens, StateColors,
    Status, StatusColors, StatusKind, SurfaceColors, TextColors, TokenRef, Tokens, TypeRole, TypeStyle, TypeTokens,
};
