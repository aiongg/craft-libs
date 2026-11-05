//! The spreadsheet cell grid: addresses in A1 notation, line sizes, geometry and the grid widget
//! (N6.01). The cell-selection model is craft-ui's ([`craft_ui::behavior::cells`]), shared with
//! craft-pages' document tables; it is re-exported here.

use std::collections::BTreeMap;

pub use craft_ui::behavior::cells::{CellAdvance, CellDirection, CellRange, CellRef, CellSelection, CellSpace};
use craft_ui::edit::EditEvent;
use craft_ui::theme::scale::Stroke as Line;
use craft_ui::theme::{StrokeExt as _, ToEgui as _, Tokens, TokensOf as _};
use egui::accesskit::Role;
use egui::{AsIdSalt, Id, IdSalt, Painter, Pos2, Rect, Response, Sense, Ui, Vec2, WidgetInfo};

use super::style::SheetStyle;

mod input;
mod paint;

/// Rows in a sheet, Excel's 1,048,576 (GridCraft `crates/core/src/addr.rs:10` `MAX_ROWS`).
pub const SHEET_ROWS: u32 = 1_048_576;

/// Columns in a sheet, Excel's 16,384 (A to XFD).
pub const SHEET_COLUMNS: u32 = 16_384;

/// The absolute reference `$B$3` of a cell (Excel's point mode inserts these).
pub fn absolute_reference(cell: CellRef) -> String {
    format!("${}${}", column_letters(cell.column), cell.row + 1)
}

/// `$A$1:$C$4`, or `$A$1` for one cell.
pub fn absolute_range_reference(range: CellRange) -> String {
    if range.is_single_cell() {
        absolute_reference(range.start)
    } else {
        format!("{}:{}", absolute_reference(range.start), absolute_reference(range.end))
    }
}

/// `A`, `Z`, `AA`, `XFD` for column 0, 25, 26, 16383.
// gridcraft: crates/core/src/addr.rs:94 `col_to_letters`.
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

/// Lines (rows or columns) with a default size and custom or hidden ones, and prefix sums, so
/// positions and the line at a position are a binary search. Sizes are in points at 100 % zoom.
#[derive(Clone, Debug, PartialEq)]
pub struct LineIndex {
    default: f64,
    count: u32,
    /// (line, its start, its size), sorted by line.
    lines: Vec<(u32, f64, f64)>,
}

impl LineIndex {
    /// `custom`: the lines whose size isn't the default (`0.0` for a hidden line); `count`
    /// lines in all (1,048,576 rows, 16,384 columns in Excel).
    // gridcraft: crates/model/src/sheet.rs:395 `LineIndex::new`.
    pub fn new(custom: &BTreeMap<u32, f32>, default: f32, count: u32) -> Self {
        let default = f64::from(default.max(0.0));
        let count = count.max(1);
        let mut lines = Vec::with_capacity(custom.len());
        let (mut position, mut next) = (0.0, 0u32);
        for (&line, &size) in custom {
            if line >= count {
                break;
            }
            position += f64::from(line - next) * default;
            let size = f64::from(size.max(0.0));
            lines.push((line, position, size));
            position += size;
            next = line + 1;
        }
        Self { default, count, lines }
    }

    /// The start of `line`.
    // gridcraft: crates/model/src/sheet.rs:410 `start`.
    pub fn start(&self, line: u32) -> f64 {
        match self.lines.binary_search_by(|entry| entry.0.cmp(&line)) {
            Ok(i) => self.lines.get(i).map_or(0.0, |entry| entry.1),
            Err(i) => match i.checked_sub(1).and_then(|k| self.lines.get(k)) {
                Some(&(before, start, size)) => start + size + f64::from(line - before - 1) * self.default,
                None => f64::from(line) * self.default,
            },
        }
    }

    /// The line holding `pos` (0 for a negative position, the last line past the end).
    // gridcraft: crates/model/src/sheet.rs:421 `at`.
    pub fn at(&self, pos: f64) -> u32 {
        if pos.is_nan() || pos <= 0.0 {
            return 0;
        }
        let last = self.count - 1;
        let after = self.lines.partition_point(|entry| entry.1 <= pos);
        let (first, first_start) = match after.checked_sub(1).and_then(|k| self.lines.get(k)) {
            Some(&(line, start, size)) if pos < start + size => return line.min(last),
            Some(&(line, start, size)) => (line + 1, start + size),
            None => (0, 0.0),
        };
        if self.default <= 0.0 {
            return first.min(last);
        }
        let line = f64::from(first) + ((pos - first_start) / self.default).floor();
        if line >= f64::from(self.count) { last } else { line as u32 }
    }

    /// The size of `line` in points at 100 % zoom; 0 for a hidden line, the default for a line with no custom size.
    pub fn size(&self, line: u32) -> f64 {
        match self.lines.binary_search_by(|entry| entry.0.cmp(&line)) {
            Ok(i) => self.lines.get(i).map_or(self.default, |entry| entry.2),
            Err(_) => self.default,
        }
    }

    /// The number of lines.
    pub fn count(&self) -> u32 {
        self.count
    }
}

/// The least zoom the column headers' height follows: below it they stay at this share of their
/// 100 % height, so the letters stay readable (GridCraft `geo.rs:35`). A proportion, not a size.
pub const HEADER_MIN_ZOOM: f32 = 0.75;

/// The least and greatest zoom (10 %…400 %).
const MIN_ZOOM: f32 = 0.1;
const MAX_ZOOM: f32 = 4.0;

/// How far below the window (points at 100 %) the row headers look for the last row whose digits
/// they must hold, so the header doesn't resize while scrolling (GridCraft `geo.rs:41`).
const ROW_HEADER_LOOKAHEAD: f64 = 200.0;

/// What a grid's geometry is made from, per frame.
#[derive(Clone, Debug)]
pub struct GridLayout {
    /// The grid's rectangle on screen, in points.
    pub rect: Rect,
    /// 0.1…4 (10 %…400 %).
    pub zoom: f32,
    /// Row heights.
    pub rows: LineIndex,
    /// Column widths.
    pub columns: LineIndex,
    /// Frozen rows and columns at the top and left.
    pub frozen: (u32, u32),
    /// Scroll of the unfrozen part, in points at 100 %.
    pub scroll: Vec2,
    /// Whether the row and column headings are shown.
    pub headings: bool,
    /// The outline bands' thickness left of the row headers and above the column headers
    /// ([`OutlineBand::thickness`](super::OutlineBand::thickness)).
    pub bands: Vec2,
    /// Whether the sheet is mirrored for a right-to-left script.
    pub right_to_left: bool,
}

/// The geometry of a grid for one frame: where each cell is on screen, and which cell is at a
/// point. Logical coordinates are left-to-right; a right-to-left sheet is mirrored across the
/// rect's centre line on screen ([`mirror_x`](Self::mirror_x)), so layout and hit tests stay
/// one code path.
#[derive(Clone, Debug)]
pub struct SheetGeometry {
    layout: GridLayout,
    cells: Rect,
    header: Vec2,
    frozen_size: Vec2,
}

impl SheetGeometry {
    /// Column headers `style.column_header_height` high at 100 % zoom (zoomed, but never below
    /// [`HEADER_MIN_ZOOM`] of it), row headers wide enough for the digits of the last visible row
    /// plus 200 (`(digits × style.row_header_digit + style.row_header_padding) × max(zoom,
    /// HEADER_MIN_ZOOM)`, at least two digits), none without headings; cells right of and below
    /// them and the bands.
    // gridcraft: crates/ui-egui/src/grid/geo.rs:35 `Geo::new`, `grid.rs:56` `COL_HEADER_H`.
    pub fn new(mut layout: GridLayout, style: &SheetStyle) -> Self {
        layout.zoom = if layout.zoom.is_finite() { layout.zoom.clamp(MIN_ZOOM, MAX_ZOOM) } else { 1.0 };
        layout.frozen = (layout.frozen.0.min(layout.rows.count()), layout.frozen.1.min(layout.columns.count()));
        let (zoom, rect) = (layout.zoom, layout.rect);
        let header_zoom = zoom.max(HEADER_MIN_ZOOM);
        let header_height = if layout.headings { (style.column_header_height * header_zoom).round() } else { 0.0 };
        let header_width = if layout.headings {
            let last_row = layout.rows.at(f64::from(layout.scroll.y + rect.height() / zoom) + ROW_HEADER_LOOKAHEAD) + 1;
            let digits = last_row.ilog10() + 1;
            ((digits.max(2) as f32 * style.row_header_digit + style.row_header_padding) * header_zoom).round()
        } else {
            0.0
        };
        let cells = Rect::from_min_max(Pos2::new(rect.left() + layout.bands.x + header_width, rect.top() + layout.bands.y + header_height), rect.max);
        let frozen_size = Vec2::new(layout.columns.start(layout.frozen.1) as f32 * zoom, layout.rows.start(layout.frozen.0) as f32 * zoom);
        Self { layout, cells, header: Vec2::new(header_width, header_height), frozen_size }
    }

    /// The zoom the geometry was built at, clamped to 0.1…4 (`1.0` for a non-finite one).
    pub fn zoom(&self) -> f32 {
        self.layout.zoom
    }

    /// The outline bands' thickness left of the row headers and above the column headers.
    pub fn bands(&self) -> Vec2 {
        self.layout.bands
    }

    /// The frozen rows and columns, at most the sheet's.
    pub fn frozen(&self) -> (u32, u32) {
        self.layout.frozen
    }

    /// The cells' area (right of the row headers, below the column headers).
    pub fn cells(&self) -> Rect {
        self.cells
    }

    /// The row headings' width by the column headings' height, in screen points; zero without headings.
    pub fn header_size(&self) -> Vec2 {
        self.header
    }

    /// The left x of `column` (frozen columns don't scroll).
    // gridcraft: crates/ui-egui/src/grid/geo.rs:141 `x`.
    pub fn x(&self, column: u32) -> f32 {
        let (columns, zoom) = (&self.layout.columns, self.layout.zoom);
        if column < self.layout.frozen.1 {
            self.cells.left() + columns.start(column) as f32 * zoom
        } else {
            let from_split = columns.start(column) - columns.start(self.layout.frozen.1) - f64::from(self.layout.scroll.x);
            self.cells.left() + self.frozen_size.x + from_split as f32 * zoom
        }
    }

    // gridcraft: crates/ui-egui/src/grid/geo.rs:151 `y`.
    /// The top y of `row` (frozen rows don't scroll).
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
    pub fn cell_rect(&self, c: CellRef) -> Rect {
        let (x, y) = (self.x(c.column), self.y(c.row));
        let size = Vec2::new(self.layout.columns.size(c.column) as f32, self.layout.rows.size(c.row) as f32) * self.layout.zoom;
        Rect::from_min_size(Pos2::new(x, y), size)
    }

    /// The rectangle covering `r`, in logical coordinates.
    pub fn range_rect(&self, r: CellRange) -> Rect {
        Rect::from_min_max(self.cell_rect(r.start).min, self.cell_rect(r.end).max)
    }

    /// The column at logical x.
    // gridcraft: crates/ui-egui/src/grid/geo.rs:172 `col_at`.
    pub fn column_at(&self, x: f32) -> u32 {
        let (columns, zoom, fc) = (&self.layout.columns, self.layout.zoom, self.layout.frozen.1);
        let from_left = f64::from((x - self.cells.left()) / zoom);
        if from_left < f64::from(self.frozen_size.x / zoom) {
            columns.at(from_left.max(0.0))
        } else {
            let past_split = from_left - f64::from(self.frozen_size.x / zoom);
            columns.at(columns.start(fc) + f64::from(self.layout.scroll.x) + past_split)
        }
    }

    // gridcraft: crates/ui-egui/src/grid/geo.rs:183 `row_at`.
    /// The row at logical y.
    pub fn row_at(&self, y: f32) -> u32 {
        let (rows, zoom, fr) = (&self.layout.rows, self.layout.zoom, self.layout.frozen.0);
        let from_top = f64::from((y - self.cells.top()) / zoom);
        if from_top < f64::from(self.frozen_size.y / zoom) {
            rows.at(from_top.max(0.0))
        } else {
            let past_split = from_top - f64::from(self.frozen_size.y / zoom);
            rows.at(rows.start(fr) + f64::from(self.layout.scroll.y) + past_split)
        }
    }

    /// The cell at the logical position `p`.
    pub fn cell_at(&self, p: Pos2) -> CellRef {
        CellRef::new(self.row_at(p.y), self.column_at(p.x))
    }

    /// The visible scrolled rows and columns (after the frozen ones), inclusive.
    // gridcraft: crates/ui-egui/src/grid/geo.rs:198 `scroll_rows`, `:204` `scroll_cols`.
    pub fn visible(&self) -> ((u32, u32), (u32, u32)) {
        let GridLayout { rows, columns, frozen, scroll, zoom, .. } = &self.layout;
        let span = |lines: &LineIndex, frozen: u32, scroll: f32, room: f32| {
            let top = lines.start(frozen) + f64::from(scroll);
            (lines.at(top).max(frozen), lines.at(top + f64::from(room / zoom)).min(lines.count() - 1))
        };
        (
            span(rows, frozen.0, scroll.y, self.cells.height() - self.frozen_size.y),
            span(columns, frozen.1, scroll.x, self.cells.width() - self.frozen_size.x),
        )
    }

    /// The panes (frozen corner, frozen rows, frozen columns, scrolling) that hold part of `r`, as
    /// screen rects to clip to, so a range scrolled under the frozen part isn't drawn over it.
    // gridcraft: crates/ui-egui/src/grid/geo.rs:213 `range_panes`.
    pub fn range_panes(&self, r: CellRange) -> Vec<Rect> {
        let (frozen_rows, frozen_columns) = self.layout.frozen;
        let split = Pos2::new(self.cells.left() + self.frozen_size.x, self.cells.top() + self.frozen_size.y);
        let mut xs = Vec::new();
        if r.start.column < frozen_columns {
            xs.push((self.cells.left(), split.x));
        }
        if r.end.column >= frozen_columns {
            xs.push((split.x, self.cells.right()));
        }
        let mut ys = Vec::new();
        if r.start.row < frozen_rows {
            ys.push((self.cells.top(), split.y));
        }
        if r.end.row >= frozen_rows {
            ys.push((split.y, self.cells.bottom()));
        }
        ys.iter()
            .flat_map(|&(top, bottom)| xs.iter().map(move |&(left, right)| Rect::from_min_max(Pos2::new(left, top), Pos2::new(right, bottom))))
            .collect()
    }

    /// The scroll that shows `range` whole (as little change as possible); a range in a frozen
    /// pane keeps the scroll of that axis.
    fn scroll_to_reveal(&self, range: CellRange) -> Vec2 {
        let GridLayout { rows, columns, frozen, scroll, zoom, .. } = &self.layout;
        let fit = |lines: &LineIndex, frozen: u32, (first, last): (u32, u32), scroll: f32, room: f32| {
            if first < frozen {
                return scroll;
            }
            let origin = lines.start(frozen);
            let (near, far) = (lines.start(first) - origin, lines.start(last) + lines.size(last) - origin);
            let (view, scroll) = (f64::from(room / zoom), f64::from(scroll));
            let next = if near < scroll {
                near
            } else if far > scroll + view {
                (far - view).min(near)
            } else {
                scroll
            };
            next.max(0.0) as f32
        };
        Vec2::new(
            fit(columns, frozen.1, (range.start.column, range.end.column), scroll.x, self.cells.width() - self.frozen_size.x),
            fit(rows, frozen.0, (range.start.row, range.end.row), scroll.y, self.cells.height() - self.frozen_size.y),
        )
    }

    /// Logical x to screen x and back (identity left-to-right).
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

/// The workbook sheet the grid shows; the app implements it on its sheet. Read only: the grid
/// reports what the user asks for as [`GridEdit`]s and moves the selection in [`GridState`].
pub trait GridModel {
    /// Row heights and column widths (points at 100 %) with hidden lines at 0.
    fn rows(&self) -> &LineIndex;
    /// Column widths in points at 100 %, with hidden columns at 0.
    fn columns(&self) -> &LineIndex;

    /// The number of frozen rows and columns; `(0, 0)` by default.
    fn frozen(&self) -> (u32, u32) {
        (0, 0)
    }

    /// The merged cells; clicks, arrows and ranges treat each as one cell.
    fn merges(&self) -> &[CellRange] {
        &[]
    }

    /// Whether a click or an arrow may land on `cell` (Excel's protected sheet with "Select
    /// unlocked cells" only).
    fn selectable(&self, cell: CellRef) -> bool {
        let _ = cell;
        true
    }

    /// Cmd/Ctrl+arrow (Excel's End mode): the edge of the data region from `from` toward
    /// `direction`, which only the app's cells know. The default goes to the sheet's edge.
    // gridcraft: crates/engine/src/selection.rs:132 `jump`.
    fn data_edge(&self, from: CellRef, direction: CellDirection) -> CellRef {
        let (rows, columns) = (self.rows().count().saturating_sub(1), self.columns().count().saturating_sub(1));
        match direction {
            CellDirection::Up => CellRef::new(0, from.column),
            CellDirection::Down => CellRef::new(rows, from.column),
            CellDirection::Left => CellRef::new(from.row, 0),
            CellDirection::Right => CellRef::new(from.row, columns),
        }
    }

    /// Cmd/Ctrl+End: the last cell with content (the bottom row and right column in use).
    fn last_used(&self) -> CellRef;

    /// Whether the grid draws the gridlines (Excel's View > Gridlines; GridCraft `show_gridlines`).
    fn show_gridlines(&self) -> bool {
        true
    }

    /// Paints cell `c` (value, format, borders, conditional format) into `rect` at sheet `zoom`
    /// (1.0 = 100 %), so the app scales its text and borders with the grid. The grid draws the
    /// gridlines (none inside a merged cell), headers and selection; the app everything in the
    /// cell.
    fn paint_cell(&self, painter: &Painter, rect: Rect, c: CellRef, zoom: f32);

    /// The column header's text: the letters, or a table's header name (Excel, GridCraft #1423).
    fn column_label(&self, column: u32) -> String {
        column_letters(column)
    }

    /// The cell's text for AccessKit and the copy fallback.
    fn cell_text(&self, c: CellRef) -> String;
}

/// What the grid keeps between passes; the app keeps one per sheet view and saves it with the
/// view (selection, scroll and zoom come back when the sheet reopens).
#[derive(Clone, Debug, PartialEq)]
pub struct GridState {
    /// The selected cells.
    pub selection: CellSelection,
    /// Scroll of the unfrozen part, in points at 100 %.
    pub scroll: Vec2,
    /// 0.1…4 (10 %…400 %); 1.0 for a new sheet.
    pub zoom: f32,
    /// The app's in-cell editor is open (the grid then leaves typing alone).
    pub editing: bool,
}

impl Default for GridState {
    /// A1 selected, unscrolled, at 100 %.
    fn default() -> Self {
        Self { selection: CellSelection::default(), scroll: Vec2::ZERO, zoom: 1.0, editing: false }
    }
}

/// What the grid asks of the app.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum GridEdit {
    /// Start editing the active cell: `typed` replaces its content (a key typed on the grid),
    /// `None` edits it in place (F2, a double-click).
    BeginEdit {
        /// Replacement text typed on the grid; `None` edits in place.
        typed: Option<String>,
    },
    /// Delete: clear the selection's contents.
    Clear,
    /// Backspace: clear the active cell and edit it.
    ClearAndEdit,
    /// A column header's right edge dragged: the new width in points at 100 % zoom, as one
    /// gesture (Esc during the drag cancels it). Several selected columns resize together: the
    /// app applies the width to every column of the selection that holds `column`, as Excel does.
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
    /// A double-click on a column header's edge: Excel's AutoFit Column Width.
    AutoFitColumn {
        /// The column whose edge was double-clicked.
        column: u32,
    },
    /// A double-click on a row header's edge: AutoFit Row Height.
    AutoFitRow {
        /// The row whose edge was double-clicked.
        row: u32,
    },
    /// A double-click on the edge where a hidden column sits: Excel's Unhide (GridCraft
    /// `grid/interact.rs:970`).
    UnhideColumn {
        /// The hidden column.
        column: u32,
    },
    /// A double-click on the edge where a hidden row sits.
    UnhideRow {
        /// The hidden row.
        row: u32,
    },
    /// The fill handle dragged: the range the current selection grows to (the source plus the
    /// filled cells), as one gesture. `Update`s show the preview, `Commit` carries the range and
    /// the modifiers held (Cmd/Ctrl swaps copying and extending a series, as in Excel), `Cancel`
    /// (Esc) drops it. A commit with `changed` false filled nothing. The grid selects the range
    /// when the gesture commits.
    Fill {
        /// The range the selection grows to.
        range: EditEvent<CellRange>,
    },
    /// The selection dragged by its border: the cell its top-left corner moves to, as one
    /// gesture like [`Fill`](Self::Fill). The app moves the cells (cut and paste); the grid
    /// selects the moved range when the gesture commits.
    MoveCells {
        /// The cell the selection's top-left corner moves to.
        to: EditEvent<CellRef>,
    },
    /// The fill handle dragged with the secondary button and released: the range the drag grew the
    /// selection to, and the pointer's screen position. The app opens its fill menu there (Copy
    /// Cells, Fill Series, Fill Formatting Only, Fill Without Formatting, and Fill Days, Weekdays,
    /// Months and Years from a date: GridCraft `grid/menus.rs:249-340`) and fills the range with
    /// the pick. Nothing is filled or selected until then; Esc during the drag reports nothing.
    FillMenu {
        /// The range the drag grew the selection to.
        range: CellRange,
        /// The pointer position in screen points.
        anchor: Pos2,
    },
    /// Esc while [`CellGrid::copy_mark`] shows the marching ants: the app ends copy mode.
    ClearCopyMark,
    /// Enter while [`CellGrid::copy_mark`] shows the marching ants: paste what was copied at the
    /// active cell and end copy mode (Excel).
    PasteAtActive,
    /// Copy the selection.
    Copy,
    /// Cut the selection.
    Cut,
    /// Paste the given text at the active cell.
    Paste(String),
    /// Cmd+Page Down / Up.
    NextSheet,
    /// Cmd+Page Up: go to the previous sheet.
    PreviousSheet,
    /// A secondary click (or the Menu key, Shift+F10) on the cells or a header. The selection has
    /// already moved as Excel moves it (a click outside the selection selects the cell or line
    /// first). `anchor` is the clicked cell's or header's rect (the active cell's for a key),
    /// where a menu opened from the keyboard goes; `pointer` is where a secondary click was, `None`
    /// for a key (the fields of `craft_ui::list::ContextMenuRequest`). Both are in screen
    /// coordinates, mirrored for a right-to-left sheet.
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

/// Excel's cell grid (look and keys from Excel, as GridCraft draws it): virtualised painting of
/// the visible cells only, frozen panes, headers, gridlines, the selection (ranges in the
/// selection tint with the active cell left clear, the current range's outline and its fill
/// handle), column and row resizing at header edges, right-to-left sheets. Sizes come from
/// [`SheetStyle`] (`sheet.*` tokens).
///
/// The grid allocates all the room it is given and draws no scroll bars: the app places them
/// beside it from [`GridState::scroll`] (GridCraft allocates its bar outside the grid's rect,
/// `grid/show.rs:25-29`).
///
/// The selection is [`GridState::selection`], moved by the rules of
/// [`craft_ui::behavior::cells`] over a [`CellSpace`] built from the model (its row and column
/// counts, [`GridModel::merges`], hidden lines from the [`LineIndex`]es,
/// [`GridModel::selectable`]); `selection_changed` on the response says it moved. Everything
/// else is a [`GridEdit`] for the app.
///
/// - Pointer: click selects ([`CellSelection::select`]), drag extends
///   ([`CellSelection::extend_to`]), Shift+click extends from the anchor, Cmd/Ctrl+click adds a
///   range ([`CellSelection::add`]); a header click selects the line, a drag over headers
///   several ([`CellSpace::whole_rows`], [`CellSpace::whole_columns`]); the corner selects all;
///   wheel scrolls (Shift horizontally, never past the sheet's last rows and columns),
///   Cmd/Ctrl+wheel zooms about the pointer. The fill handle on the current range's corner
///   drags a [`GridEdit::Fill`], the range's border a [`GridEdit::MoveCells`] (Esc cancels
///   either); a double-click on the edge where a hidden line sits reports an unhide.
/// - Keys while the grid has the focus and the app's editor is closed. The grid is one Tab stop
///   and keeps arrows, Tab, Enter and Esc (egui's focus lock filter, as GridCraft); Delete,
///   Backspace and typed text go through [`craft_ui::focus::key_owner`] so the app's shortcuts
///   leave them to the grid. Arrows move ([`CellSelection::step`]; mirrored right to left; Shift
///   extends with [`CellSelection::extend`]; Cmd/Ctrl jumps to [`GridModel::data_edge`]), Page
///   Up / Down move a screen of rows, Cmd/Ctrl+Page Down / Up the next / previous sheet, Home
///   the row start, Cmd/Ctrl+Home the first cell after the frozen rows and columns (A1 without
///   a freeze) and Cmd/Ctrl+End [`GridModel::last_used`], Enter and Tab
///   move the active cell ([`CellSelection::advance`]; Shift back), Cmd/Ctrl+A selects all, F2
///   edits, Delete clears, Backspace clears and edits, typed text starts an edit with it
///   (unless a shortcut typed it, GridCraft's Alt+= on Windows). Typing goes through
///   [`craft_ui::ime::Composer`] with its caret at the active cell, so an IME composition
///   starts there and reports [`GridEdit::BeginEdit`] with the editor taking the rest. Copy,
///   Cut and Paste events. While [`CellGrid::copy_mark`] shows ranges, Esc and Enter report
///   [`GridEdit::ClearCopyMark`] and [`GridEdit::PasteAtActive`]. F6 and Shift+F6 leave the grid for the
///   next and previous focus region (owner answer Q-S6b-11).
/// - AccessKit: `Grid` with `Row`s of `Cell`s (only the visible ones; `row_count` and
///   `column_count` give the sheet's size, each cell its `row_index` and `column_index`), the
///   active cell focused (`active_descendant`), named by its address and the model's
///   [`GridModel::cell_text`]; column and row headers as `ColumnHeader` and `RowHeader`.
pub struct CellGrid<'a, M: GridModel> {
    id_salt: IdSalt,
    model: &'a M,
    state: &'a mut GridState,
    headings: bool,
    right_to_left: bool,
    copy_mark: &'a [CellRange],
    outline: (u8, u8),
}

impl<'a, M: GridModel> CellGrid<'a, M> {
    /// A grid over `model` that keeps its selection, scroll and zoom in `state`.
    pub fn new(id_salt: impl AsIdSalt, model: &'a M, state: &'a mut GridState) -> Self {
        Self { id_salt: IdSalt::new(id_salt), model, state, headings: true, right_to_left: false, copy_mark: &[], outline: (0, 0) }
    }

    /// Copy mode: the copied ranges get marching ants. While any are shown, Esc reports
    /// [`GridEdit::ClearCopyMark`] and Enter [`GridEdit::PasteAtActive`] (Excel; GridCraft
    /// `grid/keyboard.rs:113`, `:137`). The app owns the mark and passes it every pass.
    pub fn copy_mark(mut self, ranges: &'a [CellRange]) -> Self {
        self.copy_mark = ranges;
        self
    }

    /// Reserves the outline bands for groups nested `rows` and `columns` levels deep, inside the
    /// grid's rect (GridCraft `grid/geo.rs:50-60`): a band left of the row headers and one above
    /// the column headers, and, without headings, a corner for the level buttons. The response's
    /// geometry carries them, and [`OutlineBand`](super::OutlineBand) draws in them. Zero levels
    /// reserve nothing.
    pub fn outline_levels(mut self, rows: u8, columns: u8) -> Self {
        self.outline = (rows, columns);
        self
    }

    /// Whether to show the row and column headings; `true` by default.
    pub fn headings(mut self, on: bool) -> Self {
        self.headings = on;
        self
    }

    /// Whether to mirror the sheet for a right-to-left script; `false` by default.
    pub fn right_to_left(mut self, on: bool) -> Self {
        self.right_to_left = on;
        self
    }

    // gridcraft: crates/ui-egui/src/grid/show.rs (painting), `grid/interact.rs` (pointer),
    // `grid/keyboard.rs:7` `keyboard`, `grid/selection.rs:93` `paint_selection`,
    // `grid/quadrant.rs:112` `paint_quadrant`.
    /// Shows the grid and reports what the user did.
    pub fn show(self, ui: &mut Ui) -> CellGridResponse {
        let Self { id_salt, model, state, headings, right_to_left, copy_mark, outline } = self;
        let tokens = Tokens::of(ui);
        let ppp = ui.ctx().pixels_per_point();
        let (rect, _) = ui.allocate_exact_size(ui.available_size_before_wrap(), Sense::hover());
        let id = ui.make_persistent_id(id_salt);
        state.zoom = if state.zoom.is_finite() { state.zoom.clamp(MIN_ZOOM, MAX_ZOOM) } else { 1.0 };
        state.scroll = state.scroll.max(Vec2::ZERO);

        let hidden_row = |row| model.rows().size(row) <= 0.0;
        let hidden_column = |column| model.columns().size(column) <= 0.0;
        let selectable = |cell| model.selectable(cell);
        let space = CellSpace::new(model.rows().count(), model.columns().count())
            .merges(model.merges())
            .hidden_rows(&hidden_row)
            .hidden_columns(&hidden_column)
            .selectable(&selectable);
        let pass = Pass {
            model,
            space,
            id,
            rect,
            style: SheetStyle::of(&tokens),
            headings,
            right_to_left,
            copy_mark,
            outline,
            hairline: Line::Hairline.width(ppp),
        };

        input::clamp_scroll(&pass, state);
        let response = ui.interact(rect, id, Sense::click_and_drag());
        response.widget_info(|| WidgetInfo::labeled(Role::Grid, ui.is_enabled(), ""));

        let before = state.selection.clone();
        let mut edits = Vec::new();
        input::wheel(&pass, ui, state);
        let preview = input::pointer(&pass, ui, &response, state, &mut edits);
        input::keys(&pass, ui, &response, state, &mut edits);
        input::clamp_scroll(&pass, state);
        let geometry = pass.geometry(state);
        paint::paint(&pass, ui, &tokens, state, &geometry, preview);
        if ui.ctx().has_accesskit_node(id) {
            paint::describe_cells(&pass, ui, id, &geometry);
        }
        paint::describe_grid(&pass, ui, id, state, &geometry);
        if response.has_focus() {
            let focus = Line::W2.egui(ui.ctx(), tokens.color.line.focus.to_egui());
            ui.painter().rect_stroke(rect, 0.0, focus, egui::StrokeKind::Inside);
        }
        CellGridResponse { selection_changed: state.selection != before, response, geometry, edits }
    }
}

/// What one pass of the grid reads: the model and the selection rules' space over it, the grid's
/// id and rect, and the sizes.
struct Pass<'a, M: GridModel> {
    model: &'a M,
    space: CellSpace<'a>,
    id: Id,
    rect: Rect,
    style: SheetStyle,
    headings: bool,
    right_to_left: bool,
    /// The ranges marked as copied.
    copy_mark: &'a [CellRange],
    /// The outline levels of the rows and of the columns, which reserve the bands.
    outline: (u8, u8),
    /// The width of a hairline in points.
    hairline: f32,
}

impl<M: GridModel> Pass<'_, M> {
    fn geometry(&self, state: &GridState) -> SheetGeometry {
        let layout = GridLayout {
            rect: self.rect,
            zoom: state.zoom,
            rows: self.model.rows().clone(),
            columns: self.model.columns().clone(),
            frozen: self.model.frozen(),
            scroll: state.scroll,
            headings: self.headings,
            bands: super::outline::OutlineBand::bands(self.outline, self.headings, state.zoom, &self.style),
            right_to_left: self.right_to_left,
        };
        SheetGeometry::new(layout, &self.style)
    }
}

/// What the grid did this pass.
pub struct CellGridResponse {
    /// The grid's response.
    pub response: Response,
    /// This pass's geometry (for the app's editor, objects and overlays).
    pub geometry: SheetGeometry,
    /// Whether the selection changed this pass.
    pub selection_changed: bool,
    /// What the grid asks of the app this pass, in order.
    pub edits: Vec<GridEdit>,
}
