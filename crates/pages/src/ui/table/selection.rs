//! The cell selection of a document table (N3.25): a rectangle of cells from an anchor to a head
//! cell, drawn over the table as a tint with an accent outline, as InDesign and Word show selected
//! cells; and the selection's rules (a merged cell counts as one, Esc toggles between the cells
//! and the first cell's text).
//!
//! craft-table's spreadsheet grid needs the same model. [`CellRange`] and [`CellSelection`] are
//! written here so this part compiles; they move down to `craft_ui::behavior::cell_selection`
//! once craft-table lands, and both crates use them from there (rule 3 of the crate layout).
//! needs: lead craft-ui::behavior::cell_selection (shared with craft-table, part "timeline,
//! review, tables").

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{Painter, Rect};

/// A rectangle of cells, rows `r0..=r1` and columns `c0..=c1` (r0 ≤ r1, c0 ≤ c1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CellRange {
    pub r0: usize,
    pub c0: usize,
    pub r1: usize,
    pub c1: usize,
}

impl CellRange {
    pub fn cell(r: usize, c: usize) -> Self {
        CellRange { r0: r, c0: c, r1: r, c1: c }
    }

    /// The range between two cells, either order.
    pub fn between(a: (usize, usize), b: (usize, usize)) -> Self {
        CellRange { r0: a.0.min(b.0), c0: a.1.min(b.1), r1: a.0.max(b.0), c1: a.1.max(b.1) }
    }

    pub fn contains(&self, r: usize, c: usize) -> bool {
        (self.r0..=self.r1).contains(&r) && (self.c0..=self.c1).contains(&c)
    }

    /// The range grown until no merged cell (`merged`: each merge's range) crosses its edge, as
    /// InDesign and Word extend a selection over a merged cell.
    // designcraft: crates/doc/src/table/grid.rs:7 `CellRange` (and the merge expansion in
    // crates/doc/src/table/merge.rs).
    pub fn covering_merges(self, merged: &[CellRange]) -> CellRange {
        todo!()
    }
}

/// The selected cells: where the selection started and the cell it extends to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CellSelection {
    pub anchor: (usize, usize),
    pub head: (usize, usize),
}

impl CellSelection {
    pub fn range(&self) -> CellRange {
        CellRange::between(self.anchor, self.head)
    }

    /// ⇧-arrows: the head moves a cell, kept inside `rows` × `columns`.
    pub fn extend(&mut self, dr: isize, dc: isize, rows: usize, columns: usize) {
        todo!()
    }

    /// Tab / ⇧-Tab: the next (previous) cell in reading order, wrapping to the next row; `None`
    /// past the last cell (the app adds a row there, as InDesign and Word do).
    // designcraft: crates/engine/src/cmd/table/selection.rs:37 (Tab moves to the next cell).
    pub fn next_cell(&self, back: bool, rows: usize, columns: usize) -> Option<(usize, usize)> {
        todo!()
    }
}

/// Where the table's rows and columns are on screen: the edges, top to bottom and left to right
/// (n + 1 values for n rows).
#[derive(Clone, Debug, PartialEq)]
pub struct TableGeometry {
    pub row_edges: Vec<f32>,
    pub column_edges: Vec<f32>,
}

impl TableGeometry {
    /// The cell under a screen point, if any.
    pub fn cell_at(&self, p: egui::Pos2) -> Option<(usize, usize)> {
        todo!()
    }

    /// The screen rectangle of a range.
    pub fn rect(&self, range: CellRange) -> Rect {
        todo!()
    }
}

/// The overlay over selected cells, painted after the table.
///
/// - Selected cells tint with the selection colour (DesignCraft's 80, 140, 255 at 110 alpha for
///   text highlights; a token in the fill), the range outlined 1 pt in the accent colour; a
///   caret or text selection inside one cell draws as text.
/// - Paint only: the app's table tool runs clicks and drags (press in a cell and drag selects a
///   range, ⇧-click extends) and calls [`CellSelection`]'s rules.
pub struct CellSelectionOverlay<'a> {
    geometry: &'a TableGeometry,
    selection: CellSelection,
}

impl<'a> CellSelectionOverlay<'a> {
    pub fn new(geometry: &'a TableGeometry, selection: CellSelection) -> Self {
        CellSelectionOverlay { geometry, selection }
    }

    // designcraft: crates/ui-egui/src/canvas/text_overlay.rs:126 `draw_cell_selection`.
    pub fn paint(self, painter: &Painter) {
        todo!()
    }
}
