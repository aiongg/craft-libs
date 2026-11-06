//! [`SheetStyle`]: the spreadsheet widgets' sizes, from the semantic tokens and the sheet's own constants.

use craft_theme::TypeRole;
use craft_ui::theme::{ToEgui as _, UiTokens};
use egui::Color32;

/// An outline button's least side.
const OUTLINE_BUTTON_MIN: f32 = 7.0;
/// How far an outline button's hit area reaches past its square.
const OUTLINE_HIT: f32 = 1.5;
/// The filter button's least and largest side.
const FILTER_BUTTON_MIN: f32 = 10.0;
const FILTER_BUTTON_MAX: f32 = 18.0;
/// The filter button's inset round the glyph.
const FILTER_GLYPH_INSET: f32 = 1.5;
/// How far from a header's far edge a press still grabs it.
const EDGE_GRAB: f32 = 4.0;
/// How far from a selection's border a press still moves it.
const BORDER_GRAB: f32 = 3.0;
/// The room after the last lane in an outline band.
const OUTLINE_BAND_SLACK: f32 = 3.0;
/// The distance of each line of a hidden line's double divider from its edge.
const HIDDEN_MARK_OFFSET: f32 = 1.5;
/// The status bar menu's least width.
const STATUS_MENU_MIN_WIDTH: f32 = 180.0;
/// The field list checklist's greatest height.
const FIELD_LIST_CHECKLIST_MAX: f32 = 220.0;
/// An area's least height in the field list.
const FIELD_LIST_AREA_MIN: f32 = 60.0;
/// The alpha of the selection's tint over the cells.
const SELECTION_ALPHA: u8 = 0x24;

/// The sizes the spreadsheet widgets read, at 100 % sheet zoom (the widgets scale them by the
/// zoom).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SheetStyle {
    /// The column headers' height.
    pub column_header_height: f32,
    /// The row headers' width per digit of the last visible row, and the room around the
    /// digits.
    pub row_header_digit: f32,
    /// The padding around a row header's digits.
    pub row_header_padding: f32,
    /// One outline lane.
    pub outline_lane: f32,
    /// What an outline button's side is short of the lane, its least side, the first lane's inset,
    /// how far a button's hit area reaches past its square and its corner radius.
    pub outline_button_margin: f32,
    /// An outline button's least side.
    pub outline_button_min: f32,
    /// The first lane's inset.
    pub outline_inset: f32,
    /// How far an outline button's hit area reaches past its square.
    pub outline_hit: f32,
    /// An outline button's corner radius.
    pub outline_radius: f32,
    /// The type role of a slicer's caption: the theme's strong body role.
    pub slicer_caption: TypeRole,
    /// A slicer's header, value tile, gap between tiles, padding round the tiles and clear
    /// button.
    pub slicer_header: f32,
    /// A slicer's value tile height.
    pub slicer_tile: f32,
    /// The gap between slicer tiles.
    pub slicer_gap: f32,
    /// The padding round a slicer's tiles.
    pub slicer_padding: f32,
    /// A slicer's clear button side.
    pub slicer_clear: f32,
    /// The error checking tag's square.
    pub error_tag: f32,
    /// The filter button's least and largest side and its inset from the header cell's corner.
    pub filter_button_min: f32,
    /// The filter button's largest side.
    pub filter_button_max: f32,
    /// The filter button's inset from the header cell's corner.
    pub filter_button_inset: f32,
    /// The filter button's corner radius and its inset round the glyph.
    pub filter_button_radius: f32,
    /// The filter button's inset round the glyph.
    pub filter_button_glyph_inset: f32,
    /// The error checking tag's gap to its cell and corner radius.
    pub error_tag_gap: f32,
    /// The error checking tag's corner radius.
    pub error_tag_radius: f32,
    /// The gap between the status bar's statistics.
    pub stats_gap: f32,
    /// The type role of the status bar's statistics: the theme's label role.
    pub stats_text: TypeRole,
    /// How far from a header's far edge a press still grabs it, at 100 % zoom.
    pub edge_grab: f32,
    /// How far from a selection's border a press still moves it, at 100 % zoom.
    pub border_grab: f32,
    /// The room after the last lane in an outline band, at any zoom.
    pub outline_band_slack: f32,
    /// The fill handle's side.
    pub fill_handle: f32,
    /// The accent bar on a selected header's inner edge.
    pub header_accent_bar: f32,
    /// The distance of each line of a hidden line's double divider from its edge.
    pub hidden_mark_offset: f32,
    /// How far the headers' separators stop short of the header's start.
    pub header_separator_inset: f32,
    /// The status bar menu's least width.
    pub status_menu_min_width: f32,
    /// The field list checklist's greatest height and an area's least height.
    pub field_list_checklist_max: f32,
    /// An area's least height in the field list.
    pub field_list_area_min: f32,
    /// The selection's tint over the cells.
    pub selection_fill: Color32,
    /// The fill handle's fill and its one-point ring.
    pub fill_handle_fill: Color32,
    /// The fill handle's one-point ring colour.
    pub fill_handle_ring: Color32,
    /// The type role of the column and row headers' text, the outline buttons' digits, a
    /// condition's problem line and the field list's hints: the theme's caption role.
    pub header_text: TypeRole,
    /// The type role of the outline buttons' digits.
    pub outline_text: TypeRole,
    /// The type role of a condition's problem line.
    pub problem_text: TypeRole,
    /// The type role of the field list's hints.
    pub hint_text: TypeRole,
    /// The type role of a field list's area headings and a slicer's tiles: the theme's body
    /// role.
    pub heading_text: TypeRole,
    /// The type role of a slicer's tiles.
    pub slicer_text: TypeRole,
    /// The type roles of a chart's text, plain and bold: the theme's body and strong body roles.
    pub chart_text: TypeRole,
    /// The type role of a chart's bold text.
    pub chart_text_strong: TypeRole,
}

impl SheetStyle {
    /// The style from the theme's tokens: `SheetStyle::of(&Tokens::of(ui))`.
    pub fn of(tokens: &UiTokens) -> SheetStyle {
        let (c, size, space) = (&tokens.color, &tokens.size, &tokens.space);
        SheetStyle {
            column_header_height: size.control.sm,
            row_header_digit: size.icon.sm / 2.0,
            row_header_padding: space.pad.panel,
            outline_lane: size.icon.sm,
            outline_button_margin: space.gap.tight,
            outline_button_min: OUTLINE_BUTTON_MIN,
            outline_inset: space.gap.tight,
            outline_hit: OUTLINE_HIT,
            outline_radius: tokens.shape.radius.sm,
            slicer_caption: TypeRole::BodyStrong,
            slicer_header: size.control.md,
            slicer_tile: size.control.sm,
            slicer_gap: space.gap.tight,
            slicer_padding: space.gap.inline,
            slicer_clear: size.icon.sm,
            error_tag: size.icon.md,
            filter_button_min: FILTER_BUTTON_MIN,
            filter_button_max: FILTER_BUTTON_MAX,
            filter_button_inset: space.gap.tight,
            error_tag_gap: space.gap.tight,
            error_tag_radius: tokens.shape.radius.sm,
            filter_button_radius: tokens.shape.radius.sm,
            filter_button_glyph_inset: FILTER_GLYPH_INSET,
            stats_gap: space.gap.stack,
            stats_text: TypeRole::Label,
            edge_grab: EDGE_GRAB,
            border_grab: BORDER_GRAB,
            outline_band_slack: OUTLINE_BAND_SLACK,
            fill_handle: size.handle.sm,
            header_accent_bar: tokens.stroke.thick,
            hidden_mark_offset: HIDDEN_MARK_OFFSET,
            header_separator_inset: space.gap.tight,
            status_menu_min_width: STATUS_MENU_MIN_WIDTH,
            field_list_checklist_max: FIELD_LIST_CHECKLIST_MAX,
            field_list_area_min: FIELD_LIST_AREA_MIN,
            selection_fill: c.accent.solid.with_alpha(SELECTION_ALPHA).to_egui(),
            fill_handle_fill: c.accent.solid.to_egui(),
            fill_handle_ring: c.surface.background.to_egui(),
            header_text: TypeRole::Caption,
            outline_text: TypeRole::Caption,
            problem_text: TypeRole::Caption,
            hint_text: TypeRole::Caption,
            heading_text: TypeRole::Body,
            slicer_text: TypeRole::Body,
            chart_text: TypeRole::Body,
            chart_text_strong: TypeRole::BodyStrong,
        }
    }
}
