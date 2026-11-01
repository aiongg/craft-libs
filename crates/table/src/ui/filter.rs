//! The filter button in a table's header cell (N6.10).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::list::SortOrder;
use egui::{IdSalt, Rect, Response, Ui};

use super::style::SheetStyle;

craft_ui::words! {
    /// The filter button's words, English by default (Excel's).
    #[non_exhaustive]
    pub struct FilterButtonLabels {
        filter: "filterButton.filter" = "{column} (Showing All)";
        filtered: "filterButton.filtered" = "{column}: Filter applied";
    }
}

/// The button's square in a header cell `cell` at `zoom`: side the cell's height less twice the
/// inset, clamped to `style.filter_button_min`…`style.filter_button_max` (`sheet.filter_*`)
/// and no wider than the cell's reserved filter space less the inset, `style.filter_button_inset
/// × zoom` from the bottom-right corner.
// gridcraft: crates/ui-egui/src/grid/tables.rs:4 `filter_button_rect`.
pub fn filter_button_rect(cell: Rect, zoom: f32, space: f32, style: &SheetStyle) -> Rect {
    todo!()
}

/// Excel's filter arrow (GridCraft): a small drop-down button in the header cell, a funnel when
/// the column is filtered (and a sort arrow when sorted, from the app). A click is the app's
/// (it opens its filter menu under the button). A Tab stop named by [`FilterButtonLabels`] with
/// the column's name; Alt+Down opens it from the cell, as Excel.
pub struct FilterButton {
    id_salt: IdSalt,
    rect: Rect,
    filtered: bool,
    sorted: Option<SortOrder>,
    column: String,
    labels: Option<FilterButtonLabels>,
}

impl FilterButton {
    /// `column`: the header's text, for the accessible name.
    pub fn new(id_salt: impl egui::AsIdSalt, rect: Rect, column: impl Into<String>) -> Self {
        Self { id_salt: IdSalt::new(id_salt), rect, filtered: false, sorted: None, column: column.into(), labels: None }
    }

    pub fn labels(mut self, labels: FilterButtonLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    pub fn filtered(mut self, on: bool) -> Self {
        self.filtered = on;
        self
    }

    /// The column's sort, if any.
    pub fn sorted(mut self, order: Option<SortOrder>) -> Self {
        self.sorted = order;
        self
    }

    // gridcraft: crates/ui-egui/src/grid/tables.rs (heading paint and the button), `grid.rs`
    // (the filter menu opening).
    pub fn show(self, ui: &mut Ui) -> Response {
        todo!()
    }
}
