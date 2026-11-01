//! Tables: the header's height, the sort chevron, the column edges' grab strip and the drop line of a moving column. The row's height, padding and colours are the list family's. Sizes only: the colours are the surface, text and state tokens of the controls inside, and no theme overrides these.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Table tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TableTokens {
    /// Height of the header row, the Tab strip's (`bar.tab`): `table.header_height`.
    pub header_height: f32,
    /// Side of the sort chevron: `table.sort_glyph`.
    pub sort_glyph: f32,
    /// From the header text to the chevron's box: `table.sort_gap`.
    pub sort_gap: f32,
    /// Width of a column edge's grab strip, centred on the edge: `table.grip_width`.
    pub grip_width: f32,
    /// Width of the line that shows where a moved column lands: `table.drop_line_width`.
    pub drop_line_width: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TableField {}

impl Family for TableTokens {
    type Field = TableField;
    const FIELDS: &'static [TableField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { TableTokens;
        "header_height" 0.0..=256.0 => header_height;
        "sort_glyph" 0.0..=64.0 => sort_glyph;
        "sort_gap" 0.0..=64.0 => sort_gap;
        "grip_width" 0.0..=64.0 => grip_width;
        "drop_line_width" 0.0..=16.0 => drop_line_width;
    };

    fn defaults(s: &Semantic<'_>) -> TableTokens {
        TableTokens {
            header_height: s.size.bar.tab,
            sort_glyph: values::TABLE_SORT_GLYPH,
            sort_gap: values::TABLE_SORT_GAP,
            grip_width: values::TABLE_GRIP_WIDTH,
            drop_line_width: values::TABLE_DROP_LINE_WIDTH,
        }
    }

    fn path(f: TableField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<TableField> {
        None
    }

    fn kind(f: TableField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: TableField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: TableField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
