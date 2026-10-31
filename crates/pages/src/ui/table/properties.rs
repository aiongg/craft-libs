//! [`TableProperties`] (N3.12): InDesign's Table panel (and the Table section of Properties):
//! rows and columns, row height (At Least or Exactly) and column width, Distribute Rows Evenly,
//! cell text alignment and insets, merge and unmerge, insert and delete rows and columns, the
//! table's direction, alternating row fills, header and footer rows. Every change goes out as
//! one [`TableChange`]; the table stays the app's.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{Response, Ui};

craft_ui::words! {
    /// The panel's words (DesignCraft's strings).
    #[non_exhaustive]
    pub struct TablePropertiesLabels {
        rows: "table.rows" = "Rows";
        columns: "table.columns" = "Columns";
        row_height: "table.rowHeight" = "Row Height";
        at_least: "table.atLeast" = "At Least";
        exactly: "table.exactly" = "Exactly";
        column_width: "table.columnWidth" = "Column Width";
        distribute_rows: "table.distributeRows" = "Distribute Rows Evenly";
        distribute_rows_tip: "table.distributeRowsTip" = "Equal fixed heights for the selected rows; cell text may become overset.";
        cell_text: "table.cellText" = "Cell Text";
        align_top: "table.alignTop" = "Top";
        align_center: "table.alignCenter" = "Center";
        align_bottom: "table.alignBottom" = "Bottom";
        align_justify: "table.alignJustify" = "Justify";
        inset_top: "table.insetTop" = "Top";
        inset_left: "table.insetLeft" = "Left";
        inset_bottom: "table.insetBottom" = "Bottom";
        inset_right: "table.insetRight" = "Right";
        merge: "table.merge" = "Merge Cells";
        unmerge: "table.unmerge" = "Unmerge";
        insert_row: "table.insertRow" = "Insert Row";
        insert_column: "table.insertColumn" = "Insert Column";
        delete_row: "table.deleteRow" = "Delete Row";
        delete_column: "table.deleteColumn" = "Delete Column";
        direction: "table.direction" = "Direction";
        left_to_right: "table.leftToRight" = "Left to Right";
        right_to_left: "table.rightToLeft" = "Right to Left";
        alternating_fills: "table.alternatingFills" = "Alternating Row Fills";
        repeat_header: "table.repeatHeader" = "Repeat Header Rows";
        header_rows: "table.headerRows" = "Header Rows";
        footer_rows: "table.footerRows" = "Footer Rows";
    }
}

/// How a row's height is set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum RowHeight {
    #[default]
    AtLeast,
    Exactly,
}

/// Cell text's vertical justification.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CellAlign {
    #[default]
    Top,
    Center,
    Bottom,
    Justify,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextDirection {
    #[default]
    LeftToRight,
    RightToLeft,
}

/// The values at the cell selection; `None` where the selected cells differ (the field shows
/// mixed).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TableValues {
    pub rows: usize,
    pub columns: usize,
    pub row_height_mode: Option<RowHeight>,
    /// Points.
    pub row_height: Option<f64>,
    pub column_width: Option<f64>,
    pub cell_align: Option<CellAlign>,
    /// Top, left, bottom, right, points.
    pub insets: [Option<f64>; 4],
    pub direction: TextDirection,
    pub alternating_fills: bool,
    pub repeat_header: bool,
    pub header_rows: usize,
    pub footer_rows: usize,
    /// More than one cell is selected (Merge Cells is enabled).
    pub several_cells: bool,
    /// The selection holds a merged cell (Unmerge is enabled).
    pub merged: bool,
}

/// One change the user made.
#[derive(Clone, Debug, PartialEq)]
pub enum TableChange {
    Rows(usize),
    Columns(usize),
    RowHeightMode(RowHeight),
    RowHeight(f64),
    ColumnWidth(f64),
    DistributeRows,
    CellAlign(CellAlign),
    /// Inset `index` (top, left, bottom, right) in points.
    Inset(usize, f64),
    Merge,
    Unmerge,
    InsertRow,
    InsertColumn,
    DeleteRow,
    DeleteColumn,
    Direction(TextDirection),
    AlternatingFills(bool),
    RepeatHeader(bool),
    HeaderRows(usize),
    FooterRows(usize),
}

/// The panel body, built per pass. Number fields are craft-ui number fields in the document's
/// unit (a field scrubbed reports updates, the change goes out on release; `scrubbing` tells the
/// app to coalesce the history step).
pub struct TableProperties<'a> {
    values: &'a TableValues,
    unit: craft_ui::localized::units::LengthUnit,
    labels: Option<TablePropertiesLabels>,
}

impl<'a> TableProperties<'a> {
    pub fn new(values: &'a TableValues, unit: craft_ui::localized::units::LengthUnit) -> Self {
        TableProperties { values, unit, labels: None }
    }

    pub fn labels(mut self, labels: TablePropertiesLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The response, the change made this pass and whether a field is being scrubbed.
    // designcraft: crates/ui-egui/src/panels/table.rs:11 `show`.
    pub fn show(self, ui: &mut Ui) -> (Response, Option<TableChange>, bool) {
        todo!()
    }
}
