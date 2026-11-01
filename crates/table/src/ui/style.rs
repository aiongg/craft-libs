//! [`SheetStyle`]: the spreadsheet widgets' sizes, from the `sheet` tokens.

use craft_ui::theme::UiTokens;

/// The sizes the spreadsheet widgets read, one `sheet.*` token each, at 100 % sheet zoom (the
/// widgets scale them by the zoom). Proposed in the step 6b spec with GridCraft's values; a token
/// task adds the `sheet` family to craft-theme and fills [`of`](Self::of). The family is
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
    // new: one style per family (step 6a's `LayersStyle::of`); filled by the token task.
    #[expect(clippy::todo, unused_variables, reason = "skeleton: removed when the bodies land")]
    pub fn of(tokens: &UiTokens) -> SheetStyle {
        todo!()
    }
}
