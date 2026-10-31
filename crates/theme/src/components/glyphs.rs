//! The Glyphs panel: the grid of characters of a face and the strip of recently used glyphs.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Glyphs panel tokens, in points, the same at every Size because the cells show type at a size
/// the user picks.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct GlyphsTokens {
    /// Height of the scrolling grid: `glyphs.grid_height`.
    pub grid_height: f32,
    /// Cell size when the panel's settings have none: `glyphs.cell`.
    pub cell: f32,
    /// Smallest cell the zoom slider reaches: `glyphs.cell_min`.
    pub cell_min: f32,
    /// Largest cell the zoom slider reaches: `glyphs.cell_max`.
    pub cell_max: f32,
    /// Space between a cell's edge and its glyph: `glyphs.glyph_inset`.
    pub glyph_inset: f32,
    /// Side of a Recently Used cell: `glyphs.recent_cell`.
    pub recent_cell: f32,
    /// Distance between Recently Used cells, also the height of the strip: `glyphs.recent_pitch`.
    pub recent_pitch: f32,
    /// Space between the strip's edge and its first cell and top: `glyphs.recent_pad`.
    pub recent_pad: f32,
    /// Space between a Recently Used cell's edge and its glyph: `glyphs.recent_glyph_inset`.
    pub recent_glyph_inset: f32,
}

/// The Glyphs panel has no overridable field: its sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum GlyphsField {}

impl Family for GlyphsTokens {
    type Field = GlyphsField;
    const FIELDS: &'static [GlyphsField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { GlyphsTokens;
        "grid_height" 0.0..=1024.0 => grid_height;
        "cell" 0.0..=256.0 => cell;
        "cell_min" 0.0..=256.0 => cell_min;
        "cell_max" 0.0..=256.0 => cell_max;
        "glyph_inset" 0.0..=64.0 => glyph_inset;
        "recent_cell" 0.0..=256.0 => recent_cell;
        "recent_pitch" 0.0..=256.0 => recent_pitch;
        "recent_pad" 0.0..=64.0 => recent_pad;
        "recent_glyph_inset" 0.0..=64.0 => recent_glyph_inset;
    };

    fn defaults(_: &Semantic<'_>) -> GlyphsTokens {
        GlyphsTokens {
            grid_height: values::GLYPHS_GRID_HEIGHT,
            cell: values::GLYPHS_CELL,
            cell_min: values::GLYPHS_CELL_MIN,
            cell_max: values::GLYPHS_CELL_MAX,
            glyph_inset: values::GLYPHS_GLYPH_INSET,
            recent_cell: values::GLYPHS_RECENT_CELL,
            recent_pitch: values::GLYPHS_RECENT_PITCH,
            recent_pad: values::GLYPHS_RECENT_PAD,
            recent_glyph_inset: values::GLYPHS_RECENT_GLYPH_INSET,
        }
    }

    fn path(f: GlyphsField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<GlyphsField> {
        None
    }

    fn kind(f: GlyphsField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: GlyphsField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: GlyphsField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: the cells draw in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
