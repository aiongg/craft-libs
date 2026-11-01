//! The field list pane of a pivot table (N6.04).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::behavior::Move;
use craft_ui::edit::EditEvent;
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
        compact: "fieldList.compact" = "Show in Compact Form";
        outline: "fieldList.outline" = "Show in Outline Form";
        tabular: "fieldList.tabular" = "Show in Tabular Form";
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
#[derive(Clone, Debug, PartialEq)]
pub enum FieldListEdit {
    /// Ticked: add it to its default area.
    Add {
        field: String,
    },
    /// Unticked or Remove Field.
    Remove {
        field: String,
    },
    /// A field's menu (Move to …, Move Up, Move Down), or a drag between or inside the areas,
    /// as one gesture of the kernel's [`Move`](craft_ui::behavior::Move) (the list's reorder):
    /// the app previews `Update`s and applies the `Commit`.
    Move {
        to: FieldArea,
        change: EditEvent<Move<String>>,
    },
    Refresh,
    CalculatedField,
    /// The Report Layout menu.
    Layout(ReportLayout),
}

/// The Report Layout menu's choices (Excel's).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReportLayout {
    Compact,
    Outline,
    Tabular,
}

/// The PivotTable Fields task pane (Excel's, as GridCraft lays it out): a field checklist, then
/// the four areas each listing its fields; Refresh, Calculated Field… and the Report Layout
/// [`PressMenu`](craft_ui::menu::PressMenu) at the bottom.
///
/// - The checklist is a kernel [`List`](craft_ui::list::List) whose rows carry a check-box
///   [`ToggleColumn`](craft_ui::list::ToggleColumn) (ticked: [`FieldListEdit::Add`], unticked:
///   [`FieldListEdit::Remove`]).
/// - Each area is a `List` with [`List::reorder`](craft_ui::list::List) and the
///   [`ListDrag`](craft_ui::behavior::ListDrag) payload, so fields drag inside an area and between
///   areas with the list's drop marker and ghost; each row's context menu (and the Menu key) has
///   Move to … each other area, Move Up, Move Down and Remove Field; Cmd/Ctrl+] and [ move the
///   selected field (decisions §4.2, lists). Moves arrive as [`FieldListEdit::Move`].
/// - Keyboard: each list is one Tab stop, then the buttons; AccessKit is the lists' (`ListBox`
///   named by the area's word) and the buttons'.
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
    pub fn show(self, ui: &mut Ui) -> FieldListPaneOutput {
        todo!()
    }
}

/// What the pane did this pass.
#[must_use]
pub struct FieldListPaneOutput {
    pub response: Response,
    /// This pass's edits, in order.
    pub edits: Vec<FieldListEdit>,
}
