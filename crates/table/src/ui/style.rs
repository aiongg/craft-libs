//! [`SheetStyle`]: the spreadsheet widgets' sizes, from the `sheet` tokens.

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
            slicer_header: t.slicer_header,
            slicer_tile: t.slicer_tile,
            slicer_gap: t.slicer_gap,
            slicer_padding: t.slicer_padding,
            slicer_clear: t.slicer_clear,
            error_tag: t.error_tag,
            filter_button_min: t.filter_button_min,
            filter_button_max: t.filter_button_max,
            filter_button_inset: t.filter_button_inset,
        }
    }
}
