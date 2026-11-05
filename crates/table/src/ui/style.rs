//! [`SheetStyle`]: the spreadsheet widgets' sizes, from the `sheet` tokens.

use craft_theme::TypeRole;
use craft_ui::theme::{ToEgui as _, UiTokens};
use egui::Color32;

/// The sizes the spreadsheet widgets read, one `sheet.*` token each, at 100 % sheet zoom (the
/// widgets scale them by the zoom). craft-theme's `sheet` family holds GridCraft's values. The family is
/// `sheet`, not `table`: `table.*` is the kernel's list table.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SheetStyle {
    /// The column headers' height: `sheet.column_header_height`.
    pub column_header_height: f32,
    /// The row headers' width per digit of the last visible row, and the room around the
    /// digits: `sheet.row_header_digit`, `sheet.row_header_padding`.
    pub row_header_digit: f32,
    /// The padding around a row header's digits: `sheet.row_header_padding`.
    pub row_header_padding: f32,
    /// One outline lane: `sheet.outline_lane`.
    pub outline_lane: f32,
    /// What an outline button's side is short of the lane, its least side, the first lane's inset,
    /// how far a button's hit area reaches past its square and its corner radius:
    /// `sheet.outline_button_margin`, `sheet.outline_button_min`, `sheet.outline_inset`,
    /// `sheet.outline_hit`, `sheet.outline_radius`.
    pub outline_button_margin: f32,
    /// An outline button's least side: `sheet.outline_button_min`.
    pub outline_button_min: f32,
    /// The first lane's inset: `sheet.outline_inset`.
    pub outline_inset: f32,
    /// How far an outline button's hit area reaches past its square: `sheet.outline_hit`.
    pub outline_hit: f32,
    /// An outline button's corner radius: `sheet.outline_radius`.
    pub outline_radius: f32,
    /// The type role of a slicer's caption: the theme's strong body role.
    pub slicer_caption: TypeRole,
    /// A slicer's header, value tile, gap between tiles, padding round the tiles and clear
    /// button: `sheet.slicer_header`, `sheet.slicer_tile`, `sheet.slicer_gap`,
    /// `sheet.slicer_padding`, `sheet.slicer_clear`.
    pub slicer_header: f32,
    /// A slicer's value tile height: `sheet.slicer_tile`.
    pub slicer_tile: f32,
    /// The gap between slicer tiles: `sheet.slicer_gap`.
    pub slicer_gap: f32,
    /// The padding round a slicer's tiles: `sheet.slicer_padding`.
    pub slicer_padding: f32,
    /// A slicer's clear button side: `sheet.slicer_clear`.
    pub slicer_clear: f32,
    /// The error checking tag's square: `sheet.error_tag`.
    pub error_tag: f32,
    /// The filter button's least and largest side and its inset from the header cell's corner:
    /// `sheet.filter_button_min`, `sheet.filter_button_max`, `sheet.filter_button_inset`.
    pub filter_button_min: f32,
    /// The filter button's largest side: `sheet.filter_button_max`.
    pub filter_button_max: f32,
    /// The filter button's inset from the header cell's corner: `sheet.filter_button_inset`.
    pub filter_button_inset: f32,
    /// The filter button's corner radius and its inset round the glyph: `sheet.filter_button_radius`,
    /// `sheet.filter_button_glyph_inset`.
    pub filter_button_radius: f32,
    /// The filter button's inset round the glyph: `sheet.filter_button_glyph_inset`.
    pub filter_button_glyph_inset: f32,
    /// The error checking tag's gap to its cell and corner radius: `sheet.error_tag_gap`,
    /// `sheet.error_tag_radius`.
    pub error_tag_gap: f32,
    /// The error checking tag's corner radius: `sheet.error_tag_radius`.
    pub error_tag_radius: f32,
    /// The gap between the status bar's statistics: `sheet.stats_gap`.
    pub stats_gap: f32,
    /// The type role of the status bar's statistics: the theme's label role.
    pub stats_text: TypeRole,
    /// How far from a header's far edge a press still grabs it, at 100 % zoom: `sheet.edge_grab`.
    pub edge_grab: f32,
    /// How far from a selection's border a press still moves it, at 100 % zoom: `sheet.border_grab`.
    pub border_grab: f32,
    /// The room after the last lane in an outline band, at any zoom: `sheet.outline_band_slack`.
    pub outline_band_slack: f32,
    /// The fill handle's side: `sheet.fill_handle`.
    pub fill_handle: f32,
    /// The accent bar on a selected header's inner edge: `sheet.header_accent_bar`.
    pub header_accent_bar: f32,
    /// The distance of each line of a hidden line's double divider from its edge:
    /// `sheet.hidden_mark_offset`.
    pub hidden_mark_offset: f32,
    /// How far the headers' separators stop short of the header's start:
    /// `sheet.header_separator_inset`.
    pub header_separator_inset: f32,
    /// The status bar menu's least width: `sheet.status_menu_min_width`.
    pub status_menu_min_width: f32,
    /// The field list checklist's greatest height and an area's least height:
    /// `sheet.field_list_checklist_max`, `sheet.field_list_area_min`.
    pub field_list_checklist_max: f32,
    /// An area's least height in the field list: `sheet.field_list_area_min`.
    pub field_list_area_min: f32,
    /// The selection's tint over the cells: `sheet.selection_fill`.
    pub selection_fill: Color32,
    /// The fill handle's fill and its one-point ring: `sheet.fill_handle_fill`,
    /// `sheet.fill_handle_ring`.
    pub fill_handle_fill: Color32,
    /// The fill handle's one-point ring colour: `sheet.fill_handle_ring`.
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
        let t = &tokens.component.sheet;
        SheetStyle {
            column_header_height: t.column_header_height,
            row_header_digit: t.row_header_digit,
            row_header_padding: t.row_header_padding,
            outline_lane: t.outline_lane,
            outline_button_margin: t.outline_button_margin,
            outline_button_min: t.outline_button_min,
            outline_inset: t.outline_inset,
            outline_hit: t.outline_hit,
            outline_radius: t.outline_radius,
            slicer_caption: TypeRole::BodyStrong,
            slicer_header: t.slicer_header,
            slicer_tile: t.slicer_tile,
            slicer_gap: t.slicer_gap,
            slicer_padding: t.slicer_padding,
            slicer_clear: t.slicer_clear,
            error_tag: t.error_tag,
            filter_button_min: t.filter_button_min,
            filter_button_max: t.filter_button_max,
            filter_button_inset: t.filter_button_inset,
            error_tag_gap: t.error_tag_gap,
            error_tag_radius: t.error_tag_radius,
            filter_button_radius: t.filter_button_radius,
            filter_button_glyph_inset: t.filter_button_glyph_inset,
            stats_gap: t.stats_gap,
            stats_text: TypeRole::Label,
            edge_grab: t.edge_grab,
            border_grab: t.border_grab,
            outline_band_slack: t.outline_band_slack,
            fill_handle: t.fill_handle,
            header_accent_bar: t.header_accent_bar,
            hidden_mark_offset: t.hidden_mark_offset,
            header_separator_inset: t.header_separator_inset,
            status_menu_min_width: t.status_menu_min_width,
            field_list_checklist_max: t.field_list_checklist_max,
            field_list_area_min: t.field_list_area_min,
            selection_fill: t.selection_fill.to_egui(),
            fill_handle_fill: t.fill_handle_fill.to_egui(),
            fill_handle_ring: t.fill_handle_ring.to_egui(),
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
