//! [`TableSizePicker`] (N3.24): Insert › Table's hover grid: hover a rows-by-columns square to
//! size the table, click to insert; the label above reads "4×3 Table" (columns × rows) or Insert
//! Table, and the page shows a live preview of the hovered table (Word).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::Ui;

/// The grid's size: 8 rows by 10 columns (Word and PowerPoint).
pub const ROWS: usize = 8;
pub const COLUMNS: usize = 10;

/// Each square, screen points, and the space between (WordCraft: 16 and 2).
pub const CELL: f32 = 16.0;
pub const CELL_GAP: f32 = 2.0;

craft_ui::words! {
    /// The picker's words.
    #[non_exhaustive]
    pub struct TableSizePickerLabels {
        insert_table: "tableSize.insert" = "Insert Table";
        size: "tableSize.size" = "{cols}×{rows} Table";
    }
}

/// What the picker did: the hovered `(rows, columns)` for the live preview, and the size clicked
/// (the menu closes then).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TableSizeResponse {
    pub hovered: Option<(usize, usize)>,
    pub picked: Option<(usize, usize)>,
}

/// The picker inside a menu, built per pass.
///
/// - Squares up to the hovered one fill with the checked fill and an accent outline; others the
///   input fill and strong border.
/// - Keys: the grid is one Tab stop; arrows move the hovered square, Enter inserts it, Esc closes
///   the menu. It is an AccessKit Grid whose value reads like the label.
pub struct TableSizePicker {
    id_salt: egui::IdSalt,
    labels: Option<TableSizePickerLabels>,
}

impl TableSizePicker {
    pub fn new(id_salt: impl egui::AsIdSalt) -> Self {
        TableSizePicker { id_salt: egui::IdSalt::new(id_salt), labels: None }
    }

    pub fn labels(mut self, labels: TableSizePickerLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // wordcraft: crates/ui-egui/src/dialogs/forms.rs:335 `table_grid_picker`; deckcraft:
    // crates/ui-egui/src/ribbon/insert.rs:136 `table_grid_picker`.
    pub fn show(self, ui: &mut Ui) -> TableSizeResponse {
        todo!()
    }
}
