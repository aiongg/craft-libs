//! The page widgets' sizes and colours, one style per widget, each a pure function of
//! [`UiTokens`] built by the widget's `style` function from the `pages.*` leaves (the one `pages`
//! family; names and source values in the spec's "Proposed tokens") and, for the rulers' margin
//! shade, `ruler.margin_shade`. No body here or in a widget holds a literal size.
//!
//! Proportions of a token size (a hidden slide's opacity, a mark's share of the ascent) are named
//! constants beside their widget (lead call 38), not tokens.

use craft_theme::TypeRole;
use egui::{Color32, Stroke};

use crate::layout::PageSpacing;

/// [`PageView`](super::PageView)'s spacing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageViewStyle {
    /// `pages.gap`, `pages.margin`, `pages.side`, `pages.white_space_pad`, `pages.seam`.
    pub spacing: PageSpacing,
    /// The page's paper (`pages.paper`).
    pub paper: Color32,
}

/// [`PagePreview`](super::PagePreview)'s measures.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PagePreviewStyle {
    /// The room kept around the previewed page (`pages.preview_room`).
    pub room: f32,
}

/// [`PageThumbnails`](super::PageThumbnails)' sizes: what sets the card side. Rings, the insert
/// line, captions and spacing are the grid's (`TileStyle`, `tile.*`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageThumbnailsStyle {
    /// The thumbnail width at zoom 1 (`pages.thumbnail_usual_width`).
    pub usual_width: f32,
    /// What the panel keeps free beside the thumbnails (`pages.thumbnail_panel_inset`).
    pub panel_inset: f32,
}

/// [`MarginHandles`](super::MarginHandles)' sizes and colours.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MarginHandlesStyle {
    /// How far either side of an edge grabs it (`pages.margin_handle_grab`).
    pub edge_grab: f32,
    /// The margin part of a ruler (`ruler.margin_shade`, a new leaf of craft-ui's ruler family,
    /// which craft-text's `TextRuler` reads too); the text part keeps the ruler's fill.
    pub shade: Color32,
    /// The text part of a ruler: the page's paper (`pages.paper`).
    pub paper: Color32,
}

/// [`HeaderFooterZones`](super::HeaderFooterZones)' sizes and strokes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeaderFooterStyle {
    /// A section's inset from its band, top and bottom (`pages.header_footer_inset`).
    pub inset: f32,
    /// The dashed outline of a hovered empty section (`pages.header_footer_hint_outline`, in
    /// `line.control_edge`) and its dash and gap (drawing details, constants).
    pub hint: Stroke,
    /// The edited section's outline, accent, and its neighbours' (`pages.header_footer_edit_outline`).
    pub editing: Stroke,
    pub neighbour: Stroke,
    /// The paper a section is cleared to and the ink its text takes (`pages.paper`, `pages.ink`).
    pub paper: Color32,
    pub ink: Color32,
}

/// [`TextSelectTool`](super::TextSelectTool)'s and [`paint_text_marks`](super::paint_text_marks)'
/// colours and sizes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextSelectStyle {
    /// `pages.text_selection_fill`, `pages.find_match`, `pages.find_current`.
    pub selection: Color32,
    pub found: Color32,
    pub current: Color32,
    /// Corner radii of the selection and match rects, and the current match's (`pages.text_selection_radius`,
    /// `pages.find_current_radius`); how much the current match grows (`pages.find_current_grow`).
    pub radius: f32,
    pub current_radius: f32,
    pub current_grow: f32,
    /// The column rectangle's outline while dragging (`pages.text_selection_column_outline`, accent).
    pub column_outline: Stroke,
}

/// [`ColumnBar`](super::ColumnBar)'s sizes and colours.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColumnBarStyle {
    /// The bar's height: a control height (`size.control.md`).
    pub height: f32,
    /// The bar's outline (`pages.column_bar_outline`, `line.control_edge`) and corner radius
    /// (`radius.control`).
    pub outline: Stroke,
    pub radius: f32,
    /// A divider at rest (`pages.column_bar_divider`, `line.control_edge`) and hovered, dragged or
    /// focused (`pages.column_bar_divider_active`, accent).
    pub divider: Stroke,
    pub divider_active: Stroke,
    /// How far either side of a divider grabs it (`pages.column_bar_grab`).
    pub grab: f32,
    /// The fill of the even and the odd columns (`pages.column_bar_fill`, `pages.column_bar_fill_alt`).
    pub fill: Color32,
    pub fill_alt: Color32,
    /// The type role of the column names.
    pub label: TypeRole,
}

/// [`ReadingView`](super::ReadingView)'s sizes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReadingStyle {
    /// Between columns (`pages.reading_column_gap`).
    pub column_gap: f32,
    /// Room under the columns for the page-turn bar (`pages.reading_nav_bar`).
    pub nav_bar: f32,
    /// One column per this much width, and the least column width, screen points
    /// (`pages.reading_column_target`, `pages.reading_column_min`).
    pub column_target: f32,
    pub column_min: f32,
}

/// [`TextFrameMarks`](super::TextFrameMarks)' sizes and colours.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameMarksStyle {
    /// A port's side and its distance from the frame's corner (`pages.frame_marks_port`,
    /// `pages.frame_marks_port_inset`).
    pub port: f32,
    pub port_inset: f32,
    /// The overset mark: its colour, arm length and stroke (`pages.frame_marks_overset`,
    /// `pages.frame_marks_overset_arm`, `pages.frame_marks_overset_stroke`).
    pub overset: Color32,
    pub overset_arm: f32,
    pub overset_stroke: f32,
    /// The least and largest hidden-character mark size (`pages.frame_marks_mark_min`, `pages.frame_marks_mark_max`).
    pub mark_min: f32,
    pub mark_max: f32,
}

/// [`EdgePicker`](super::EdgePicker)'s sizes and colours.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgePickerStyle {
    /// The square's side and the lines' inset from it (`pages.edge_picker_side`, `pages.edge_picker_inset`).
    pub side: f32,
    pub inset: f32,
    /// The field inside the inset (`pages.edge_picker_fill`), the square's border (`field.border`).
    pub fill: Color32,
    /// A chosen line (`pages.edge_picker_line_on`, accent), a line not chosen (`pages.edge_picker_line_off`,
    /// its colour `pages.edge_picker_line_off_color`), the line under the pointer
    /// (`pages.edge_picker_line_hover`).
    pub line_on: f32,
    pub line_off: f32,
    pub line_off_color: Color32,
    pub line_hover: f32,
}

/// [`TableSizePicker`](super::TableSizePicker)'s sizes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TableSizeStyle {
    /// A square's side and the space between squares (`pages.table_size_cell`, `pages.table_size_gap`).
    pub cell: f32,
    pub gap: f32,
    /// The type role of the size label above the squares.
    pub label: TypeRole,
}

/// [`CellSelectionOverlay`](super::CellSelectionOverlay)'s colours.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellSelectionStyle {
    /// The selected cells' tint (`pages.table_selection_fill`) and the range outline (accent,
    /// `pages.table_selection_outline`).
    pub fill: Color32,
    pub outline: Stroke,
}
