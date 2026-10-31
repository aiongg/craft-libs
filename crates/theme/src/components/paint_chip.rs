//! Paint chips: the shape fill and stroke chip. The sizes scale with the control height, and no
//! theme overrides them; the chip's edge and checker are `swatch.*` tokens.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Paint chip tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PaintChipTokens {
    /// Width and height of the chip: `paint_chip.size`.
    pub size: [f32; 2],
}

/// Paint chips have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PaintChipField {}

impl Family for PaintChipTokens {
    type Field = PaintChipField;
    const FIELDS: &'static [PaintChipField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { PaintChipTokens;
        "size.0" 0.0..=1024.0 => size[0];
        "size.1" 0.0..=1024.0 => size[1];
    };

    /// The Compact sizes (PhotoCraft's), scaled by the control height of the size and rounded to
    /// whole points.
    fn defaults(s: &Semantic<'_>) -> PaintChipTokens {
        let k = s.size.control.md / values::COLOR_PICKER_BASE_CONTROL;
        let scale = |w: f32| (w * k).round();
        PaintChipTokens { size: [scale(values::PAINT_CHIP_WIDTH), scale(values::PAINT_CHIP_HEIGHT)] }
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
