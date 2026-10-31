//! Theme chips: the small preview of a theme in the Appearance page. Sizes only (no colours): the colours are the surface, text and state tokens of the controls
//! inside, and no theme overrides these. The values are PhotoCraft's.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// ThemeChip tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ThemeChipTokens {
    /// Size of a chip: `theme_chip.size`.
    pub size: [f32; 2],
    /// From the outline to the fill: `theme_chip.inset`.
    pub inset: f32,
    /// Height of the chrome strip along the top of the fill: `theme_chip.strip`.
    pub strip: f32,
    /// Radius of the accent dot: `theme_chip.dot_radius`.
    pub dot_radius: f32,
    /// From the fill's bottom end corner to the dot's centre, on both axes: `theme_chip.dot_offset`.
    pub dot_offset: f32,
    /// Corner radius of the outline: `theme_chip.outline_radius`.
    pub outline_radius: f32,
    /// Corner radius of the fill: `theme_chip.fill_radius`.
    pub fill_radius: f32,
    /// Radius of the favourite star's badge: `theme_chip.badge_radius`.
    pub badge_radius: f32,
    /// How far the badge's centre sits inside the chip's top end corner, as `[x, y]`: `theme_chip.badge_inset`.
    pub badge_inset: [f32; 2],
    /// Between two chips, with room for the badge that overhangs a corner: `theme_chip.gap`.
    pub gap: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ThemeChipField {}

impl Family for ThemeChipTokens {
    type Field = ThemeChipField;
    const FIELDS: &'static [ThemeChipField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { ThemeChipTokens;
        "size.0" 0.0..=256.0 => size[0];
        "size.1" 0.0..=256.0 => size[1];
        "inset" 0.0..=16.0 => inset;
        "strip" 0.0..=64.0 => strip;
        "dot_radius" 0.0..=16.0 => dot_radius;
        "dot_offset" 0.0..=64.0 => dot_offset;
        "outline_radius" 0.0..=64.0 => outline_radius;
        "fill_radius" 0.0..=64.0 => fill_radius;
        "badge_radius" 0.0..=64.0 => badge_radius;
        "badge_inset.0" 0.0..=64.0 => badge_inset[0];
        "badge_inset.1" 0.0..=64.0 => badge_inset[1];
        "gap" 0.0..=64.0 => gap;
    };

    fn defaults(_: &Semantic<'_>) -> ThemeChipTokens {
        ThemeChipTokens {
            size: values::THEME_CHIP_SIZE,
            inset: values::THEME_CHIP_INSET,
            strip: values::THEME_CHIP_STRIP,
            dot_radius: values::THEME_CHIP_DOT_RADIUS,
            dot_offset: values::THEME_CHIP_DOT_OFFSET,
            outline_radius: values::THEME_CHIP_OUTLINE_RADIUS,
            fill_radius: values::THEME_CHIP_FILL_RADIUS,
            badge_radius: values::THEME_CHIP_BADGE_RADIUS,
            badge_inset: values::THEME_CHIP_BADGE_INSET,
            gap: values::THEME_CHIP_GAP,
        }
    }

    fn path(f: ThemeChipField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<ThemeChipField> {
        None
    }

    fn kind(f: ThemeChipField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: ThemeChipField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: ThemeChipField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
