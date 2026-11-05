//! The spreadsheet grid: the sizes of its headers, outline lanes, slicers and filter buttons, at
//! 100 % sheet zoom, and the colours of its selection fill and fill handle. `table.*` is the
//! kernel list table's family. The colours are overridable.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
use crate::contrast::relative_luminance;
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
    /// The gap between the error checking tag and its cell: [`values::SHEET_ERROR_TAG_GAP`].
    pub error_tag_gap: f32,
    /// The error checking tag's corner radius: [`values::SHEET_ERROR_TAG_RADIUS`].
    pub error_tag_radius: f32,
    /// The warning diamond's half-diagonal in the error checking tag: [`values::SHEET_ERROR_TAG_MARK`].
    pub error_tag_mark: f32,
    /// The filter button's corner radius: [`values::SHEET_FILTER_BUTTON_RADIUS`].
    pub filter_button_radius: f32,
    /// The filter button's inset round its glyph: [`values::SHEET_FILTER_BUTTON_GLYPH_INSET`].
    pub filter_button_glyph_inset: f32,
    /// The gap between statistics in the status bar: [`values::SHEET_STATS_GAP`].
    pub stats_gap: f32,
    /// How far from a header's far edge a press still grabs it: [`values::SHEET_EDGE_GRAB`].
    pub edge_grab: f32,
    /// The fill handle's side: [`values::SHEET_FILL_HANDLE`].
    pub fill_handle: f32,
    /// The accent bar on a selected header's inner edge: [`values::SHEET_HEADER_ACCENT_BAR`].
    pub header_accent_bar: f32,
    /// The distance of each line of the double divider from a hidden line's edge: [`values::SHEET_HIDDEN_MARK_OFFSET`].
    pub hidden_mark_offset: f32,
    /// How far the headers' separators stop short of the header's start: [`values::SHEET_HEADER_SEPARATOR_INSET`].
    pub header_separator_inset: f32,
    /// The status bar menu's least width: [`values::SHEET_STATUS_MENU_MIN_WIDTH`].
    pub status_menu_min_width: f32,
    /// The field list checklist's greatest height: [`values::SHEET_FIELD_LIST_CHECKLIST_MAX`].
    pub field_list_checklist_max: f32,
    /// A field list area's least height: [`values::SHEET_FIELD_LIST_AREA_MIN`].
    pub field_list_area_min: f32,
    /// The selection's tint over the cells: `sheet.selection_fill`, `accent.solid` at
    /// [`values::SHEET_SELECTION_FILL_ALPHA`].
    pub selection_fill: Rgba,
    /// The fill handle's fill: `sheet.fill_handle_fill`, `accent.solid`, the selection's border.
    pub fill_handle_fill: Rgba,
    /// The fill handle's one-point ring: `sheet.fill_handle_ring`, `surface.background`.
    pub fill_handle_ring: Rgba,
}

/// An overridable sheet field. Paths: `selection_fill`, `fill_handle_fill`, `fill_handle_ring`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SheetField {
    /// `sheet.selection_fill`
    SelectionFill,
    /// `sheet.fill_handle_fill`
    FillHandleFill,
    /// `sheet.fill_handle_ring`
    FillHandleRing,
}

impl SheetField {
    const ALL: [SheetField; 3] = [SheetField::SelectionFill, SheetField::FillHandleFill, SheetField::FillHandleRing];

    const fn name(self) -> &'static str {
        match self {
            SheetField::SelectionFill => "selection_fill",
            SheetField::FillHandleFill => "fill_handle_fill",
            SheetField::FillHandleRing => "fill_handle_ring",
        }
    }
}

impl SheetTokens {
    fn color(&self, f: SheetField) -> Rgba {
        match f {
            SheetField::SelectionFill => self.selection_fill,
            SheetField::FillHandleFill => self.fill_handle_fill,
            SheetField::FillHandleRing => self.fill_handle_ring,
        }
    }

    fn color_mut(&mut self, f: SheetField) -> &mut Rgba {
        match f {
            SheetField::SelectionFill => &mut self.selection_fill,
            SheetField::FillHandleFill => &mut self.fill_handle_fill,
            SheetField::FillHandleRing => &mut self.fill_handle_ring,
        }
    }
}

impl Family for SheetTokens {
    type Field = SheetField;
    const FIELDS: &'static [SheetField] = &SheetField::ALL;
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
        "error_tag_gap" 0.0..=16.0 => error_tag_gap;
        "error_tag_radius" 0.0..=16.0 => error_tag_radius;
        "error_tag_mark" 1.0..=32.0 => error_tag_mark;
        "filter_button_radius" 0.0..=16.0 => filter_button_radius;
        "filter_button_glyph_inset" 0.0..=16.0 => filter_button_glyph_inset;
        "stats_gap" 0.0..=64.0 => stats_gap;
        "edge_grab" 1.0..=16.0 => edge_grab;
        "fill_handle" 2.0..=32.0 => fill_handle;
        "header_accent_bar" 0.5..=8.0 => header_accent_bar;
        "hidden_mark_offset" 0.5..=8.0 => hidden_mark_offset;
        "header_separator_inset" 0.0..=16.0 => header_separator_inset;
        "status_menu_min_width" 60.0..=640.0 => status_menu_min_width;
        "field_list_checklist_max" 60.0..=1024.0 => field_list_checklist_max;
        "field_list_area_min" 16.0..=512.0 => field_list_area_min;
    };

    fn defaults(s: &Semantic<'_>) -> SheetTokens {
        let c = s.color;
        // A dark theme's text is lighter than its background.
        let dark = relative_luminance(c.surface.background) < relative_luminance(c.text.normal);
        let (light_alpha, dark_alpha) = values::SHEET_SELECTION_FILL_ALPHA;
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
            error_tag_gap: values::SHEET_ERROR_TAG_GAP,
            error_tag_radius: values::SHEET_ERROR_TAG_RADIUS,
            error_tag_mark: values::SHEET_ERROR_TAG_MARK,
            filter_button_radius: values::SHEET_FILTER_BUTTON_RADIUS,
            filter_button_glyph_inset: values::SHEET_FILTER_BUTTON_GLYPH_INSET,
            stats_gap: values::SHEET_STATS_GAP,
            edge_grab: values::SHEET_EDGE_GRAB,
            fill_handle: values::SHEET_FILL_HANDLE,
            header_accent_bar: values::SHEET_HEADER_ACCENT_BAR,
            hidden_mark_offset: values::SHEET_HIDDEN_MARK_OFFSET,
            header_separator_inset: values::SHEET_HEADER_SEPARATOR_INSET,
            status_menu_min_width: values::SHEET_STATUS_MENU_MIN_WIDTH,
            field_list_checklist_max: values::SHEET_FIELD_LIST_CHECKLIST_MAX,
            field_list_area_min: values::SHEET_FIELD_LIST_AREA_MIN,
            selection_fill: c.accent.solid.with_alpha(if dark { dark_alpha } else { light_alpha }),
            fill_handle_fill: c.accent.solid,
            fill_handle_ring: c.surface.background,
        }
    }

    fn path(f: SheetField) -> String {
        f.name().to_owned()
    }

    fn from_path(path: &str) -> Option<SheetField> {
        SheetField::ALL.into_iter().find(|f| f.name() == path)
    }

    fn kind(_: SheetField) -> FieldKind {
        FieldKind::Color
    }

    fn get(&self, f: SheetField) -> OverrideValue {
        OverrideValue::Color(self.color(f))
    }

    fn set(&mut self, f: SheetField, v: OverrideValue) {
        if let OverrideValue::Color(c) = v {
            *self.color_mut(f) = c;
        }
    }

    // The selection fill is a tint over cells the app paints; the handle's ring separates it
    // from them.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
