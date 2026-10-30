//! Gradient palettes: the grid and list of gradient presets in groups. The sizes scale with the
//! control height, and no theme overrides them; the colours are semantic tokens.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// GradientPalette tokens. Sizes are in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct GradientPaletteTokens {
    /// Width of a grid cell: `gradient_palette.cell_width`.
    pub cell_width: f32,
    /// Height of a grid cell: `gradient_palette.cell_height`.
    pub cell_height: f32,
    /// Width of a list row's swatch: `gradient_palette.list_swatch_width`.
    pub list_swatch_width: f32,
    /// Height of a list row's swatch: `gradient_palette.list_swatch_height`.
    pub list_swatch_height: f32,
    /// Height at which the palette scrolls: `gradient_palette.max_height`.
    pub max_height: f32,
}

/// GradientPalettes have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GradientPaletteField {}

impl Family for GradientPaletteTokens {
    type Field = GradientPaletteField;
    const FIELDS: &'static [GradientPaletteField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { GradientPaletteTokens;
        "cell_width" 0.0..=1024.0 => cell_width;
        "cell_height" 0.0..=1024.0 => cell_height;
        "list_swatch_width" 0.0..=1024.0 => list_swatch_width;
        "list_swatch_height" 0.0..=1024.0 => list_swatch_height;
        "max_height" 0.0..=1024.0 => max_height;
    };

    /// The Compact sizes, scaled by the control height of the size and rounded to whole points.
    fn defaults(s: &Semantic<'_>) -> GradientPaletteTokens {
        let k = s.size.control.md / values::COLOR_WHEEL_BASE_CONTROL;
        let scale = |w: f32| if k == 1.0 { w } else { (w * k).round() };
        GradientPaletteTokens {
            cell_width: scale(values::GRADIENT_PALETTE_CELL_WIDTH),
            cell_height: scale(values::GRADIENT_PALETTE_CELL_HEIGHT),
            list_swatch_width: scale(values::GRADIENT_PALETTE_LIST_SWATCH_WIDTH),
            list_swatch_height: scale(values::GRADIENT_PALETTE_LIST_SWATCH_HEIGHT),
            max_height: scale(values::GRADIENT_PALETTE_MAX_HEIGHT),
        }
    }

    fn path(f: GradientPaletteField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<GradientPaletteField> {
        None
    }

    fn kind(f: GradientPaletteField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: GradientPaletteField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: GradientPaletteField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
