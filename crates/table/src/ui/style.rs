//! [`SheetStyle`]: the spreadsheet widgets' sizes, from the `sheet` tokens.

use craft_theme::TypeRole;
use craft_ui::theme::UiTokens;

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
    pub row_header_padding: f32,
    /// One outline lane: `sheet.outline_lane`.
    pub outline_lane: f32,
    /// What an outline button's side is short of the lane, its least side, the first lane's inset,
    /// how far a button's hit area reaches past its square and its corner radius:
    /// `sheet.outline_button_margin`, `sheet.outline_button_min`, `sheet.outline_inset`,
    /// `sheet.outline_hit`, `sheet.outline_radius`.
    pub outline_button_margin: f32,
    pub outline_button_min: f32,
    pub outline_inset: f32,
    pub outline_hit: f32,
    pub outline_radius: f32,
    /// The type role of a slicer's caption: the theme's strong body role.
    pub slicer_caption: TypeRole,
    /// A slicer's header, value tile, gap between tiles, padding round the tiles and clear
    /// button: `sheet.slicer_header`, `sheet.slicer_tile`, `sheet.slicer_gap`,
    /// `sheet.slicer_padding`, `sheet.slicer_clear`.
    pub slicer_header: f32,
    pub slicer_tile: f32,
    pub slicer_gap: f32,
    pub slicer_padding: f32,
    pub slicer_clear: f32,
    /// The error checking tag's square: `sheet.error_tag`.
    pub error_tag: f32,
    /// The filter button's least and largest side and its inset from the header cell's corner:
    /// `sheet.filter_button_min`, `sheet.filter_button_max`, `sheet.filter_button_inset`.
    pub filter_button_min: f32,
    pub filter_button_max: f32,
    pub filter_button_inset: f32,
    /// The filter button's corner radius and its inset round the glyph: `sheet.filter_button_radius`,
    /// `sheet.filter_button_glyph_inset`.
    pub filter_button_radius: f32,
    pub filter_button_glyph_inset: f32,
    /// The error checking tag's gap to its cell, corner radius, warning diamond's half-diagonal and
    /// exclamation mark's text size: `sheet.error_tag_gap`, `sheet.error_tag_radius`,
    /// `sheet.error_tag_mark`, `sheet.error_tag_mark_text`.
    pub error_tag_gap: f32,
    pub error_tag_radius: f32,
    pub error_tag_mark: f32,
    pub error_tag_mark_text: f32,
    /// A dashed chart line's dash and gap: `sheet.chart_dash`, `sheet.chart_dash_gap`.
    pub chart_dash: f32,
    pub chart_dash_gap: f32,
    /// The gap between the status bar's statistics: `sheet.stats_gap`.
    pub stats_gap: f32,
    /// The type role of the status bar's statistics: the theme's label role.
    pub stats_text: TypeRole,
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
            chart_dash: t.chart_dash,
            chart_dash_gap: t.chart_dash_gap,
            error_tag_gap: t.error_tag_gap,
            error_tag_radius: t.error_tag_radius,
            error_tag_mark: t.error_tag_mark,
            error_tag_mark_text: t.error_tag_mark_text,
            filter_button_radius: t.filter_button_radius,
            filter_button_glyph_inset: t.filter_button_glyph_inset,
            stats_gap: t.stats_gap,
            stats_text: TypeRole::Label,
        }
    }
}
