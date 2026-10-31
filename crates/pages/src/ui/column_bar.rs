//! [`ColumnBar`] (N3.19): a bar split into column shares with draggable dividers, as PdfCraft's
//! Create Table of Fields sets its column widths: each column a share of the whole, a divider
//! between two columns moving only those two.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::edit::EditEvent;
use egui::{Response, Ui};

/// The smallest share a column keeps when a divider moves, or a quarter of what the two columns
/// have between them when that is less, so narrow columns can still be resized (PdfCraft's
/// `MIN_SHARE`).
pub const MIN_SHARE: f64 = 0.04;

/// The share an arrow key moves the focused divider by (PdfCraft: 0.01).
pub const KEY_STEP: f64 = 0.01;

/// `widths` (any unit) as shares of their sum; equal shares when the sum isn't positive and
/// finite.
// pdfcraft: crates/ui-egui/src/tools/field_table_ui.rs:100 `FieldTableDraft::shares`.
pub fn shares(widths: &[f64]) -> Vec<f64> {
    todo!()
}

/// Move the divider after column `i` by `delta` (a share of the whole): the two columns it
/// divides keep [`MIN_SHARE`] each (or a quarter of their pair), the others keep theirs. Never
/// panics on narrow pairs (`max` then `min`, never `clamp`).
// pdfcraft: crates/ui-egui/src/tools/field_table_ui.rs:109 `FieldTableDraft::move_divider`.
pub fn move_divider(widths: &mut [f64], i: usize, delta: f64) {
    todo!()
}

/// The bar, editing `widths` in place.
///
/// - The bar is outlined 1 pt in the border colour, corner radius 3; each divider is a 1.5 pt
///   border-coloured line, 3 pt accent while hovered, dragged or focused. Labels (the app's, "A",
///   "B", … by default) are centred in their share when they fit.
/// - A divider grabs within a few points but never past the middle of a column; a drag reports
///   one edit gesture (Begin, Update, Commit with the new widths; Esc cancels).
/// - Keys: Tab moves through the dividers (each an AccessKit Splitter "Column {n} width"); ← / →
///   move the focused one by [`KEY_STEP`] as a one-shot [`EditEvent::Set`].
pub struct ColumnBar<'a> {
    id_salt: egui::IdSalt,
    widths: &'a [f64],
    labels: Option<&'a [String]>,
}

impl<'a> ColumnBar<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, widths: &'a [f64]) -> Self {
        ColumnBar { id_salt: egui::IdSalt::new(id_salt), widths, labels: None }
    }

    /// Column names drawn in their shares.
    pub fn labels(mut self, labels: &'a [String]) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The bar with the new widths when a divider moved.
    // pdfcraft: crates/ui-egui/src/tools/field_table_ui.rs:338 `column_bar`, :376 `column_dividers`.
    pub fn show(self, ui: &mut Ui) -> (Response, Option<EditEvent<Vec<f64>>>) {
        todo!()
    }
}
