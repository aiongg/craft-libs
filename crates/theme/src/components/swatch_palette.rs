//! Swatch palettes: the grid of colour wells. This family holds the side of a table cell; the
//! wells themselves are swatches. No theme overrides it.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// SwatchPalette tokens. Sizes are in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SwatchPaletteTokens {
    /// Side of a cell in a palette's table mode, where cells touch: `swatch_palette.table_cell`.
    pub table_cell: f32,
}

/// SwatchPalettes have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SwatchPaletteField {}

impl Family for SwatchPaletteTokens {
    type Field = SwatchPaletteField;
    const FIELDS: &'static [SwatchPaletteField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { SwatchPaletteTokens;
        "table_cell" 0.0..=1024.0 => table_cell;
    };

    /// The Compact side, scaled by the control height of the size and rounded to whole points.
    fn defaults(s: &Semantic<'_>) -> SwatchPaletteTokens {
        let k = s.size.control.md / values::COLOR_WHEEL_BASE_CONTROL;
        let scale = |w: f32| if k == 1.0 { w } else { (w * k).round() };
        SwatchPaletteTokens { table_cell: scale(values::SWATCH_PALETTE_TABLE_CELL) }
    }

    fn path(f: SwatchPaletteField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<SwatchPaletteField> {
        None
    }

    fn kind(f: SwatchPaletteField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: SwatchPaletteField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: SwatchPaletteField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: the cells are swatches.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
