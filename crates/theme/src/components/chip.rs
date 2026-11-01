//! Chips: pills with a remove cross, in a field that wraps whole chips. Sizes only: the colours are the surface, text and state tokens of the controls inside, and no theme overrides these.

use super::{BySize, ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Chip tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ChipTokens {
    /// Height of a chip: `chip.height`.
    pub height: BySize<f32>,
    /// Between a chip's edge and its label, and after a label with no cross: `chip.pad_x`.
    pub pad_x: f32,
    /// Width of the remove cross's target, at most the chip's height: `chip.remove_width`.
    pub remove_width: f32,
    /// Between chips and between lines: `chip.gap`.
    pub gap: f32,
    /// Reach of the cross's arms from its centre: `chip.cross_arm`.
    pub cross_arm: f32,
    /// Stroke width of the cross: `chip.cross_stroke`.
    pub cross_stroke: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ChipField {}

impl Family for ChipTokens {
    type Field = ChipField;
    const FIELDS: &'static [ChipField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { ChipTokens;
        "height.sm" 0.0..=256.0 => height.sm;
        "height.md" 0.0..=256.0 => height.md;
        "pad_x" 0.0..=64.0 => pad_x;
        "remove_width" 0.0..=256.0 => remove_width;
        "gap" 0.0..=64.0 => gap;
        "cross_arm" 0.0..=32.0 => cross_arm;
        "cross_stroke" 0.0..=16.0 => cross_stroke;
    };

    fn defaults(_: &Semantic<'_>) -> ChipTokens {
        ChipTokens {
            height: BySize { sm: values::CHIP_HEIGHT.0, md: values::CHIP_HEIGHT.1 },
            pad_x: values::CHIP_PAD_X,
            remove_width: values::CHIP_REMOVE_WIDTH,
            gap: values::CHIP_GAP,
            cross_arm: values::CHIP_CROSS_ARM,
            cross_stroke: values::CHIP_CROSS_STROKE,
        }
    }

    fn path(f: ChipField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<ChipField> {
        None
    }

    fn kind(f: ChipField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: ChipField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: ChipField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
