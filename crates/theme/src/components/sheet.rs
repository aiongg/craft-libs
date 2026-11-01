//! The spreadsheet grid: the sizes of its headers, outline lanes, slicers and filter buttons, at
//! 100 % sheet zoom. `table.*` is the kernel list table's family. No theme overrides it.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Sheet tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SheetTokens {
    /// The column headers' height: [`values::SHEET_COLUMN_HEADER_HEIGHT`].
    pub column_header_height: f32,
    /// The row headers' width per digit of the last visible row: [`values::SHEET_ROW_HEADER_DIGIT`].
    pub row_header_digit: f32,
    /// The room around the row header's digits: [`values::SHEET_ROW_HEADER_PADDING`].
    pub row_header_padding: f32,
    /// One outline lane: [`values::SHEET_OUTLINE_LANE`].
    pub outline_lane: f32,
    /// A slicer's header: [`values::SHEET_SLICER_HEADER`].
    pub slicer_header: f32,
    /// A slicer's value tile: [`values::SHEET_SLICER_TILE`].
    pub slicer_tile: f32,
    /// The gap between slicer tiles: [`values::SHEET_SLICER_GAP`].
    pub slicer_gap: f32,
    /// The padding round a slicer's tiles: [`values::SHEET_SLICER_PADDING`].
    pub slicer_padding: f32,
    /// A slicer's clear button: [`values::SHEET_SLICER_CLEAR`].
    pub slicer_clear: f32,
    /// The error checking tag's square: [`values::SHEET_ERROR_TAG`].
    pub error_tag: f32,
    /// The filter button's least side: [`values::SHEET_FILTER_BUTTON_MIN`].
    pub filter_button_min: f32,
    /// The filter button's largest side: [`values::SHEET_FILTER_BUTTON_MAX`].
    pub filter_button_max: f32,
    /// The filter button's inset from the header cell's corner: [`values::SHEET_FILTER_BUTTON_INSET`].
    pub filter_button_inset: f32,
}

/// Sheet tokens have no overridable field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SheetField {}

impl Family for SheetTokens {
    type Field = SheetField;
    const FIELDS: &'static [SheetField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { SheetTokens;
        "column_header_height" 8.0..=96.0 => column_header_height;
        "row_header_digit" 2.0..=32.0 => row_header_digit;
        "row_header_padding" 0.0..=64.0 => row_header_padding;
        "outline_lane" 4.0..=64.0 => outline_lane;
        "slicer_header" 8.0..=96.0 => slicer_header;
        "slicer_tile" 8.0..=96.0 => slicer_tile;
        "slicer_gap" 0.0..=32.0 => slicer_gap;
        "slicer_padding" 0.0..=32.0 => slicer_padding;
        "slicer_clear" 8.0..=64.0 => slicer_clear;
        "error_tag" 6.0..=64.0 => error_tag;
        "filter_button_min" 4.0..=48.0 => filter_button_min;
        "filter_button_max" 6.0..=64.0 => filter_button_max;
        "filter_button_inset" 0.0..=16.0 => filter_button_inset;
    };

    fn defaults(_: &Semantic<'_>) -> SheetTokens {
        SheetTokens {
            column_header_height: values::SHEET_COLUMN_HEADER_HEIGHT,
            row_header_digit: values::SHEET_ROW_HEADER_DIGIT,
            row_header_padding: values::SHEET_ROW_HEADER_PADDING,
            outline_lane: values::SHEET_OUTLINE_LANE,
            slicer_header: values::SHEET_SLICER_HEADER,
            slicer_tile: values::SHEET_SLICER_TILE,
            slicer_gap: values::SHEET_SLICER_GAP,
            slicer_padding: values::SHEET_SLICER_PADDING,
            slicer_clear: values::SHEET_SLICER_CLEAR,
            error_tag: values::SHEET_ERROR_TAG,
            filter_button_min: values::SHEET_FILTER_BUTTON_MIN,
            filter_button_max: values::SHEET_FILTER_BUTTON_MAX,
            filter_button_inset: values::SHEET_FILTER_BUTTON_INSET,
        }
    }

    fn path(f: SheetField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<SheetField> {
        None
    }

    fn kind(f: SheetField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: SheetField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: SheetField, _: OverrideValue) {
        match f {}
    }

    // Glyphs and bars drawn over surfaces other families check.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
