//! The filter button in a table's header cell (N6.10).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{IdSalt, Rect, Response, Ui};

/// The button's square in a header cell `cell` at `zoom`: side `(cell height − 4)` clamped to
/// 10…18 and no wider than the cell's reserved filter space less the padding, `2 × zoom` from the
/// bottom-right corner.
// gridcraft: crates/ui-egui/src/grid/tables.rs:4 `filter_button_rect`.
pub fn filter_button_rect(cell: Rect, zoom: f32, space: f32) -> Rect {
    todo!()
}

/// Excel's filter arrow (GridCraft): a small drop-down button in the header cell, a funnel when
/// the column is filtered (and sorted arrows when sorted, from the app). A click is the app's
/// (it opens its filter menu under the button). A Tab stop named by the column and "Filter";
/// Alt+Down opens it from the cell, as Excel.
pub struct FilterButton {
    id_salt: IdSalt,
    rect: Rect,
    filtered: bool,
    sorted: Option<bool>,
}

impl FilterButton {
    pub fn new(id_salt: impl egui::AsIdSalt, rect: Rect) -> Self {
        Self { id_salt: IdSalt::new(id_salt), rect, filtered: false, sorted: None }
    }

    pub fn filtered(mut self, on: bool) -> Self {
        self.filtered = on;
        self
    }

    /// `Some(true)` ascending, `Some(false)` descending.
    pub fn sorted(mut self, ascending: Option<bool>) -> Self {
        self.sorted = ascending;
        self
    }

    // gridcraft: crates/ui-egui/src/grid/tables.rs (heading paint and the button), `grid.rs`
    // (the filter menu opening).
    pub fn show(self, ui: &mut Ui) -> Response {
        todo!()
    }
}
