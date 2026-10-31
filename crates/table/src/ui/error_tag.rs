//! The error checking tag beside a cell (N6.09).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{IdSalt, Rect, Response, Ui};

use super::grid::CellRef;
use super::grid::GridGeometry;

/// Excel's error checking tag (GridCraft): an 18 × 18 tag left of `cell` (right of it when it
/// would cover the row headings), not drawn when outside the cells; an amber diamond with "!"
/// (the `warning` fill and ink), the problem as tooltip ("Number Stored as Text"); a click opens
/// a menu: the problem as its title, then the fixes. A Tab stop named "Error checking options";
/// Enter or Space opens the menu.
pub struct ErrorTag<'a> {
    id_salt: IdSalt,
    cell: CellRef,
    geometry: &'a GridGeometry,
    problem: &'a str,
    fixes: &'a [&'a str],
}

impl<'a> ErrorTag<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, cell: CellRef, geometry: &'a GridGeometry, problem: &'a str, fixes: &'a [&'a str]) -> Self {
        Self { id_salt: IdSalt::new(id_salt), cell, geometry, problem, fixes }
    }

    /// Where the tag goes, `None` when it is outside the cells.
    // gridcraft: crates/ui-egui/src/grid/menus.rs:424 (placement).
    pub fn rect(&self) -> Option<Rect> {
        todo!()
    }

    /// The fix picked this pass, by index.
    // gridcraft: crates/ui-egui/src/grid/menus.rs:416 `error_check_tag`.
    pub fn show(self, ui: &mut Ui) -> (Option<Response>, Option<usize>) {
        todo!()
    }
}
