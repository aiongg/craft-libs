//! Paint chips: the shape fill and stroke chip. The sizes scale with the control height, and no
//! theme overrides them; the chip's edge and checker are `swatch.*` tokens.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Paint chip tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PaintChipTokens {
    /// Width of the chip: `paint_chip.width`.
    pub width: f32,
    /// Height of the chip: `paint_chip.height`.
    pub height: f32,
}

/// Paint chips have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PaintChipField {}

impl Family for PaintChipTokens {
    type Field = PaintChipField;
    const FIELDS: &'static [PaintChipField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { PaintChipTokens;
        "width" 0.0..=1024.0 => width;
        "height" 0.0..=1024.0 => height;
    };

    /// The Compact sizes (PhotoCraft's), scaled by the control height of the size and rounded to
    /// whole points.
    fn defaults(s: &Semantic<'_>) -> PaintChipTokens {
        let k = s.size.control.md / values::COLOR_PICKER_BASE_CONTROL;
        let scale = |w: f32| (w * k).round();
        PaintChipTokens { width: scale(values::PAINT_CHIP_WIDTH), height: scale(values::PAINT_CHIP_HEIGHT) }
    }

    fn path(f: PaintChipField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<PaintChipField> {
        None
    }

    fn kind(f: PaintChipField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: PaintChipField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: PaintChipField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
