//! [`ColumnBar`] (N3.19): a bar split into column shares with draggable dividers, as PdfCraft's
//! Create Table of Fields sets its column widths: each column a share of the whole, a divider
//! between two columns moving only those two.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::edit::EditEvent;
use egui::{Response, Ui};

use super::style::ColumnBarStyle;

craft_ui::words! {
    /// The bar's words.
    #[non_exhaustive]
    pub struct ColumnBarLabels {
        divider: "columnBar.divider" = "Column {n} width";
        column: "columnBar.column" = "Column {letter}";
    }
}

/// What the bar did this pass.
#[derive(Debug)]
pub struct ColumnBarResponse {
    pub response: Response,
    /// A divider gesture or key: the new widths.
    pub edit: Option<EditEvent<Vec<f64>>>,
}

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

/// The bar over the app's `widths`; it reports edits, it never writes them.
///
/// - Look ([`ColumnBarStyle`]): the bar outlined, rounded; each divider a line, the active
///   divider (hovered, dragged or focused) in the accent. Labels (the app's, "A", "B", … by
///   default, [`ColumnBarLabels::column`]) are centred in their share when they fit.
/// - A divider grabs within the style's `grab` but never past the middle of a column; a drag is
///   one edit gesture, [`EditEvent`] (Begin, Update per move, Commit with the new widths; Esc
///   and focus loss Cancel), at most one event a pass.
/// - Keys: each divider is a Tab stop (an AccessKit Splitter named by
///   [`ColumnBarLabels::divider`], "Column {n} width"); ← / → move the focused one by
///   [`KEY_STEP`] as one [`EditEvent::Set`], mirrored in a right-to-left interface.
pub struct ColumnBar<'a> {
    id_salt: egui::IdSalt,
    widths: &'a [f64],
    names: Option<&'a [String]>,
    labels: Option<ColumnBarLabels>,
}

impl<'a> ColumnBar<'a> {
    pub fn new(id_salt: impl egui::AsIdSalt, widths: &'a [f64]) -> Self {
        ColumnBar { id_salt: egui::IdSalt::new(id_salt), widths, names: None, labels: None }
    }

    /// The bar's sizes and colours, from the tokens.
    pub fn style(tokens: &craft_ui::theme::UiTokens) -> ColumnBarStyle {
        ColumnBarStyle::of(tokens)
    }

    /// Column names drawn in their shares (the app's field names).
    pub fn names(mut self, names: &'a [String]) -> Self {
        self.names = Some(names);
        self
    }

    pub fn labels(mut self, labels: ColumnBarLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // pdfcraft: crates/ui-egui/src/tools/field_table_ui.rs:338 `column_bar`, :376 `column_dividers`.
    pub fn show(self, ui: &mut Ui) -> ColumnBarResponse {
        todo!()
    }
}
