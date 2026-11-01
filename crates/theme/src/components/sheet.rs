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
    /// What an outline button's side is short of the lane: [`values::SHEET_OUTLINE_BUTTON_MARGIN`].
    pub outline_button_margin: f32,
    /// An outline button's least side: [`values::SHEET_OUTLINE_BUTTON_MIN`].
    pub outline_button_min: f32,
    /// The first outline lane's inset from the band's edge: [`values::SHEET_OUTLINE_INSET`].
    pub outline_inset: f32,
    /// How far an outline button's hit area reaches past its square: [`values::SHEET_OUTLINE_HIT`].
    pub outline_hit: f32,
    /// An outline button's corner radius: [`values::SHEET_OUTLINE_RADIUS`].
    pub outline_radius: f32,
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
    /// A dashed chart line's dash: [`values::SHEET_CHART_DASH`].
    pub chart_dash: f32,
    /// The gap between a dashed chart line's dashes: [`values::SHEET_CHART_DASH_GAP`].
    pub chart_dash_gap: f32,
    /// The gap between the error checking tag and its cell: [`values::SHEET_ERROR_TAG_GAP`].
    pub error_tag_gap: f32,
    /// The error checking tag's corner radius: [`values::SHEET_ERROR_TAG_RADIUS`].
    pub error_tag_radius: f32,
    /// The warning diamond's half-diagonal in the error checking tag: [`values::SHEET_ERROR_TAG_MARK`].
    pub error_tag_mark: f32,
    /// The exclamation mark's text size in the error checking tag: [`values::SHEET_ERROR_TAG_MARK_TEXT`].
    pub error_tag_mark_text: f32,
    /// The filter button's corner radius: [`values::SHEET_FILTER_BUTTON_RADIUS`].
    pub filter_button_radius: f32,
    /// The filter button's inset round its glyph: [`values::SHEET_FILTER_BUTTON_GLYPH_INSET`].
    pub filter_button_glyph_inset: f32,
    /// The gap between statistics in the status bar: [`values::SHEET_STATS_GAP`].
    pub stats_gap: f32,
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
        "outline_button_margin" 0.0..=16.0 => outline_button_margin;
        "outline_button_min" 2.0..=32.0 => outline_button_min;
        "outline_inset" 0.0..=16.0 => outline_inset;
        "outline_hit" 0.0..=8.0 => outline_hit;
        "outline_radius" 0.0..=8.0 => outline_radius;
        "slicer_header" 8.0..=96.0 => slicer_header;
        "slicer_tile" 8.0..=96.0 => slicer_tile;
        "slicer_gap" 0.0..=32.0 => slicer_gap;
        "slicer_padding" 0.0..=32.0 => slicer_padding;
        "slicer_clear" 8.0..=64.0 => slicer_clear;
        "error_tag" 6.0..=64.0 => error_tag;
        "filter_button_min" 4.0..=48.0 => filter_button_min;
        "filter_button_max" 6.0..=64.0 => filter_button_max;
        "filter_button_inset" 0.0..=16.0 => filter_button_inset;
        "chart_dash" 0.5..=32.0 => chart_dash;
        "chart_dash_gap" 0.5..=32.0 => chart_dash_gap;
        "error_tag_gap" 0.0..=16.0 => error_tag_gap;
        "error_tag_radius" 0.0..=16.0 => error_tag_radius;
        "error_tag_mark" 1.0..=32.0 => error_tag_mark;
        "error_tag_mark_text" 4.0..=32.0 => error_tag_mark_text;
        "filter_button_radius" 0.0..=16.0 => filter_button_radius;
        "filter_button_glyph_inset" 0.0..=16.0 => filter_button_glyph_inset;
        "stats_gap" 0.0..=64.0 => stats_gap;
    };

    fn defaults(_: &Semantic<'_>) -> SheetTokens {
        SheetTokens {
            column_header_height: values::SHEET_COLUMN_HEADER_HEIGHT,
            row_header_digit: values::SHEET_ROW_HEADER_DIGIT,
            row_header_padding: values::SHEET_ROW_HEADER_PADDING,
            outline_lane: values::SHEET_OUTLINE_LANE,
            outline_button_margin: values::SHEET_OUTLINE_BUTTON_MARGIN,
            outline_button_min: values::SHEET_OUTLINE_BUTTON_MIN,
            outline_inset: values::SHEET_OUTLINE_INSET,
            outline_hit: values::SHEET_OUTLINE_HIT,
            outline_radius: values::SHEET_OUTLINE_RADIUS,
            slicer_header: values::SHEET_SLICER_HEADER,
            slicer_tile: values::SHEET_SLICER_TILE,
            slicer_gap: values::SHEET_SLICER_GAP,
            slicer_padding: values::SHEET_SLICER_PADDING,
            slicer_clear: values::SHEET_SLICER_CLEAR,
            error_tag: values::SHEET_ERROR_TAG,
            filter_button_min: values::SHEET_FILTER_BUTTON_MIN,
            filter_button_max: values::SHEET_FILTER_BUTTON_MAX,
            filter_button_inset: values::SHEET_FILTER_BUTTON_INSET,
            chart_dash: values::SHEET_CHART_DASH,
            chart_dash_gap: values::SHEET_CHART_DASH_GAP,
            error_tag_gap: values::SHEET_ERROR_TAG_GAP,
            error_tag_radius: values::SHEET_ERROR_TAG_RADIUS,
            error_tag_mark: values::SHEET_ERROR_TAG_MARK,
            error_tag_mark_text: values::SHEET_ERROR_TAG_MARK_TEXT,
            filter_button_radius: values::SHEET_FILTER_BUTTON_RADIUS,
            filter_button_glyph_inset: values::SHEET_FILTER_BUTTON_GLYPH_INSET,
            stats_gap: values::SHEET_STATS_GAP,
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
