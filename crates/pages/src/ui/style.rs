//! The page widgets' sizes and colours, one style per widget, each a pure function of
//! [`UiTokens`] (the `pages.*`, `thumbnails.*`, `margins.*`, `header_footer.*`, `text_selection.*`, `find.*`, `column_bar.*`,
//! `reading.*`, `frame_marks.*`, `edge_picker.*`, `table_size.*` and `cell_selection.*` component
//! tokens; names and source values in the spec's "Proposed tokens"). The token task fills every
//! function here from the tokens it adds; no body here or in a widget holds a literal size.
//!
//! Proportions of a token size (a hidden slide's opacity, a mark's share of the ascent) are named
//! constants beside their widget (lead call 38), not tokens.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the token task lands")]

use craft_ui::theme::UiTokens;
use egui::{Color32, Stroke};

use crate::layout::PageSpacing;

/// [`PageView`](super::PageView)'s spacing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageViewStyle {
    /// `pages.gap`, `pages.margin`, `pages.side`, `pages.white_space_pad`, `pages.seam`.
    pub spacing: PageSpacing,
}

impl PageViewStyle {
    pub fn of(tokens: &UiTokens) -> Self {
        todo!()
    }
}

/// [`PageThumbnails`](super::PageThumbnails)' sizes and colours.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageThumbnailsStyle {
    /// The thumbnail width at zoom 1 (`thumbnails.usual_width`).
    pub usual_width: f32,
    /// What the panel keeps free beside the thumbnails (`thumbnails.panel_inset`).
    pub panel_inset: f32,
    /// A cell's padding around its thumbnail (`thumbnails.cell_pad`).
    pub cell_pad: f32,
    /// Between rows of cells (`thumbnails.row_gap`).
    pub row_gap: f32,
    /// The current page's border, accent (`thumbnails.current_border`); others take
    /// `thumbnails.border` in `line.control_edge`.
    pub current_border: f32,
    pub border: f32,
    /// A picked page's outline, accent (`thumbnails.picked_outline`).
    pub picked_outline: f32,
    /// PowerPoint's outline round the current and picked slides and its offset outward
    /// (`thumbnails.strip_outline`, `thumbnails.strip_outline_offset`).
    pub strip_outline: f32,
    pub strip_outline_offset: f32,
    /// The drop bar's width, accent (`thumbnails.drop_bar`).
    pub drop_bar: f32,
    /// The selected-row wash (`list.selected_fill`, the list family's own token, as every
    /// selected row).
    pub wash: Color32,
    pub accent: Color32,
}

impl PageThumbnailsStyle {
    pub fn of(tokens: &UiTokens) -> Self {
        todo!()
    }
}

/// [`MarginHandles`](super::MarginHandles)' sizes and colours.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MarginHandlesStyle {
    /// How far either side of an edge grabs it (`margins.edge_grab`).
    pub edge_grab: f32,
    /// The margin part of a ruler (`ruler.margin_shade`, a new leaf of craft-ui's ruler family,
    /// which craft-text's `TextRuler` reads too); the text part keeps the ruler's fill.
    pub shade: Color32,
}

impl MarginHandlesStyle {
    pub fn of(tokens: &UiTokens) -> Self {
        todo!()
    }
}

/// [`HeaderFooterZones`](super::HeaderFooterZones)' sizes and strokes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeaderFooterStyle {
    /// A section's inset from its band, top and bottom (`header_footer.inset`).
    pub inset: f32,
    /// The dashed outline of a hovered empty section (`header_footer.hint_outline`, in
    /// `line.control_edge`) and its dash and gap (drawing details, constants).
    pub hint: Stroke,
    /// The edited section's outline, accent, and its neighbours' (`header_footer.edit_outline`).
    pub editing: Stroke,
    pub neighbour: Stroke,
}

impl HeaderFooterStyle {
    pub fn of(tokens: &UiTokens) -> Self {
        todo!()
    }
}

/// [`TextSelectTool`](super::TextSelectTool)'s and [`paint_text_marks`](super::paint_text_marks)'
/// colours and sizes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextSelectStyle {
    /// `text_selection.fill`, `find.match`, `find.current`.
    pub selection: Color32,
    pub found: Color32,
    pub current: Color32,
    /// Corner radii of the selection and match rects, and the current match's (`text_selection.radius`,
    /// `find.current_radius`); how much the current match grows (`find.current_grow`).
    pub radius: f32,
    pub current_radius: f32,
    pub current_grow: f32,
    /// The column rectangle's outline while dragging (`text_selection.column_outline`, accent).
    pub column_outline: Stroke,
}

impl TextSelectStyle {
    pub fn of(tokens: &UiTokens) -> Self {
        todo!()
    }
}

/// [`ColumnBar`](super::ColumnBar)'s sizes and colours.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColumnBarStyle {
    /// The bar's height: a control height (`size.control.md`).
    pub height: f32,
    /// The bar's outline (`column_bar.outline`, `line.control_edge`) and corner radius
    /// (`radius.control`).
    pub outline: Stroke,
    pub radius: f32,
    /// A divider at rest (`column_bar.divider`, `line.control_edge`) and hovered, dragged or
    /// focused (`column_bar.divider_active`, accent).
    pub divider: Stroke,
    pub divider_active: Stroke,
    /// How far either side of a divider grabs it (`column_bar.grab`).
    pub grab: f32,
}

impl ColumnBarStyle {
    pub fn of(tokens: &UiTokens) -> Self {
        todo!()
    }
}

/// [`ReadingView`](super::ReadingView)'s sizes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReadingStyle {
    /// Between columns (`reading.column_gap`).
    pub column_gap: f32,
    /// Room under the columns for the page-turn bar (`reading.nav_bar`).
    pub nav_bar: f32,
    /// One column per this much width, and the least column width, screen points
    /// (`reading.column_target`, `reading.column_min`).
    pub column_target: f32,
    pub column_min: f32,
}

impl ReadingStyle {
    pub fn of(tokens: &UiTokens) -> Self {
        todo!()
    }
}

/// [`TextFrameMarks`](super::TextFrameMarks)' sizes and colours.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameMarksStyle {
    /// A port's side and its distance from the frame's corner (`frame_marks.port`,
    /// `frame_marks.port_inset`).
    pub port: f32,
    pub port_inset: f32,
    /// The overset mark: its colour, arm length and stroke (`frame_marks.overset`,
    /// `frame_marks.overset_arm`, `frame_marks.overset_stroke`).
    pub overset: Color32,
    pub overset_arm: f32,
    pub overset_stroke: f32,
    /// The least and largest hidden-character mark size (`frame_marks.mark_min`, `frame_marks.mark_max`).
    pub mark_min: f32,
    pub mark_max: f32,
}

impl FrameMarksStyle {
    pub fn of(tokens: &UiTokens) -> Self {
        todo!()
    }
}

/// [`EdgePicker`](super::EdgePicker)'s sizes and colours.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgePickerStyle {
    /// The square's side and the lines' inset from it (`edge_picker.side`, `edge_picker.inset`).
    pub side: f32,
    pub inset: f32,
    /// The field inside the inset (`edge_picker.fill`), the square's border (`field.border`).
    pub fill: Color32,
    /// A chosen line (`edge_picker.line_on`, accent), a line not chosen (`edge_picker.line_off`,
    /// its colour `edge_picker.line_off_color`), the line under the pointer
    /// (`edge_picker.line_hover`).
    pub line_on: f32,
    pub line_off: f32,
    pub line_off_color: Color32,
    pub line_hover: f32,
}

impl EdgePickerStyle {
    pub fn of(tokens: &UiTokens) -> Self {
        todo!()
    }
}

/// [`TableSizePicker`](super::TableSizePicker)'s sizes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TableSizeStyle {
    /// A square's side and the space between squares (`table_size.cell`, `table_size.gap`).
    pub cell: f32,
    pub gap: f32,
}

impl TableSizeStyle {
    pub fn of(tokens: &UiTokens) -> Self {
        todo!()
    }
}

/// [`CellSelectionOverlay`](super::CellSelectionOverlay)'s colours.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellSelectionStyle {
    /// The selected cells' tint (`cell_selection.fill`) and the range outline (accent,
    /// `cell_selection.outline`).
    pub fill: Color32,
    pub outline: Stroke,
}

impl CellSelectionStyle {
    pub fn of(tokens: &UiTokens) -> Self {
        todo!()
    }
}
