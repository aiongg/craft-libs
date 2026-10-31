//! The field list pane of a pivot table (N6.04).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, IdSalt, Response, Ui};

craft_ui::words! {
    /// The pane's words, English by default (GridCraft's, Excel's).
    #[non_exhaustive]
    pub struct FieldListLabels {
        title: "fieldList.title" = "PivotTable Fields";
        choose: "fieldList.choose" = "Choose fields to add to report:";
        areas_hint: "fieldList.areasHint" = "Use a field's menu to move it between areas:";
        filters: "fieldList.filters" = "Filters";
        columns: "fieldList.columns" = "Columns";
        rows: "fieldList.rows" = "Rows";
        values: "fieldList.values" = "Values";
        move_to: "fieldList.moveTo" = "Move to {area}";
        move_up: "fieldList.moveUp" = "Move Up";
        move_down: "fieldList.moveDown" = "Move Down";
        remove: "fieldList.remove" = "Remove Field";
        refresh: "fieldList.refresh" = "Refresh";
        calculated: "fieldList.calculated" = "Calculated Field…";
        layout: "fieldList.layout" = "Report Layout";
    }
}

/// A pivot area.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FieldArea {
    Filters,
    Columns,
    Rows,
    Values,
}

impl FieldArea {
    /// Excel's order in the pane.
    pub const ALL: [FieldArea; 4] = [FieldArea::Filters, FieldArea::Columns, FieldArea::Rows, FieldArea::Values];
}

/// A source field: its name, whether it holds numbers (ticked, it goes to Values; text to Rows).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PivotField {
    pub name: String,
    pub numeric: bool,
}

/// What the pane asks of the app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldListEdit {
    /// Ticked: add it to its default area.
    Add {
        field: String,
    },
    /// Unticked or Remove Field.
    Remove {
        field: String,
    },
    Move {
        field: String,
        to: FieldArea,
        index: usize,
    },
    Refresh,
    CalculatedField,
    /// The Report Layout menu (Compact, Outline, Tabular), by index.
    Layout(usize),
}

/// The PivotTable Fields task pane (GridCraft): a field checklist, then the four areas each
/// listing its fields with a menu (Move to … each other area, Move Up, Move Down, Remove Field);
/// fields drag between areas (a drop line shows where); Refresh, Calculated Field… and the Report
/// Layout menu at the bottom. Checkboxes, field menus and buttons are Tab stops.
pub struct FieldListPane<'a> {
    id_salt: IdSalt,
    fields: &'a [PivotField],
    areas: &'a [(FieldArea, Vec<String>)],
    labels: Option<FieldListLabels>,
}

impl<'a> FieldListPane<'a> {
    pub fn new(id_salt: impl AsIdSalt, fields: &'a [PivotField], areas: &'a [(FieldArea, Vec<String>)]) -> Self {
        Self { id_salt: IdSalt::new(id_salt), fields, areas, labels: None }
    }

    pub fn labels(mut self, labels: FieldListLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // gridcraft: crates/ui-egui/src/pivot_pane.rs:21 `show`.
    pub fn show(self, ui: &mut Ui) -> (Response, Vec<FieldListEdit>) {
        todo!()
    }
}
