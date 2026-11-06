//! The page widgets' sizes and colours, one style per widget, each a pure function of
//! [`UiTokens`](craft_ui::theme::UiTokens) built by the widget's `style` function from the semantic tokens.
//!
//! Proportions of a token size (a hidden slide's opacity, a mark's share of the ascent) are named
//! constants beside their widget.

use craft_theme::TypeRole;
use egui::{Color32, Stroke};

use crate::layout::PageSpacing;

/// The default paper of a page and of the sections cleared over it; the app's own colour replaces it.
pub const DEFAULT_PAPER: Color32 = Color32::WHITE;
/// The default ink of header and footer text.
pub const DEFAULT_INK: Color32 = Color32::BLACK;

/// [`PageView`](super::PageView)'s spacing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageViewStyle {
    /// The gaps, margins, side column, Hide White Space pad and seam.
    pub spacing: PageSpacing,
    /// The page's paper.
    pub paper: Color32,
}

/// [`PagePreview`](super::PagePreview)'s measures.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PagePreviewStyle {
    /// The room kept around the previewed page (`space.pad.dialog`).
    pub room: f32,
}

/// [`PageThumbnails`](super::PageThumbnails)' sizes: what sets the card side. Rings, the insert
/// line, captions and spacing are the grid's (`TileStyle`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageThumbnailsStyle {
    /// The thumbnail width at zoom 1 (a constant).
    pub usual_width: f32,
    /// What the panel keeps free beside the thumbnails (a constant).
    pub panel_inset: f32,
}

/// [`MarginHandles`](super::MarginHandles)' sizes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MarginHandlesStyle {
    /// How far either side of an edge grabs it (`size.hit.slop`).
    pub edge_grab: f32,
}

/// [`HeaderFooterZones`](super::HeaderFooterZones)' sizes and strokes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeaderFooterStyle {
    /// A section's inset from its band, top and bottom (`space.gap.tight`).
    pub inset: f32,
    /// The dashed outline of a hovered empty section (`stroke.thin`, in
    /// `line.control_edge`) and its dash and gap (drawing details, constants).
    pub hint: Stroke,
    /// The edited section's outline, accent, and its neighbours' (`stroke.thin`).
    pub editing: Stroke,
    #[allow(missing_docs)]
    pub neighbour: Stroke,
    /// The paper a section is cleared to and the ink its text takes ([`DEFAULT_PAPER`], [`DEFAULT_INK`]).
    pub paper: Color32,
    #[allow(missing_docs)]
    pub ink: Color32,
}

/// [`TextSelectTool`](super::TextSelectTool)'s and [`paint_text_marks`](super::paint_text_marks)'
/// colours and sizes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextSelectStyle {
    /// `state.text_selection`, `state.match`, `state.match_current`.
    pub selection: Color32,
    #[allow(missing_docs)]
    pub found: Color32,
    #[allow(missing_docs)]
    pub current: Color32,
    /// Corner radii of the selection and match rects, and the current match's (`shape.radius.sm`); how much the current match grows (`stroke.thin`).
    pub radius: f32,
    #[allow(missing_docs)]
    pub current_radius: f32,
    #[allow(missing_docs)]
    pub current_grow: f32,
    /// The column rectangle's outline while dragging (`stroke.thin`, accent).
    pub column_outline: Stroke,
}

/// [`ColumnBar`](super::ColumnBar)'s sizes and colours.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColumnBarStyle {
    /// The bar's height: a control height (`size.control.md`).
    pub height: f32,
    /// The bar's outline (`stroke.thin`, `line.control_edge`) and corner radius
    /// (`radius.control`).
    pub outline: Stroke,
    #[allow(missing_docs)]
    pub radius: f32,
    /// A divider at rest (`stroke.medium`, `line.control_edge`) and hovered, dragged or
    /// focused (accent).
    pub divider: Stroke,
    #[allow(missing_docs)]
    pub divider_active: Stroke,
    /// How far either side of a divider grabs it (`size.hit.slop`).
    pub grab: f32,
    /// The fill of the even and the odd columns (`accent.solid` at a low alpha).
    pub fill: Color32,
    #[allow(missing_docs)]
    pub fill_alt: Color32,
    /// The type role of the column percentages (PdfCraft: medium 11 pt; the caption role is the
    /// nearest), and the narrowest column that paints one.
    pub label: TypeRole,
    #[allow(missing_docs)]
    pub label_min: f32,
}

/// [`ReadingView`](super::ReadingView)'s sizes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReadingStyle {
    /// Between columns (`space.gap.section`).
    pub column_gap: f32,
    /// Room under the columns for the page-turn bar (`size.control.md + 2 * space.pad.control`).
    pub nav_bar: f32,
    /// One column per this much width, and the least column width, screen points.
    pub column_target: f32,
    #[allow(missing_docs)]
    pub column_min: f32,
}

/// [`TextFrameMarks`](super::TextFrameMarks)' sizes and colours.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameMarksStyle {
    /// A port's side and its distance from the frame's corner (`size.handle.md`,
    /// `space.pad.control`).
    pub port: f32,
    #[allow(missing_docs)]
    pub port_inset: f32,
    /// The overset mark: its colour, arm length and stroke (`status.bad.solid`,
    /// a constant arm, `stroke.medium`).
    pub overset: Color32,
    #[allow(missing_docs)]
    pub overset_arm: f32,
    #[allow(missing_docs)]
    pub overset_stroke: f32,
    /// The least and largest hidden-character mark size.
    pub mark_min: f32,
    #[allow(missing_docs)]
    pub mark_max: f32,
    /// The fill of a port ([`DEFAULT_PAPER`]).
    pub paper: Color32,
}

/// [`EdgePicker`](super::EdgePicker)'s sizes and colours.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgePickerStyle {
    /// The square's side and the lines' inset from it (a constant, `space.pad.panel`).
    pub side: f32,
    #[allow(missing_docs)]
    pub inset: f32,
    /// The field inside the inset (`surface.sunken`), the square's border (the border).
    pub fill: Color32,
    /// A chosen line (`stroke.thick`, accent), a line not chosen (`stroke.thin`,
    /// its colour `text.faint`), the line under the pointer
    /// (a constant).
    pub line_on: f32,
    #[allow(missing_docs)]
    pub line_off: f32,
    #[allow(missing_docs)]
    pub line_off_color: Color32,
    #[allow(missing_docs)]
    pub line_hover: f32,
}

/// [`TableSizePicker`](super::TableSizePicker)'s sizes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TableSizeStyle {
    /// A square's side and the space between squares (`size.icon.md`, `space.gap.tight`).
    pub cell: f32,
    #[allow(missing_docs)]
    pub gap: f32,
    /// The type role of the size label above the squares.
    pub label: TypeRole,
}

/// [`CellSelectionOverlay`](super::CellSelectionOverlay)'s colours.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellSelectionStyle {
    /// The selected cells' tint (`accent.solid` at a low alpha) and the range outline (accent,
    /// `stroke.thin`).
    pub fill: Color32,
    #[allow(missing_docs)]
    pub outline: Stroke,
}
