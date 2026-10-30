//! Readouts: the rows of values beside the pointer during a canvas gesture (a marquee's size, a
//! guide's position). They draw in `surface.raised`, `line.border` and the text colours; the family
//! holds their placement and spacing.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Readout tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ReadoutTokens {
    /// From the pointer to the readout's corner, across: `readout.offset_x`.
    pub offset_x: f32,
    /// From the pointer to the readout's corner, down: `readout.offset_y`.
    pub offset_y: f32,
    /// Padding beside the rows: `readout.pad_x`.
    pub pad_x: f32,
    /// Padding above and below the rows: `readout.pad_y`.
    pub pad_y: f32,
    /// Between the widest label and the widest value: `readout.col_gap`.
    pub col_gap: f32,
    /// Between rows: `readout.row_gap`.
    pub row_gap: f32,
}

/// Readouts have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ReadoutField {}

impl Family for ReadoutTokens {
    type Field = ReadoutField;
    const FIELDS: &'static [ReadoutField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { ReadoutTokens;
        "offset_x" 0.0..=64.0 => offset_x;
        "offset_y" 0.0..=64.0 => offset_y;
        "pad_x" 0.0..=64.0 => pad_x;
        "pad_y" 0.0..=64.0 => pad_y;
        "col_gap" 0.0..=64.0 => col_gap;
        "row_gap" 0.0..=64.0 => row_gap;
    };

    /// The Compact sizes (PhotoCraft's), scaled by the control height of the size and rounded to whole points.
    fn defaults(s: &Semantic<'_>) -> ReadoutTokens {
        let k = s.size.control.md / values::READOUT_BASE_CONTROL;
        let scale = |w: f32| (w * k).round();
        ReadoutTokens {
            offset_x: scale(values::READOUT_OFFSET.0),
            offset_y: scale(values::READOUT_OFFSET.1),
            pad_x: scale(values::READOUT_PAD.0),
            pad_y: scale(values::READOUT_PAD.1),
            col_gap: scale(values::READOUT_COL_GAP),
            row_gap: scale(values::READOUT_ROW_GAP),
        }
    }

    fn path(f: ReadoutField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<ReadoutField> {
        None
    }

    fn kind(f: ReadoutField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: ReadoutField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: ReadoutField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
