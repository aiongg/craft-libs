//! Colour harmonies (F04.16): groups of colours built from a base colour by turning its hue and
//! scaling its saturation and brightness, the variation grid round each, and five-colour themes.
//! VectorCraft's Color Guide and Color Themes math, on encoded sRGB through HSB.
//!
//! ```no_run
//! use craft_color::harmony::{Guide, GuideOptions, Harmony};
//! use craft_theme::Srgba;
//! let base = Srgba::rgb(0.24, 0.39, 0.87);
//! let triad = Harmony::Triad.apply(base);              // base first, then two partners
//! let guide = Guide::new(base, Harmony::Triad, &GuideOptions::default());
//! ```

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_theme::Srgba;

/// A harmony rule: a group of colours built from a base colour, base first. VectorCraft's 21
/// rules, in its menu order ([`Harmony::ALL`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "camelCase"))]
pub enum Harmony {
    #[default]
    Complementary,
    Complementary2,
    SplitComplementary,
    LeftComplement,
    RightComplement,
    Analogous,
    Analogous2,
    Monochromatic,
    Shades,
    Triad,
    Triad2,
    Triad3,
    Tetrad,
    Tetrad2,
    Tetrad3,
    Compound,
    Compound2,
    HighContrast,
    HighContrast2,
    HighContrast3,
    Pentagram,
}

impl Harmony {
    /// Every rule, in the menu's order.
    pub const ALL: [Harmony; 21] = [
        Harmony::Complementary,
        Harmony::Complementary2,
        Harmony::SplitComplementary,
        Harmony::LeftComplement,
        Harmony::RightComplement,
        Harmony::Analogous,
        Harmony::Analogous2,
        Harmony::Monochromatic,
        Harmony::Shades,
        Harmony::Triad,
        Harmony::Triad2,
        Harmony::Triad3,
        Harmony::Tetrad,
        Harmony::Tetrad2,
        Harmony::Tetrad3,
        Harmony::Compound,
        Harmony::Compound2,
        Harmony::HighContrast,
        Harmony::HighContrast2,
        Harmony::HighContrast3,
        Harmony::Pentagram,
    ];

    /// The rule's name for the menu, English (a words key): "Split Complementary", "Triad 2".
    // inv-vectorcraft: crates/color/src/harmony.rs:101 `label`.
    pub fn label(self) -> &'static str {
        todo!()
    }

    /// The rule's colours, the base first. Each partner is the base with its hue turned, its
    /// saturation scaled and its brightness scaled or flipped to the contrasting one (dark for a
    /// light base, light for a dark one). Alpha is the base's.
    // inv-vectorcraft: crates/color/src/harmony.rs:178 `apply`, with the `tones` table at :36–70
    // (`t`, `soft`, `deep`, `hc`) and `contrast` at :72; `keep_model` is not needed (sRGB only).
    pub fn apply(self, base: Srgba) -> Vec<Srgba> {
        todo!()
    }

    /// A [`THEME_SIZE`]-colour theme from `base`: the rule's colours, each followed by lighter or
    /// darker variations that fill the remaining places (the rule's first colours get them
    /// first), base first.
    // inv-vectorcraft: crates/color/src/harmony.rs:194 `theme`.
    pub fn theme(self, base: Srgba) -> Vec<Srgba> {
        todo!()
    }
}

/// The number of colours in a colour theme.
pub const THEME_SIZE: usize = 5;

/// Moves colour `i` of a harmony on the hue and saturation wheel to `hsb` (hue in degrees,
/// saturation and brightness 0–1). With `linked` the other colours keep their relation to it:
/// they turn by the same hue, scale their saturation alike and shift their brightness by as
/// much. An index past the end changes nothing.
// inv-vectorcraft: crates/color/src/harmony.rs:216 `move_on_wheel`.
pub fn move_on_wheel(colors: &mut [Srgba], i: usize, hsb: [f32; 3], linked: bool) {
    todo!()
}

/// What the variation grid varies.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "camelCase"))]
pub enum Variation {
    /// Towards black (left) and white (right): Show Tints/Shades.
    #[default]
    TintsShades,
    /// Towards cool and warm: Show Warm/Cool.
    WarmCool,
    /// Towards grey and full saturation: Show Vivid/Muted.
    VividMuted,
}

impl Variation {
    pub const ALL: [Variation; 3] = [Variation::TintsShades, Variation::WarmCool, Variation::VividMuted];

    /// The grid's ends, English (words keys): ("Shades", "Tints"), ("Cool", "Warm"),
    /// ("Muted", "Vivid").
    pub fn sides(self) -> (&'static str, &'static str) {
        match self {
            Variation::TintsShades => ("Shades", "Tints"),
            Variation::WarmCool => ("Cool", "Warm"),
            Variation::VividMuted => ("Muted", "Vivid"),
        }
    }
}

/// The variation grid's options: what it varies, steps on each side and how far they reach.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default, rename_all = "camelCase"))]
pub struct GuideOptions {
    pub variation: Variation,
    /// Variations on each side of a harmony colour, 1 to [`GuideOptions::MAX_STEPS`].
    pub steps: u32,
    /// How far the outermost variations go, 0–100 (Less … More): 10 % to 90 % of the way.
    pub amount: f32,
}

impl Default for GuideOptions {
    fn default() -> Self {
        GuideOptions { variation: Variation::TintsShades, steps: 4, amount: 50.0 }
    }
}

impl GuideOptions {
    pub const MAX_STEPS: u32 = 20;

    /// The options with `steps` and `amount` in their ranges.
    pub fn clamped(self) -> GuideOptions {
        GuideOptions { steps: self.steps.clamp(1, Self::MAX_STEPS), amount: self.amount.clamp(0.0, 100.0), ..self }
    }
}

/// The variation of `color` at `step` (−steps to +steps; negative towards shades, cool or muted,
/// positive towards tints, warm or vivid; 0 is `color`).
// inv-vectorcraft: crates/color/src/harmony.rs:299 `variation`, :292 `reach`.
pub fn variation(color: Srgba, options: &GuideOptions, step: i32) -> Srgba {
    todo!()
}

/// The Color Guide for a base colour: the harmony colours and, per colour, its row of variations
/// from −steps to +steps, the colour itself in the centre column.
#[derive(Clone, Debug, PartialEq)]
pub struct Guide {
    pub colors: Vec<Srgba>,
    pub grid: Vec<Vec<Srgba>>,
}

impl Guide {
    // inv-vectorcraft: crates/color/src/harmony.rs:328 `Guide::new`.
    pub fn new(base: Srgba, rule: Harmony, options: &GuideOptions) -> Guide {
        todo!()
    }

    /// The index of the centre column (the harmony colours) in each row.
    pub fn centre(&self) -> usize {
        self.grid.first().map_or(0, |row| row.len() / 2)
    }

    /// The guide with every colour moved to the nearest of `palette` (Limit to Library; ΔE2000
    /// in Lab), the centre column too. An empty palette changes nothing.
    // inv-vectorcraft: crates/color/src/harmony.rs:342 `limited`.
    pub fn limited(self, palette: &[Srgba]) -> Guide {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    /// Complementary gives the base and its hue turned 180°; every rule starts with the base.
    #[test]
    #[ignore = "skeleton"]
    fn every_rule_starts_with_the_base_and_complementary_turns_half_way() {
        todo!()
    }

    /// Moving one colour with `linked` turns the others by the same hue.
    #[test]
    #[ignore = "skeleton"]
    fn a_linked_move_turns_every_colour_alike() {
        todo!()
    }

    /// A theme always has five colours, base first.
    #[test]
    #[ignore = "skeleton"]
    fn a_theme_has_five_colours_base_first() {
        todo!()
    }

    /// The guide's centre column is the harmony; step 0 is the colour itself.
    #[test]
    #[ignore = "skeleton"]
    fn the_guide_centre_column_is_the_harmony() {
        todo!()
    }
}
