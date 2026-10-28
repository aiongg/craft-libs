//! What a theme supplies (seeds, pins, optional shape) and what the user chooses (appearance).

use std::collections::BTreeMap;

use super::color::Rgba;
use super::components::{ComponentField, FieldKind, OverrideError, OverrideValue};
use super::ramp::Mode;
use super::tokens::{ColorTokens, Edge, RadiusSet, Status, TokenRef};
use crate::{AppId, ThemeId};

/// A theme: seeds to derive from, values that override the derivation, and an optional shape.
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeDef {
    /// Stable identity.
    pub id: ThemeId,
    /// Display name.
    pub name: String,
    /// The colours every token derives from.
    pub seeds: Seeds,
    /// Radius and edge of themes with their own shape (Classic, Breeze); `None` takes the Size
    /// preset's.
    pub shape: Option<ShapeSeed>,
    /// Which modes the theme has.
    pub variants: Variants,
    /// Fixed colours per mode, semantic and component, replacing derived ones.
    pub pins: ModePins,
    /// Fixed colours used only with [`Contrast::High`], applied over `pins`.
    pub high_contrast: ModePins,
    /// Mode-independent component overrides (radii, edges, shadows). Colours go in the pins.
    pub components: BTreeMap<ComponentField, OverrideValue>,
    /// Whether the theme is meant to have low contrast.
    pub contrast_intent: ContrastIntent,
    /// Per-app overrides, applied over the theme's own values when resolving for that app.
    pub apps: BTreeMap<AppId, AppOverrides>,
}

/// What a theme changes for one app. Applied after the theme's pins and before its
/// high-contrast pins; the components after the theme's components.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AppOverrides {
    /// Fixed colours per mode.
    pub pins: ModePins,
    /// Mode-independent component overrides.
    pub components: BTreeMap<ComponentField, OverrideValue>,
}

impl ThemeDef {
    /// Every override that resolution skips, in the theme and in its app overrides: a pin on a
    /// component field that is not a colour, a colour in `components`, or a value of the wrong
    /// kind there.
    pub fn override_errors(&self) -> Vec<OverrideError> {
        let mut out = Vec::new();
        let high = [&self.high_contrast.light, &self.high_contrast.dark];
        let own = [&self.pins.light, &self.pins.dark];
        pin_errors(own.into_iter().chain(high), &mut out);
        component_errors(&self.components, &mut out);
        for app in self.apps.values() {
            pin_errors([&app.pins.light, &app.pins.dark], &mut out);
            component_errors(&app.components, &mut out);
        }
        out
    }
}

fn pin_errors<'a>(pins: impl IntoIterator<Item = &'a Pins>, out: &mut Vec<OverrideError>) {
    for pins in pins {
        for &key in pins.0.keys() {
            if let TokenRef::Component(field) = key
                && field.kind() != FieldKind::Color
            {
                out.push(OverrideError::PinNotColor(field));
            }
        }
    }
}

fn component_errors(components: &BTreeMap<ComponentField, OverrideValue>, out: &mut Vec<OverrideError>) {
    for (&field, value) in components {
        let (expected, found) = (field.kind(), value.kind());
        if expected == FieldKind::Color {
            out.push(OverrideError::ColorNotPerMode(field));
        } else if expected != found {
            out.push(OverrideError::WrongKind { field, expected, found });
        }
    }
}

/// Seed colours. Each one becomes a 12-step ramp per mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Seeds {
    /// The grey family and its tint.
    pub neutral: Rgba,
    /// The accent hue.
    pub accent: Rgba,
    /// One hue per status.
    pub status: Status<Rgba>,
}

/// Fixed colours for some semantic tokens and component colour fields.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Pins(pub BTreeMap<TokenRef, Rgba>);

impl Pins {
    /// The pinned value of `token`, if any.
    pub fn get(&self, token: impl Into<TokenRef>) -> Option<Rgba> {
        self.0.get(&token.into()).copied()
    }
}

impl<K: Into<TokenRef>> FromIterator<(K, Rgba)> for Pins {
    fn from_iter<I: IntoIterator<Item = (K, Rgba)>>(iter: I) -> Pins {
        Pins(iter.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }
}

/// Pins for each mode.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ModePins {
    /// Used in light mode.
    pub light: Pins,
    /// Used in dark mode.
    pub dark: Pins,
}

impl ModePins {
    /// The pins of `mode`.
    pub fn get(&self, mode: Mode) -> &Pins {
        match mode {
            Mode::Light => &self.light,
            Mode::Dark => &self.dark,
        }
    }
}

/// A theme's own shape. Fields left `None` come from the Size preset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapeSeed {
    /// Replaces every radius of the preset.
    pub radius: Option<RadiusSet>,
    /// Replaces the preset's edge style.
    pub edge: Option<EdgeSeed>,
}

/// A theme's edge style. A bevel's colours are the `edge.light` and `edge.dark` colour tokens,
/// derived from the neutral ramp unless pinned.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EdgeSeed {
    /// No edge effect.
    Flat,
    /// A two-tone bevel.
    Bevel {
        /// Line width in logical pixels.
        width: f32,
    },
}

impl EdgeSeed {
    /// The edge this seed gives, a bevel taking `edge.light` and `edge.dark` from `color`.
    pub fn resolve(self, color: &ColorTokens) -> Edge {
        match self {
            EdgeSeed::Flat => Edge::Flat,
            EdgeSeed::Bevel { width } => Edge::Bevel { light: color.edge.light, dark: color.edge.dark, width },
        }
    }
}

/// Which modes a theme has.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Variants {
    /// Light and dark.
    Both,
    /// Light only; dark mode shows the light variant.
    LightOnly,
    /// Dark only; light mode shows the dark variant.
    DarkOnly,
}

impl Variants {
    /// The mode that is shown when `wanted` is asked for.
    pub fn mode_for(self, wanted: Mode) -> Mode {
        match self {
            Variants::Both => wanted,
            Variants::LightOnly => Mode::Light,
            Variants::DarkOnly => Mode::Dark,
        }
    }
}

/// The contrast a theme is designed for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ContrastIntent {
    /// Meets the standard minimums except where pins say otherwise.
    #[default]
    Standard,
    /// Deliberately soft (Soft Dark, Soft Light); the picker labels it.
    Low,
}

/// The user's contrast setting.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Contrast {
    /// WCAG AA minimums; pinned colours are kept and reported.
    #[default]
    Standard,
    /// Higher minimums, enforced on every colour including pinned ones.
    High,
}

/// The density preset: control heights, padding, gaps, type and default shape.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "snake_case"))]
pub enum Size {
    /// Dense, sharper corners (Pro).
    #[default]
    Compact,
    /// Roomier, rounder corners (Studio).
    Comfortable,
}

/// The text-size setting; scales type roles only.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "snake_case"))]
pub enum TextSize {
    /// 0.9×.
    Small,
    /// 1×.
    #[default]
    Default,
    /// 1.15×.
    Large,
    /// 1.3×.
    Larger,
}

impl TextSize {
    /// Every step, smallest first.
    pub const ALL: [TextSize; 4] = [TextSize::Small, TextSize::Default, TextSize::Large, TextSize::Larger];
}

/// The user's appearance choices, after "System" has been replaced by what the OS reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Appearance {
    /// Light or dark.
    pub mode: Mode,
    /// Standard or high contrast.
    pub contrast: Contrast,
    /// Density preset.
    pub size: Size,
    /// Type scaling.
    pub text_size: TextSize,
    /// Whether animations are turned off.
    pub reduced_motion: bool,
}
