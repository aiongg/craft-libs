//! The spreadsheet cell grid: addresses in A1 notation, the sheet's geometry and the grid component.
//!
//! The grid is a collection on `SheetLayout`: one item per visible row ([`GridRow`]), cells as parts of the row, the
//! cell selection and the cell rules of `craft_behavior`, and the in-cell editor on the editor contract. The column
//! header strip, the row headers, the corner, the header edges, the fill handle and the range border are the grid's own
//! parts and paint.

pub use craft_behavior::{CellAdvance, CellAt, CellDirection, CellRange, CellSelection, CellSpace, LineIndex, Positions, SHEET_ZOOM};
use craft_behavior::{DisplayOrder, EditEvent, Insets, ItemSource, Metrics, SheetLayout};
use craft_ui::contract::{
    Bind, BindMut, Collection, CollectionEvent, CollectionProps, CollectionResponse, Common, ContainerProps, ContextMenuRequest, Cx, Label, Outcome,
    PaintUi, Props as _, Remember as _, Role, SelectMode, Sizing,
};
use craft_ui::runtime::focus::{KeepKeys, KeyUse};
use craft_ui::style::{ComponentStyle, StyleColor, StyleCx, SurfaceLook};
use egui::{Pos2, Rect, Response, Vec2};

use super::filter::FilterButtonEvent;
use super::outline::{OutlineAction, OutlineAxis};

mod editor;
mod input;
mod paint;
mod row;
#[cfg(feature = "testing")]
mod stories;

pub use row::GridRow;

/// Rows in a sheet, Excel's 1,048,576.
pub const SHEET_ROWS: u32 = 1_048_576;

/// Columns in a sheet, Excel's 16,384 (A to XFD).
pub const SHEET_COLUMNS: u32 = 16_384;

/// The absolute reference `$B$3` of a cell (point mode inserts these).
pub fn absolute_reference(cell: CellAt<u32>) -> String {
    format!("${}${}", column_letters(cell.column), cell.row + 1)
}

/// `$A$1:$C$4`, or `$A$1` for one cell.
pub fn absolute_range_reference(range: CellRange<u32>) -> String {
    let (top, bottom, left, right) = edges(&range);
    if top == bottom && left == right {
        absolute_reference(CellAt::new(top, left))
    } else {
        format!("{}:{}", absolute_reference(CellAt::new(top, left)), absolute_reference(CellAt::new(bottom, right)))
    }
}

/// `A`, `Z`, `AA`, `XFD` for column 0, 25, 26, 16383.
pub fn column_letters(column: u32) -> String {
    let mut rest = u64::from(column) + 1;
    let mut letters = Vec::new();
    while rest > 0 {
        rest -= 1;
        letters.push(char::from(b'A' + (rest % 26) as u8));
        rest /= 26;
    }
    letters.iter().rev().collect()
}

/// The top row, bottom row, left column and right column of `range`.
pub(crate) fn edges(range: &CellRange<u32>) -> (u32, u32, u32, u32) {
    let rows = range.rows();
    let columns = range.column_span();
    (*rows.start(), *rows.end(), *columns.start(), *columns.end())
}

/// The least zoom the column headers' height follows: below it they stay at this share of their 100 % height, so the
/// letters stay readable. A proportion, not a size.
pub const HEADER_MIN_ZOOM: f32 = 0.75;

/// How far below the window (points at 100 %) the row headers look for the last row whose digits they must hold, so
/// the header does not resize while scrolling.
const ROW_HEADER_LOOKAHEAD: f64 = 200.0;

/// The digits the row headers hold for rows up to `last` (at least two).
pub(crate) fn digits_of(last: u32) -> u32 {
    (last.saturating_add(1).ilog10() + 1).max(2)
}

/// The header sizes geometry needs, taken from the grid's style.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SheetMetrics {
    /// The column headers' height at 100 % zoom.
    pub column_header: f32,
    /// The width of one digit of the row headers at 100 % zoom.
    pub row_header_digit: f32,
    /// The row headers' padding at 100 % zoom.
    pub row_header_pad: f32,
}

impl SheetMetrics {
    /// Header sizes of `column_header` high, `row_header_digit` per digit and `row_header_pad` of padding.
    pub const fn new(column_header: f32, row_header_digit: f32, row_header_pad: f32) -> Self {
        SheetMetrics { column_header, row_header_digit, row_header_pad }
    }

    /// The row headings' width by the column headings' height at `zoom` for row numbers of `digits` digits; zero
    /// without headings. Headings shrink no further than [`HEADER_MIN_ZOOM`].
    pub fn header(&self, headings: bool, zoom: f32, digits: u32) -> Vec2 {
        if !headings {
            return Vec2::ZERO;
        }
        let zoom = zoom.max(HEADER_MIN_ZOOM);
        Vec2::new(((digits.max(2) as f32 * self.row_header_digit + self.row_header_pad) * zoom).round(), (self.column_header * zoom).round())
    }
}

/// What a grid's geometry is made from, per frame.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct GridLayout {
    /// The grid's rectangle on screen, in points.
    pub rect: Rect,
    /// The zoom, within [`SHEET_ZOOM`].
    pub zoom: f32,
    /// Row heights.
    pub rows: LineIndex,
    /// Column widths.
    pub columns: LineIndex,
    /// Frozen rows and columns at the top and start.
    pub frozen: (u32, u32),
    /// Scroll of the unfrozen part, in points at 100 %.
    pub scroll: Vec2,
    /// Whether the row and column headings are shown.
    pub headings: bool,
    /// The outline bands' thickness before the row headers and above the column headers.
    pub bands: Vec2,
    /// Whether the sheet is mirrored for a right-to-left script.
    pub right_to_left: bool,
    /// The digits the row headers hold; read from the rows shown when `None`.
    pub digits: Option<u32>,
}

impl GridLayout {
    /// A layout in `rect` over `rows` and `columns` at 100 % zoom, unscrolled, with headings.
    pub fn new(rect: Rect, rows: LineIndex, columns: LineIndex) -> Self {
        GridLayout {
            rect,
            zoom: 1.0,
            rows,
            columns,
            frozen: (0, 0),
            scroll: Vec2::ZERO,
            headings: true,
            bands: Vec2::ZERO,
            right_to_left: false,
            digits: None,
        }
    }

    /// The zoom.
    #[must_use]
    pub fn zoom(mut self, zoom: f32) -> Self {
        self.zoom = zoom;
        self
    }

    /// The frozen rows and columns.
    #[must_use]
    pub fn frozen(mut self, rows: u32, columns: u32) -> Self {
        self.frozen = (rows, columns);
        self
    }

    /// The scroll of the unfrozen part, in points at 100 %.
    #[must_use]
    pub fn scroll(mut self, scroll: Vec2) -> Self {
        self.scroll = scroll;
        self
    }

    /// Whether the headings show.
    #[must_use]
    pub fn headings(mut self, on: bool) -> Self {
        self.headings = on;
        self
    }

    /// The outline bands' thickness.
    #[must_use]
    pub fn bands(mut self, bands: Vec2) -> Self {
        self.bands = bands;
        self
    }

    /// Whether the sheet is mirrored.
    #[must_use]
    pub fn right_to_left(mut self, on: bool) -> Self {
        self.right_to_left = on;
        self
    }

    /// The digits the row headers hold.
    #[must_use]
    pub fn digits(mut self, digits: u32) -> Self {
        self.digits = Some(digits);
        self
    }
}

/// The geometry of a grid for one frame: where each cell is on screen, and which cell is at a point. Logical
/// coordinates run left to right; a right-to-left sheet is mirrored across the rect's centre line on screen
/// ([`mirror_x`](Self::mirror_x)), so layout and hit tests are one code path.
#[derive(Clone, Debug)]
pub struct SheetGeometry {
    layout: GridLayout,
    cells: Rect,
    header: Vec2,
    frozen_size: Vec2,
}

impl SheetGeometry {
    /// Column headers `metrics.column_header` high at 100 % zoom (zoomed, but never below [`HEADER_MIN_ZOOM`] of it),
    /// row headers wide for the digits of the last visible row plus 200 (or `layout.digits`), none without headings;
    /// cells after them and the bands.
    pub fn new(mut layout: GridLayout, metrics: SheetMetrics) -> Self {
        let (least, greatest) = (*SHEET_ZOOM.start(), *SHEET_ZOOM.end());
        layout.zoom = if layout.zoom.is_finite() { layout.zoom.clamp(least, greatest) } else { 1.0 };
        layout.frozen = (layout.frozen.0.min(layout.rows.count()), layout.frozen.1.min(layout.columns.count()));
        let (zoom, rect) = (layout.zoom, layout.rect);
        let digits = layout.digits.unwrap_or_else(|| {
            let last_row = layout.rows.at(f64::from(layout.scroll.y + rect.height() / zoom) + ROW_HEADER_LOOKAHEAD);
            digits_of(last_row)
        });
        let header = metrics.header(layout.headings, zoom, digits);
        let cells = Rect::from_min_max(Pos2::new(rect.left() + layout.bands.x + header.x, rect.top() + layout.bands.y + header.y), rect.max);
        let frozen_size = Vec2::new(layout.columns.start(layout.frozen.1) as f32 * zoom, layout.rows.start(layout.frozen.0) as f32 * zoom);
        Self { layout, cells, header, frozen_size }
    }

    /// The zoom the geometry was built at.
    pub fn zoom(&self) -> f32 {
        self.layout.zoom
    }

    /// The outline bands' thickness before the row headers and above the column headers.
    pub fn bands(&self) -> Vec2 {
        self.layout.bands
    }

    /// The frozen rows and columns, at most the sheet's.
    pub fn frozen(&self) -> (u32, u32) {
        self.layout.frozen
    }

    /// The screen-logical position where the scrolling cells begin: after the frozen columns and rows.
    pub fn split(&self) -> Pos2 {
        Pos2::new(self.cells.left() + self.frozen_size.x, self.cells.top() + self.frozen_size.y)
    }

    /// The grid's rectangle.
    pub fn rect(&self) -> Rect {
        self.layout.rect
    }

    /// The cells' area (after the row headers, below the column headers).
    pub fn cells(&self) -> Rect {
        self.cells
    }

    /// The row headings' width by the column headings' height, in screen points; zero without headings.
    pub fn header_size(&self) -> Vec2 {
        self.header
    }

    /// The row heights.
    pub fn rows(&self) -> &LineIndex {
        &self.layout.rows
    }

    /// The column widths.
    pub fn columns(&self) -> &LineIndex {
        &self.layout.columns
    }

    /// The left x of `column` (frozen columns do not scroll).
    pub fn x(&self, column: u32) -> f32 {
        let (columns, zoom) = (&self.layout.columns, self.layout.zoom);
        if column < self.layout.frozen.1 {
            self.cells.left() + columns.start(column) as f32 * zoom
        } else {
            let from_split = columns.start(column) - columns.start(self.layout.frozen.1) - f64::from(self.layout.scroll.x);
            self.cells.left() + self.frozen_size.x + from_split as f32 * zoom
        }
    }

    /// The top y of `row` (frozen rows do not scroll).
    pub fn y(&self, row: u32) -> f32 {
        let (rows, zoom) = (&self.layout.rows, self.layout.zoom);
        if row < self.layout.frozen.0 {
            self.cells.top() + rows.start(row) as f32 * zoom
        } else {
            let from_split = rows.start(row) - rows.start(self.layout.frozen.0) - f64::from(self.layout.scroll.y);
            self.cells.top() + self.frozen_size.y + from_split as f32 * zoom
        }
    }

    /// The rectangle of cell `c`, in logical coordinates.
    pub fn cell_rect(&self, c: CellAt<u32>) -> Rect {
        let (x, y) = (self.x(c.column), self.y(c.row));
        let size = Vec2::new(self.layout.columns.size(c.column) as f32, self.layout.rows.size(c.row) as f32) * self.layout.zoom;
        Rect::from_min_size(Pos2::new(x, y), size)
    }

    /// The rectangle covering `r`, in logical coordinates.
    pub fn range_rect(&self, r: &CellRange<u32>) -> Rect {
        let (top, bottom, left, right) = edges(r);
        Rect::from_min_max(self.cell_rect(CellAt::new(top, left)).min, self.cell_rect(CellAt::new(bottom, right)).max)
    }

    /// The column at logical x.
    pub fn column_at(&self, x: f32) -> u32 {
        let (columns, zoom, frozen) = (&self.layout.columns, self.layout.zoom, self.layout.frozen.1);
        let from_left = f64::from((x - self.cells.left()) / zoom);
        if from_left < f64::from(self.frozen_size.x / zoom) {
            columns.at(from_left.max(0.0))
        } else {
            let past_split = from_left - f64::from(self.frozen_size.x / zoom);
            columns.at(columns.start(frozen) + f64::from(self.layout.scroll.x) + past_split)
        }
    }

    /// The row at logical y.
    pub fn row_at(&self, y: f32) -> u32 {
        let (rows, zoom, frozen) = (&self.layout.rows, self.layout.zoom, self.layout.frozen.0);
        let from_top = f64::from((y - self.cells.top()) / zoom);
        if from_top < f64::from(self.frozen_size.y / zoom) {
            rows.at(from_top.max(0.0))
        } else {
            let past_split = from_top - f64::from(self.frozen_size.y / zoom);
            rows.at(rows.start(frozen) + f64::from(self.layout.scroll.y) + past_split)
        }
    }

    /// The cell at the logical position `p`.
    pub fn cell_at(&self, p: Pos2) -> CellAt<u32> {
        CellAt::new(self.row_at(p.y), self.column_at(p.x))
    }

    /// The visible scrolled rows and columns (after the frozen ones), inclusive.
    pub fn visible(&self) -> ((u32, u32), (u32, u32)) {
        let GridLayout { rows, columns, frozen, scroll, zoom, .. } = &self.layout;
        let span = |lines: &LineIndex, frozen: u32, scroll: f32, room: f32| {
            let top = lines.start(frozen) + f64::from(scroll);
            (lines.at(top).max(frozen), lines.at(top + f64::from(room / zoom)).min(lines.count().saturating_sub(1)))
        };
        (
            span(rows, frozen.0, scroll.y, self.cells.height() - self.frozen_size.y),
            span(columns, frozen.1, scroll.x, self.cells.width() - self.frozen_size.x),
        )
    }

    /// The panes (frozen corner, frozen rows, frozen columns, scrolling) that hold part of `r`, as screen rects to clip
    /// to, so a range scrolled under the frozen part is not drawn over it.
    pub fn range_panes(&self, r: &CellRange<u32>) -> Vec<Rect> {
        let (frozen_rows, frozen_columns) = self.layout.frozen;
        let (top, bottom, left, right) = edges(r);
        let split = Pos2::new(self.cells.left() + self.frozen_size.x, self.cells.top() + self.frozen_size.y);
        let mut xs = Vec::new();
        if left < frozen_columns {
            xs.push((self.cells.left(), split.x));
        }
        if right >= frozen_columns {
            xs.push((split.x, self.cells.right()));
        }
        let mut ys = Vec::new();
        if top < frozen_rows {
            ys.push((self.cells.top(), split.y));
        }
        if bottom >= frozen_rows {
            ys.push((split.y, self.cells.bottom()));
        }
        ys.iter()
            .flat_map(|&(top, bottom)| xs.iter().map(move |&(left, right)| Rect::from_min_max(Pos2::new(left, top), Pos2::new(right, bottom))))
            .collect()
    }

    /// Logical x to screen x and back (identity left to right).
    pub fn mirror_x(&self, x: f32) -> f32 {
        if self.layout.right_to_left { self.layout.rect.left() + self.layout.rect.right() - x } else { x }
    }

    /// `p` with its x mirrored; see [`mirror_x`](Self::mirror_x).
    pub fn mirror_pos(&self, p: Pos2) -> Pos2 {
        Pos2::new(self.mirror_x(p.x), p.y)
    }

    /// `r` with its x range mirrored; see [`mirror_x`](Self::mirror_x).
    pub fn mirror_rect(&self, r: Rect) -> Rect {
        let (a, b) = (self.mirror_x(r.left()), self.mirror_x(r.right()));
        Rect::from_x_y_ranges(a.min(b)..=a.max(b), r.y_range())
    }
}

/// The workbook sheet the grid shows; the app implements it on its sheet. Read only: the grid reports what the user
/// asks for as [`GridEdit`]s and moves the selection in [`GridState`].
pub trait GridModel {
    /// Row heights (points at 100 %), with hidden rows at 0.
    fn rows(&self) -> &LineIndex;

    /// Column widths in points at 100 %, with hidden columns at 0.
    fn columns(&self) -> &LineIndex;

    /// The number of frozen rows and columns; `(0, 0)` by default.
    fn frozen(&self) -> (u32, u32) {
        (0, 0)
    }

    /// The merged cells; clicks, arrows and ranges treat each as one cell.
    fn merges(&self) -> &[CellRange<u32>] {
        &[]
    }

    /// Whether a click or an arrow may land on `cell` (a protected sheet with "Select unlocked cells" only).
    fn selectable(&self, cell: CellAt<u32>) -> bool {
        let _ = cell;
        true
    }

    /// Cmd/Ctrl+arrow (End mode): the edge of the data region from `from` toward `direction`, which only the app's
    /// cells know. The default goes to the sheet's edge.
    fn data_edge(&self, from: CellAt<u32>, direction: CellDirection) -> CellAt<u32> {
        let (rows, columns) = (self.rows().count().saturating_sub(1), self.columns().count().saturating_sub(1));
        match direction {
            CellDirection::Up => CellAt::new(0, from.column),
            CellDirection::Down => CellAt::new(rows, from.column),
            CellDirection::Left => CellAt::new(from.row, 0),
            CellDirection::Right => CellAt::new(from.row, columns),
            _ => from,
        }
    }

    /// Cmd/Ctrl+End: the last cell with content (the bottom row and right column in use).
    fn last_used(&self) -> CellAt<u32>;

    /// Whether the grid draws the gridlines.
    fn show_gridlines(&self) -> bool {
        true
    }

    /// Paints cell `cell` (value, format, borders, conditional format) into `rect` at sheet `zoom` (1.0 = 100 %), so
    /// the app scales its text and borders with the grid. The grid draws the gridlines (none inside a merged cell),
    /// headers and selection; the app everything in the cell.
    fn paint_cell(&self, ui: &mut PaintUi<'_>, rect: Rect, cell: CellAt<u32>, zoom: f32);

    /// The column header's text: the letters, or a table's header name.
    fn column_label(&self, column: u32) -> String {
        column_letters(column)
    }

    /// The cell's text for AccessKit, the editor's start text and the copy fallback.
    fn cell_text(&self, cell: CellAt<u32>) -> String;
}

/// What the grid keeps between passes; the app keeps one per sheet view and saves it with the view.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct GridState {
    /// The selected cells.
    pub selection: CellSelection<u32>,
    /// The zoom, within [`SHEET_ZOOM`]; 1.0 for a new sheet.
    pub zoom: f32,
}

impl Default for GridState {
    /// A1 selected at 100 %.
    fn default() -> Self {
        Self { selection: CellSelection::cell(CellAt::new(0, 0)), zoom: 1.0 }
    }
}

/// The text the in-cell editor committed.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct CellEdit {
    /// The cell.
    pub cell: CellAt<u32>,
    /// The text to write into it.
    pub text: String,
}

impl CellEdit {
    /// An edit writing `text` into `cell`.
    pub fn new(cell: CellAt<u32>, text: impl Into<String>) -> Self {
        CellEdit { cell, text: text.into() }
    }
}

/// What the grid asks of the app.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum GridEdit {
    /// A cell was opened for editing: typing replaced its content (`typed`), or F2 or a double click edits in place
    /// (`None`). The app may show the draft in its formula bar from [`GridOutput::draft`].
    BeginEdit {
        /// Replacement text typed on the grid; `None` edits in place.
        typed: Option<String>,
    },
    /// Delete: clear the selection's contents.
    Clear,
    /// Backspace: clear the active cell and edit it.
    ClearAndEdit,
    /// A column header's end edge dragged: the new width in points at 100 % zoom, as one gesture (Esc cancels). Several
    /// selected columns resize together: the app applies the width to every column of the selection that holds
    /// `column`.
    ResizeColumn {
        /// The column whose edge was dragged.
        column: u32,
        /// The new width, in points at 100 % zoom.
        width: EditEvent<f32>,
    },
    /// A row header's bottom edge dragged, as [`ResizeColumn`](Self::ResizeColumn).
    ResizeRow {
        /// The row whose edge was dragged.
        row: u32,
        /// The new height, in points at 100 % zoom.
        height: EditEvent<f32>,
    },
    /// A double-click on a column header's edge: AutoFit Column Width.
    AutoFitColumn {
        /// The column whose edge was double-clicked.
        column: u32,
    },
    /// A double-click on a row header's edge: AutoFit Row Height.
    AutoFitRow {
        /// The row whose edge was double-clicked.
        row: u32,
    },
    /// A double-click on the edge where a hidden column sits: Unhide.
    UnhideColumn {
        /// The hidden column.
        column: u32,
    },
    /// A double-click on the edge where a hidden row sits.
    UnhideRow {
        /// The hidden row.
        row: u32,
    },
    /// The fill handle dragged: the range the current selection grows to (the source plus the filled cells), as one
    /// gesture. `Update`s show the preview, `Commit` carries the range, `Cancel` (Esc) drops it. Cmd/Ctrl at the
    /// commit swaps copying and extending a series ([`GridOutput::fill_swapped`]).
    Fill {
        /// The range the selection grows to.
        range: EditEvent<CellRange<u32>>,
    },
    /// The selection dragged by its border: the cell its top-left corner moves to, as one gesture like
    /// [`Fill`](Self::Fill). The app moves the cells; the grid selects the moved range when the gesture commits.
    MoveCells {
        /// The cell the selection's top-left corner moves to.
        to: EditEvent<CellAt<u32>>,
    },
    /// The fill handle dragged with the secondary button and released: the range the drag grew the selection to, and
    /// the pointer's screen position. The app opens its fill menu there; nothing is filled or selected until then.
    FillMenu {
        /// The range the drag grew the selection to.
        range: CellRange<u32>,
        /// The pointer position in screen points.
        anchor: Pos2,
    },
    /// Esc while [`CellGrid::copy_mark`] shows the marching ants: the app ends copy mode.
    ClearCopyMark,
    /// Enter while [`CellGrid::copy_mark`] shows the marching ants: paste what was copied at the active cell and end
    /// copy mode.
    PasteAtActive,
    /// Copy the selection.
    Copy,
    /// Cut the selection.
    Cut,
    /// Paste the given text at the active cell.
    Paste(String),
    /// Cmd+Page Down: go to the next sheet.
    NextSheet,
    /// Cmd+Page Up: go to the previous sheet.
    PreviousSheet,
    /// A secondary click (or the Menu key, Shift+F10) on the cells or a header. The selection has already moved as the
    /// sheet moves it (a click outside the selection selects the cell or line first). `anchor` is the clicked cell's or
    /// header's rect (the active cell's for a key), where a menu opened from the keyboard goes; `pointer` is where a
    /// secondary click was, `None` for a key. Both are in screen coordinates.
    ContextMenu {
        /// The clicked cell's or header's rectangle, in screen points.
        anchor: Rect,
        /// Where a secondary click was, `None` for a key.
        pointer: Option<Pos2>,
        /// What was clicked.
        target: GridMenuTarget,
    },
}

/// Where a grid's context menu was asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum GridMenuTarget {
    /// The cells.
    Cells,
    /// A column header, by column index.
    ColumnHeader(u32),
    /// A row header, by row index.
    RowHeader(u32),
}

/// A column that shows a filter button at the end corner of its header cell.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ColumnFilter {
    /// The column.
    pub column: u32,
    /// Whether the column is filtered.
    pub filtered: bool,
    /// The column's sort, shown beside the funnel or chevron.
    pub sort: Option<craft_ui::contract::ColumnSort>,
}

impl ColumnFilter {
    /// A filter button on `column`, unfiltered and unsorted.
    pub const fn new(column: u32) -> Self {
        ColumnFilter { column, filtered: false, sort: None }
    }

    /// Whether the column is filtered.
    #[must_use]
    pub const fn filtered(mut self, on: bool) -> Self {
        self.filtered = on;
        self
    }

    /// The column's sort.
    #[must_use]
    pub const fn sort(mut self, sort: Option<craft_ui::contract::ColumnSort>) -> Self {
        self.sort = sort;
        self
    }
}

/// A problem in a cell, shown as an error tag beside it with its fixes in a menu.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct CellProblem<'a> {
    /// The cell.
    pub cell: CellAt<u32>,
    /// What is wrong.
    pub problem: Label,
    /// The ways to fix it.
    pub fixes: &'a [Label],
}

impl<'a> CellProblem<'a> {
    /// A problem in `cell` with its `fixes`.
    pub fn new(cell: CellAt<u32>, problem: impl Into<Label>, fixes: &'a [Label]) -> Self {
        CellProblem { cell, problem: problem.into(), fixes }
    }
}

/// The rows of a sheet as the collection's source: one key per row, equal to its position.
struct SheetRows(Positions);

impl DisplayOrder<u32> for SheetRows {
    fn len(&self) -> usize {
        self.0.len()
    }

    fn key(&self, at: usize) -> u32 {
        self.0.key(at)
    }

    fn index_of(&self, key: &u32) -> Option<usize> {
        self.0.index_of(key)
    }
}

impl ItemSource<u32> for SheetRows {}

/// What the grid returns.
#[non_exhaustive]
pub struct GridOutput {
    /// The collection's response and events.
    pub collection: CollectionResponse<u32, ()>,
    /// This pass's geometry, for the app's objects and overlays.
    pub geometry: SheetGeometry,
    /// What the grid asks of the app this pass, in order.
    pub edits: Vec<GridEdit>,
    /// The draft committed this pass, for the app to write into the cell.
    pub committed: Option<CellEdit>,
    /// The draft of the open cell, for the app's formula bar; `None` while no cell is open.
    pub draft: Option<String>,
    /// Whether Cmd/Ctrl was held at a fill commit this pass (copy and series swap).
    pub fill_swapped: bool,
    /// The filter button pressed this pass: its column and event.
    pub filter: Option<(u32, FilterButtonEvent)>,
    /// The fix chosen from the error tag's menu, by index.
    pub fix: Option<usize>,
    /// The outline buttons pressed this pass.
    pub outline: Vec<OutlineAction>,
}

impl Outcome for GridOutput {
    type Value = craft_ui::contract::NoValue;

    fn response(&self) -> &Response {
        &self.collection.response
    }

    #[cfg(feature = "testing")]
    fn story_facts(&self) -> craft_ui::testing::StoryOutput {
        self.collection.story_facts()
    }
}

/// What one pass of the grid collected for its output.
#[derive(Default)]
pub(crate) struct PassOutput {
    pub(crate) edits: Vec<GridEdit>,
    pub(crate) committed: Option<CellEdit>,
    pub(crate) draft: Option<String>,
    pub(crate) fill_swapped: bool,
    pub(crate) filter: Option<(u32, FilterButtonEvent)>,
    pub(crate) fix: Option<usize>,
    pub(crate) outline: Vec<OutlineAction>,
}

/// The style of the grid: the sheet look, the header and selection colours and the sizes the geometry reads.
#[derive(Clone, Debug, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct CellGridStyle {
    #[style(surface)]
    surface: SurfaceLook,
    #[style(metrics)]
    metrics: Metrics,
    pub(crate) sheet: SheetMetrics,
    pub(crate) bands: Vec2,
    pub(crate) background: StyleColor,
    pub(crate) header: StyleColor,
    pub(crate) header_text: StyleColor,
    pub(crate) header_selected: StyleColor,
    pub(crate) header_selected_text: StyleColor,
    pub(crate) header_whole: StyleColor,
    pub(crate) gridline: StyleColor,
    pub(crate) header_line: StyleColor,
    pub(crate) accent: StyleColor,
    pub(crate) preview: StyleColor,
    pub(crate) selection: StyleColor,
    pub(crate) handle_fill: StyleColor,
    pub(crate) handle_ring: StyleColor,
    pub(crate) edit_fill: StyleColor,
    pub(crate) edit_edge: StyleColor,
    pub(crate) handle: f32,
    pub(crate) mark: f32,
    pub(crate) pad: f32,
    pub(crate) border_reach: f32,
    pub(crate) side: f32,
    pub(crate) gap: f32,
}

/// The cell grid: a sheet of cells with headers, frozen panes, merged cells, a selection of ranges, an in-cell editor,
/// a fill handle and copy marks.
///
/// - Pointer: click selects, drag extends with auto-scroll, Shift+click extends from the anchor, Cmd/Ctrl+click adds a
///   range on an unselected cell and removes a selected one; a header click selects the line, a drag over headers
///   several; the corner selects all. A header edge drags a resize, a double-click autofits (or unhides a hidden
///   line); the fill handle on the current range's corner drags a [`GridEdit::Fill`], the range's border a
///   [`GridEdit::MoveCells`] (Esc cancels either).
/// - Keys: arrows move (Shift extends; Cmd/Ctrl jumps to [`GridModel::data_edge`]), Page Up and Down a screen, Home the
///   row start, Cmd/Ctrl+Home the first cell after the frozen panes, Cmd/Ctrl+End [`GridModel::last_used`], Enter and
///   Tab advance inside a selection (Shift back), Cmd/Ctrl+A selects all, Cmd/Ctrl+Space the column, Shift+Space the
///   row, F2 edits, Delete clears, Backspace clears and edits, typed text starts an edit with it. Esc and Enter with a
///   copy mark report [`GridEdit::ClearCopyMark`] and [`GridEdit::PasteAtActive`].
/// - The editor: typing opens the active cell in Enter mode (the arrows move to another cell and commit), F2 in Edit
///   mode (the arrows move the caret), `=` and an operator in point mode (the arrows move a reference cursor).
///
/// The scroll lives in the collection.
#[derive(craft_ui::contract::Component)]
#[component(collection)]
pub struct CellGrid<'a, M: GridModel> {
    #[common]
    common: Common,
    #[container]
    props: ContainerProps,
    items: CollectionProps,
    #[style_key]
    headings: bool,
    #[style_key]
    digits: u8,
    #[style_key]
    percent: u16,
    #[style_key]
    levels: (u8, u8),
    model: &'a M,
    selection: Bind<'a, CellSelection<u32>>,
    /// The selection as the app held it when the pass began, for the parts that declare before the item step runs.
    snapshot: CellSelection<u32>,
    zoom: f32,
    copy_mark: &'a [CellRange<u32>],
    filters: &'a [ColumnFilter],
    error_tag: Option<CellProblem<'a>>,
    outline: Option<(&'a OutlineAxis, &'a OutlineAxis)>,
    selectable: Box<dyn Fn(CellAt<u32>) -> bool + 'a>,
    frame: Option<paint::Frame>,
    pass: PassOutput,
}

impl<'a, M: GridModel> CellGrid<'a, M> {
    /// A grid named `label` over `model` that keeps its selection and zoom in `state`.
    pub fn new(label: impl Into<Label>, model: &'a M, state: &'a mut GridState) -> Self {
        let GridState { selection, zoom } = state;
        let zoom = if zoom.is_finite() { zoom.clamp(*SHEET_ZOOM.start(), *SHEET_ZOOM.end()) } else { 1.0 };
        let digits = digits_of(model.last_used().row.saturating_add(ROW_HEADER_LOOKAHEAD as u32)) as u8;
        CellGrid {
            common: Common::new(),
            props: ContainerProps::new().sizing(Sizing::Fill),
            items: CollectionProps::new().select(SelectMode::Cells),
            headings: true,
            digits,
            percent: (zoom * 100.0).round() as u16,
            levels: (0, 0),
            model,
            snapshot: selection.clone(),
            selection: Bind::bound(selection),
            zoom,
            copy_mark: &[],
            filters: &[],
            error_tag: None,
            outline: None,
            selectable: Box::new(move |cell| model.selectable(cell)),
            frame: None,
            pass: PassOutput::default(),
        }
        .label(label)
    }

    /// Copy mode: the copied ranges get marching ants. While any are shown, Esc reports [`GridEdit::ClearCopyMark`] and
    /// Enter [`GridEdit::PasteAtActive`]. The app owns the mark and passes it every pass.
    #[must_use]
    pub fn copy_mark(mut self, ranges: &'a [CellRange<u32>]) -> Self {
        self.copy_mark = ranges;
        self
    }

    /// The columns that show a filter button.
    #[must_use]
    pub fn filters(mut self, filters: &'a [ColumnFilter]) -> Self {
        self.filters = filters;
        self
    }

    /// The cell with a problem: an error tag sits beside it.
    #[must_use]
    pub fn error_tag(mut self, problem: Option<CellProblem<'a>>) -> Self {
        self.error_tag = problem;
        self
    }

    /// Reserves the outline bands for the groups of `rows` and `columns` inside the grid's rect and shows one band over
    /// them. Zero levels reserve nothing.
    #[must_use]
    pub fn outline(mut self, rows: &'a OutlineAxis, columns: &'a OutlineAxis) -> Self {
        self.levels = (rows.levels, columns.levels);
        self.outline = Some((rows, columns));
        self
    }

    /// Whether to show the row and column headings; `true` by default.
    #[must_use]
    pub fn headings(mut self, on: bool) -> Self {
        self.headings = on;
        self
    }

    /// Shows the grid and reports what the user did. The rows are the model's, one item each, drawn by [`GridRow`].
    pub fn show(self, ui: &mut egui::Ui) -> GridOutput {
        use craft_ui::contract::ShowCollection as _;
        let model: &'a M = self.model;
        let rows = SheetRows(Positions::new(model.rows().count()));
        let zoom = self.zoom;
        self.show_items(ui, &rows, &mut |key, icx| GridRow::new(model, *key, zoom, icx.cells().to_vec()))
    }
}

impl<M: GridModel> craft_ui::contract::Component for CellGrid<'_, M> {
    const ROLE: Role = Role::GRID;
    const INPUT: craft_ui::contract::Input = craft_ui::contract::Input::Editor;
    const KEYS: KeyUse = KeyUse::of(craft_ui::contract::Input::Editor).with_keep(KeepKeys::None);
    type Style = CellGridStyle;
    type Output = GridOutput;

    fn style(cx: &StyleCx<'_>, key: CellGridStyleKey) -> CellGridStyle {
        paint::style(cx, key)
    }

    fn parts(&mut self, rect: Rect, cx: &mut craft_ui::contract::PartsCx<'_, Self>) {
        self.declare_parts(rect, cx);
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        self.run_input(cx);
    }

    fn access(&self, access: &mut craft_ui::contract::Access, cx: &Cx<'_, Self>) {
        self.describe(access, cx);
    }

    fn paint_over(&mut self, ui: &mut PaintUi<'_>, cx: &Cx<'_, Self>) {
        self.paint_sheet(ui, cx);
    }

    fn finish(self, mut cx: Cx<'_, Self>) -> GridOutput {
        let mut collection = cx.collection_response();
        let geometry = self.frame.as_ref().map(|frame| frame.geometry.clone());
        let mut pass = self.pass;
        if collection.events.iter().any(|event| matches!(event, CollectionEvent::Activate(_)))
            && let Some(cell) = cx.collection_selection().and_then(|selection| selection.active().copied())
        {
            let state = cx.memory::<editor::EditState>();
            if state.mode == editor::Mode::Ready {
                state.open(cell, editor::Mode::Edit, &self.model.cell_text(cell));
                pass.draft = Some(state.draft.text().to_owned());
                pass.edits.push(GridEdit::BeginEdit { typed: None });
            }
        }
        collection.events.retain(|event| !matches!(event, CollectionEvent::Activate(_)));
        if let Some(geometry) = &geometry {
            for event in &collection.events {
                if let CollectionEvent::ContextMenu(ContextMenuRequest { at, .. }) = event {
                    let cell = geometry.cell_at(geometry.mirror_pos(*at));
                    pass.edits.push(GridEdit::ContextMenu {
                        anchor: geometry.mirror_rect(geometry.cell_rect(cell)),
                        pointer: Some(*at),
                        target: GridMenuTarget::Cells,
                    });
                }
            }
        }
        collection.events.retain(|event| !matches!(event, CollectionEvent::ContextMenu(_)));
        let geometry = geometry.unwrap_or_else(|| {
            let layout = GridLayout::new(cx.rect, self.model.rows().clone(), self.model.columns().clone());
            SheetGeometry::new(layout, cx.style.sheet)
        });
        GridOutput {
            collection,
            geometry,
            edits: pass.edits,
            committed: pass.committed,
            draft: pass.draft,
            fill_swapped: pass.fill_swapped,
            filter: pass.filter,
            fix: pass.fix,
            outline: pass.outline,
        }
    }
}

impl<M: GridModel> Collection for CellGrid<'_, M> {
    type Key = u32;
    type ItemLayout = SheetLayout;
    type Selection = CellSelection<u32>;
    type Item<'r> = GridRow<'r>;
    type ItemEvent = ();

    fn items(&self) -> &CollectionProps {
        &self.items
    }

    fn selection(&mut self) -> Option<BindMut<'_, CellSelection<u32>>> {
        Some(self.selection.reborrow())
    }

    fn cell_space<'s>(&'s self, space: CellSpace<'s, u32>) -> CellSpace<'s, u32> {
        space.merges(self.model.merges()).selectable(&*self.selectable)
    }

    fn item_layout(&self, _: &dyn ItemSource<u32>, _: f32, _: &StyleCx<'_>) -> SheetLayout {
        let (rows, columns) = self.model.frozen();
        SheetLayout::lines(self.model.rows().clone(), self.model.columns().clone()).frozen(columns as usize).frozen_rows(rows).zoom(self.zoom)
    }
}

craft_ui::words! {
    /// The grid's words, English by default.
    #[non_exhaustive]
    pub struct CellGridWords {
        select: "cellGrid.select" = "Select Cells";
        resize_column: "cellGrid.resizeColumn" = "Resize Column";
        resize_row: "cellGrid.resizeRow" = "Resize Row";
        fill: "cellGrid.fill" = "Fill";
        move_cells: "cellGrid.moveCells" = "Move Cells";
        column_header: template "cellGrid.columnHeader" = "Column {name}";
        row_header: template "cellGrid.rowHeader" = "Row {n}";
        corner: "cellGrid.corner" = "Select All";
    }
}

/// The insets a style holds for the headers: the row headers' width at the start and the column headers' height on
/// top, plus the bands.
pub(crate) fn header_insets(sheet: SheetMetrics, headings: bool, zoom: f32, digits: u32, bands: Vec2) -> Insets {
    let header = sheet.header(headings, zoom, digits);
    Insets::new(header.x + bands.x, 0.0, header.y + bands.y, 0.0)
}
