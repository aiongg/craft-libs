//! The error checking tag beside a cell (N6.09).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{IdSalt, Rect, Response, Ui};

craft_ui::words! {
    /// The tag's accessible name, English by default (Excel's).
    #[non_exhaustive]
    pub struct ErrorTagLabels {
        options: "errorTag.options" = "Error checking options";
    }
}

use super::grid::CellRef;
use super::grid::GridGeometry;

/// Excel's error checking tag (GridCraft): a `sheet.error_tag` square ([`SheetStyle`](super::SheetStyle))
/// left of `cell` (right of it when it would cover the row headings), not drawn when outside the
/// cells; an amber diamond with "!" (the `status.warn` fill and ink), the problem as tooltip
/// (the app's words, "Number Stored as Text"); a click opens a [`PressMenu`](craft_ui::menu::PressMenu):
/// the problem as its title, then the fixes. A Tab stop (AccessKit `Button` with a popup) named
/// [`ErrorTagLabels::options`]; Enter, Space or Alt+Down opens the menu.
pub struct ErrorTag<'a> {
    id_salt: IdSalt,
    cell: CellRef,
    geometry: &'a GridGeometry,
    problem: &'a str,
    fixes: &'a [&'a str],
    labels: Option<ErrorTagLabels>,
}

impl<'a> ErrorTag<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, cell: CellRef, geometry: &'a GridGeometry, problem: &'a str, fixes: &'a [&'a str]) -> Self {
        Self { id_salt: IdSalt::new(id_salt), cell, geometry, problem, fixes, labels: None }
    }

    pub fn labels(mut self, labels: ErrorTagLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Where the tag goes for a tag `side` points square (`sheet.error_tag` at the sheet's
    /// zoom), `None` when it is outside the cells.
    // gridcraft: crates/ui-egui/src/grid/menus.rs:424 (placement).
    pub fn rect(&self, side: f32) -> Option<Rect> {
        todo!()
    }

    // gridcraft: crates/ui-egui/src/grid/menus.rs:416 `error_check_tag`.
    pub fn show(self, ui: &mut Ui) -> ErrorTagResponse {
        todo!()
    }
}

/// What the tag did this pass.
pub struct ErrorTagResponse {
    /// The tag's response; `None` when the tag is outside the cells and not drawn.
    pub response: Option<Response>,
    /// The fix picked this pass, by index into the fixes.
    pub fix: Option<usize>,
}
