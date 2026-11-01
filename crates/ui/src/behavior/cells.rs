//! The selected cells of a grid of cells: a spreadsheet's sheet (craft-table's `CellGrid`) and a
//! document table (craft-pages' tables). Pure data and rules: no egui, no drawing, no keys read.
//! Each widget maps its own input onto these rules and draws the selection its own way (Excel's
//! fill with the active cell left clear, InDesign's tint with an accent outline).
//!
//! - [`CellRef`]: a cell's address; [`CellRange`]: a rectangle of cells.
//! - [`CellSpace`]: the grid the rules run in: its size, its merged cells, its hidden rows and
//!   columns and the cells that can't be selected.
//! - [`CellSelection`]: the selected ranges, the active cell and the anchor, with Excel's rules
//!   for clicks, Shift and Cmd/Ctrl, the arrows, and Enter and Tab inside a selection.
//!
//! The rules are Excel's (GridCraft's `crates/engine/src/selection.rs` and
//! `cmd/edit/selection.rs:58`). A document table adds InDesign's and Word's Tab, which runs on
//! along the rows ([`CellSpace::next_in_reading_order`]).

use std::ops::RangeInclusive;

use serde::{Deserialize, Serialize};

/// A cell's address, zero-based: row 0, column 0 is a sheet's A1 and a table's first cell.
/// Ordered row first, then column (reading order left to right).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CellRef {
    pub row: u32,
    pub column: u32,
}

impl CellRef {
    pub fn new(row: u32, column: u32) -> Self {
        Self { row, column }
    }
}

/// A rectangle of cells, `start` the top-left and `end` the bottom-right corner, both inclusive.
/// Logical: in a right-to-left sheet the widget mirrors it on screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CellRange {
    pub start: CellRef,
    pub end: CellRef,
}

impl CellRange {
    /// The range spanned by two corners in any order.
    pub fn new(a: CellRef, b: CellRef) -> Self {
        Self { start: CellRef::new(a.row.min(b.row), a.column.min(b.column)), end: CellRef::new(a.row.max(b.row), a.column.max(b.column)) }
    }

    /// One cell.
    pub fn cell(c: CellRef) -> Self {
        Self { start: c, end: c }
    }

    pub fn contains(&self, c: CellRef) -> bool {
        self.rows().contains(&c.row) && self.columns().contains(&c.column)
    }

    pub fn is_single_cell(&self) -> bool {
        self.start == self.end
    }

    pub fn rows(&self) -> RangeInclusive<u32> {
        self.start.row..=self.end.row
    }

    pub fn columns(&self) -> RangeInclusive<u32> {
        self.start.column..=self.end.column
    }

    /// Whether the two ranges share a cell.
    pub fn intersects(&self, other: &CellRange) -> bool {
        self.start.row <= other.end.row
            && other.start.row <= self.end.row
            && self.start.column <= other.end.column
            && other.start.column <= self.end.column
    }

    /// The smallest range holding both.
    pub fn union(&self, other: &CellRange) -> CellRange {
        CellRange::new(
            CellRef::new(self.start.row.min(other.start.row), self.start.column.min(other.start.column)),
            CellRef::new(self.end.row.max(other.end.row), self.end.column.max(other.end.column)),
        )
    }

    /// The range grown until no merged cell (`merges`: each merge's range) crosses its edge, as
    /// Excel, InDesign and Word grow a selection over a merged cell. Grows again after each
    /// pass that took in a merge, since a merge taken in can reach another one.
    // gridcraft: crates/engine/src/selection.rs:68 `expand_merges`;
    // designcraft: crates/doc/src/table/merge.rs (the same rule for document tables).
    #[expect(clippy::todo, unused_variables, reason = "skeleton: removed when the bodies land")]
    pub fn covering_merges(self, merges: &[CellRange]) -> CellRange {
        todo!()
    }
}

/// A move of one cell. Logical: in a right-to-left sheet the widget maps the Left arrow to
/// [`Right`](Self::Right) (Excel mirrors the arrows with the sheet).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CellDirection {
    Up,
    Down,
    Left,
    Right,
}

/// Which key moves the active cell after an entry: Enter goes down the rows, Tab along a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CellAdvance {
    Enter,
    Tab,
}

/// The grid the selection rules run in, borrowed for one call: `rows` × `columns` cells (a sheet
/// has Excel's 1,048,576 × 16,384, a table its own), the merged cells, the hidden rows and
/// columns the arrows skip, and the cells that can't be selected (Excel's protected sheet with
/// "Select unlocked cells" only).
///
/// ```ignore
/// let hidden = |row| sheet.is_row_hidden(row);
/// let space = CellSpace::new(SHEET_ROWS, SHEET_COLUMNS).merges(&sheet.merges).hidden_rows(&hidden);
/// state.selection.step(CellDirection::Down, &space);
/// ```
#[derive(Clone, Copy)]
pub struct CellSpace<'a> {
    rows: u32,
    columns: u32,
    merges: &'a [CellRange],
    hidden_rows: Option<&'a dyn Fn(u32) -> bool>,
    hidden_columns: Option<&'a dyn Fn(u32) -> bool>,
    selectable: Option<&'a dyn Fn(CellRef) -> bool>,
}

impl<'a> CellSpace<'a> {
    /// A grid of `rows` × `columns` cells with no merges, nothing hidden and every cell
    /// selectable. Both are at least 1.
    pub fn new(rows: u32, columns: u32) -> Self {
        Self { rows: rows.max(1), columns: columns.max(1), merges: &[], hidden_rows: None, hidden_columns: None, selectable: None }
    }

    /// The merged cells, each as its range. They don't overlap.
    pub fn merges(mut self, merges: &'a [CellRange]) -> Self {
        self.merges = merges;
        self
    }

    pub fn hidden_rows(mut self, hidden: &'a dyn Fn(u32) -> bool) -> Self {
        self.hidden_rows = Some(hidden);
        self
    }

    pub fn hidden_columns(mut self, hidden: &'a dyn Fn(u32) -> bool) -> Self {
        self.hidden_columns = Some(hidden);
        self
    }

    /// Which cells a click or a move may land on; the arrows step over the others in their
    /// direction and stay put when there is none (GridCraft's `cell_selectable`).
    pub fn selectable(mut self, selectable: &'a dyn Fn(CellRef) -> bool) -> Self {
        self.selectable = Some(selectable);
        self
    }

    pub fn row_count(&self) -> u32 {
        self.rows
    }

    pub fn column_count(&self) -> u32 {
        self.columns
    }

    /// The merged cell holding `c`, if any.
    pub fn merge_at(&self, c: CellRef) -> Option<CellRange> {
        self.merges.iter().copied().find(|m| m.contains(c))
    }

    /// Every cell (Cmd/Ctrl+A on a sheet, Select › Table in a document).
    pub fn all(&self) -> CellRange {
        CellRange::new(CellRef::new(0, 0), CellRef::new(self.rows - 1, self.columns - 1))
    }

    /// Rows `first..=last` across every column (a row header click, or a drag over row headers).
    pub fn whole_rows(&self, first: u32, last: u32) -> CellRange {
        CellRange::new(CellRef::new(first, 0), CellRef::new(last, self.columns - 1))
    }

    /// Columns `first..=last` down every row (a column header click).
    pub fn whole_columns(&self, first: u32, last: u32) -> CellRange {
        CellRange::new(CellRef::new(0, first), CellRef::new(self.rows - 1, last))
    }

    /// `range` grown over the merged cells it crosses ([`CellRange::covering_merges`]); whole
    /// rows and whole columns stay as they are: a click on column B's header selects only B even
    /// when B1:C1 is merged, as in Excel.
    // gridcraft: crates/engine/src/selection.rs:66 (`is_full_cols`, `is_full_rows`).
    #[expect(clippy::todo, unused_variables, reason = "skeleton: removed when the bodies land")]
    pub fn cover(&self, range: CellRange) -> CellRange {
        todo!()
    }

    /// The cell one step from `from` toward `direction`: out of `from`'s merged cell from its far
    /// edge, over hidden rows and columns, onto the merged cell's top-left when it lands in one,
    /// and on over cells that aren't selectable. `None` at the grid's edge, or when no selectable
    /// cell lies that way.
    // gridcraft: crates/engine/src/cmd/edit/selection.rs:58 `selection_move` (hidden lines,
    // "Leaving a merged cell moves from its edge", the merge's start, `cell_selectable`).
    #[expect(clippy::todo, unused_variables, reason = "skeleton: removed when the bodies land")]
    pub fn neighbour(&self, from: CellRef, direction: CellDirection) -> Option<CellRef> {
        todo!()
    }

    /// A document table's Tab (InDesign, Word): the next cell along the row, then the first cell
    /// of the next row (`back`: the previous cell, then the last cell of the row above); a merged
    /// cell is visited once, at its top-left; hidden lines and unselectable cells are skipped.
    /// `None` past the last cell or before the first: InDesign and Word add a row there, which is
    /// the app's edit. A sheet's Tab is [`CellSelection::advance`], which stops at the sheet's
    /// edge.
    // designcraft: crates/engine/src/cmd/table/selection.rs:37 (Tab moves to the next cell).
    #[expect(clippy::todo, unused_variables, reason = "skeleton: removed when the bodies land")]
    pub fn next_in_reading_order(&self, from: CellRef, back: bool) -> Option<CellRef> {
        todo!()
    }
}

/// The selected cells: one or more ranges, the active cell (where typing goes, drawn clear of the
/// selection fill) and the anchor (the corner Shift extends from). The last range is the current
/// one; it holds the active cell. Never empty: a grid always has an active cell, A1 at first.
///
/// Excel's rules, which document tables share:
/// - A click selects the cell, or its whole merged cell with the active cell at the merge's
///   top-left ([`select`](Self::select)). A drag extends from the press ([`extend_to`](Self::extend_to)).
/// - Shift+click extends the current range from the anchor; the active cell stays
///   ([`extend_to`](Self::extend_to)). Shift+arrow moves the range's far corner, the
///   [`head`](Self::head) ([`extend`](Self::extend)).
/// - Cmd/Ctrl+click adds a range at the cell, which becomes active and the anchor
///   ([`add`](Self::add)); a drag from there extends the new range.
/// - An arrow collapses the selection to the active cell's neighbour ([`step`](Self::step)).
/// - Enter and Tab move the active cell inside a selection of more than one cell, wrapping, and
///   keep the selection; with one cell they move the selection, and Enter after a run of Tabs
///   returns to the column the run started in ([`advance`](Self::advance)).
/// - Header clicks and Select All replace the selection with whole lines or every cell
///   ([`select_range`](Self::select_range) with [`CellSpace::whole_rows`] and the like).
///
/// Every change goes through these methods, which keep the invariants (ranges cover the merged
/// cells they cross, the active cell is in the current range, a Tab run ends on any other move).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellSelection {
    ranges: Vec<CellRange>,
    active: CellRef,
    anchor: CellRef,
    /// The column the current run of Tabs started in, while the last move was a Tab.
    tab_run: Option<u32>,
}

impl Default for CellSelection {
    /// A1 (row 0, column 0), as a new sheet opens.
    fn default() -> Self {
        Self::cell(CellRef::default())
    }
}

impl CellSelection {
    /// One cell, active and the anchor. A cell inside a merge selects the merge through
    /// [`select`](Self::select) instead.
    pub fn cell(c: CellRef) -> Self {
        Self { ranges: vec![CellRange::cell(c)], active: c, anchor: c, tab_run: None }
    }

    /// One range with its top-left active and the anchor.
    pub fn range(r: CellRange) -> Self {
        Self { ranges: vec![r], active: r.start, anchor: r.start, tab_run: None }
    }

    /// Several ranges (an automation's `A1:B2,D4`) with `active` in the last one, the anchor at
    /// `active`. `None` when `ranges` is empty or the last range doesn't hold `active`.
    pub fn from_ranges(ranges: Vec<CellRange>, active: CellRef) -> Option<Self> {
        let holds = ranges.last().is_some_and(|r| r.contains(active));
        holds.then_some(Self { ranges, active, anchor: active, tab_run: None })
    }

    pub fn ranges(&self) -> &[CellRange] {
        &self.ranges
    }

    pub fn active(&self) -> CellRef {
        self.active
    }

    pub fn anchor(&self) -> CellRef {
        self.anchor
    }

    /// The range Shift extends (the last one).
    pub fn current(&self) -> CellRange {
        self.ranges.last().copied().unwrap_or(CellRange::cell(self.active))
    }

    /// The smallest range holding every selected range.
    pub fn bounds(&self) -> CellRange {
        self.ranges.iter().fold(CellRange::cell(self.active), |a, b| a.union(b))
    }

    /// The corner of the current range opposite the anchor: the end Shift+arrows move.
    // gridcraft: crates/engine/src/selection.rs:47 `moving_end`.
    pub fn head(&self) -> CellRef {
        let r = self.current();
        let row = if self.anchor.row == r.start.row { r.end.row } else { r.start.row };
        let column = if self.anchor.column == r.start.column { r.end.column } else { r.start.column };
        CellRef::new(row, column)
    }

    pub fn contains(&self, c: CellRef) -> bool {
        self.ranges.iter().any(|r| r.contains(c))
    }

    /// One cell, or one merged cell, selected: Enter and Tab then move the selection rather than
    /// the active cell inside it ([`advance`](Self::advance)).
    pub fn is_single_cell(&self, space: &CellSpace<'_>) -> bool {
        match self.ranges.as_slice() {
            [r] => r.is_single_cell() || space.merge_at(self.active) == Some(*r),
            _ => false,
        }
    }

    /// A click, or an arrow's landing cell: `c` alone (its merged cell whole, the active cell at
    /// the merge's top-left), active and the anchor. Ends a Tab run. Returns whether the
    /// selection changed.
    // gridcraft: crates/engine/src/cmd/edit/selection.rs:58 (`m.start` for a merged cell).
    #[expect(clippy::todo, unused_variables, reason = "skeleton: removed when the bodies land")]
    pub fn select(&mut self, c: CellRef, space: &CellSpace<'_>) -> bool {
        todo!()
    }

    /// A header click, a header drag or Select All: `range` alone, covered over merges unless it
    /// is whole lines ([`CellSpace::cover`]); the active cell and the anchor at its top-left.
    /// Ends a Tab run.
    #[expect(clippy::todo, unused_variables, reason = "skeleton: removed when the bodies land")]
    pub fn select_range(&mut self, range: CellRange, space: &CellSpace<'_>) -> bool {
        todo!()
    }

    /// Shift+click and a drag: the current range runs from the anchor to `c`, covered over
    /// merges; the active cell stays (Excel keeps it at the anchor). Ends a Tab run.
    // gridcraft: crates/engine/src/selection.rs:54 `extend_to`, `:68` `expand_merges`.
    #[expect(clippy::todo, unused_variables, reason = "skeleton: removed when the bodies land")]
    pub fn extend_to(&mut self, c: CellRef, space: &CellSpace<'_>) -> bool {
        todo!()
    }

    /// Shift+arrow: the [`head`](Self::head) moves one cell toward `direction` through
    /// [`CellSpace::neighbour`] (rows and columns hidden are skipped), and the current range runs
    /// from the anchor to it, covered over merges. Nothing changes at the grid's edge. Ends a
    /// Tab run.
    // gridcraft: crates/engine/src/cmd/edit/selection.rs:58 ("Extension moves the far corner").
    #[expect(clippy::todo, unused_variables, reason = "skeleton: removed when the bodies land")]
    pub fn extend(&mut self, direction: CellDirection, space: &CellSpace<'_>) -> bool {
        todo!()
    }

    /// Cmd+click (Ctrl+click): a new range at `c` (its merged cell whole) after the others; `c`
    /// becomes active and the anchor. A drag from there extends the new range. Excel never
    /// removes a range on Cmd+click, even over a selected cell. Ends a Tab run.
    #[expect(clippy::todo, unused_variables, reason = "skeleton: removed when the bodies land")]
    pub fn add(&mut self, c: CellRef, space: &CellSpace<'_>) -> bool {
        todo!()
    }

    /// An arrow: the selection collapses to the active cell's neighbour toward `direction`
    /// ([`CellSpace::neighbour`]), as [`select`](Self::select) does. At the grid's edge the
    /// selection collapses to the active cell. Ends a Tab run. A Cmd/Ctrl+arrow jump to the data
    /// edge is the widget's: it asks the app for the target and calls `select` or `extend_to`.
    #[expect(clippy::todo, unused_variables, reason = "skeleton: removed when the bodies land")]
    pub fn step(&mut self, direction: CellDirection, space: &CellSpace<'_>) -> bool {
        todo!()
    }

    /// Enter and Tab after an entry (`back` with Shift).
    ///
    /// - More than one cell selected: the active cell moves inside the current range, along the
    ///   row for Tab and down the column for Enter, wrapping to the next row or column; past the
    ///   range's last cell it goes on to the next range's first cell, after the last range back to
    ///   the first (Excel cycles every range; `back` runs the other way). Merged cells are
    ///   visited once, hidden lines skipped. The selection is kept.
    /// - One cell: the selection moves one cell, Tab right and Enter down ([`step`](Self::step),
    ///   stopping at the sheet's edge). A Tab starts or continues a Tab run; Enter after a run
    ///   goes down one row to the column the run started in (Excel's "return to the starting
    ///   column"), which ends the run.
    ///
    /// Returns whether the active cell moved.
    // gridcraft: crates/engine/src/selection.rs:90 `cycle` (inside the current range only;
    // Excel's rule goes on to the next range); crates/ui-egui/src/app/editing.rs:186
    // `move_after_enter` (the Tab run).
    #[expect(clippy::todo, unused_variables, reason = "skeleton: removed when the bodies land")]
    pub fn advance(&mut self, along: CellAdvance, back: bool, space: &CellSpace<'_>) -> bool {
        todo!()
    }
}
