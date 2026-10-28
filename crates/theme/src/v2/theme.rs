//! What a theme supplies (seeds, pins, optional shape) and what the user chooses (appearance).

use std::collections::BTreeMap;

use super::color::Rgba;
use super::ramp::Mode;
use super::tokens::{ColorToken, RadiusSet, Status};
use crate::ThemeId;

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
    /// Fixed values per mode, replacing derived ones.
    pub pins: ModePins,
    /// Fixed values used only with [`Contrast::High`], applied over `pins`.
    pub high_contrast: ModePins,
    /// Whether the theme is meant to have low contrast.
    pub contrast_intent: ContrastIntent,
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

/// Fixed colours for some tokens.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Pins(pub BTreeMap<ColorToken, Rgba>);

impl Pins {
    /// The pinned value of `token`, if any.
    pub fn get(&self, token: ColorToken) -> Option<Rgba> {
        self.0.get(&token).copied()
    }
}

impl FromIterator<(ColorToken, Rgba)> for Pins {
    fn from_iter<I: IntoIterator<Item = (ColorToken, Rgba)>>(iter: I) -> Pins {
        Pins(iter.into_iter().collect())
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
pub enum Size {
    /// Dense, sharper corners (Pro).
    #[default]
    Compact,
    /// Roomier, rounder corners (Studio).
    Comfortable,
}

/// The text-size setting; scales type roles only.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
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
