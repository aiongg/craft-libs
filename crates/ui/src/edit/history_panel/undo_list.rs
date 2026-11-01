//! `UndoList`: the Undo drop-down list over `StepRows`.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, IdSalt, Response, Ui};

use super::labels::UndoListLabels;
use crate::edit::steps::StepRows;

/// What the undo list reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum UndoListEvent {
    /// Undo this many actions: the marked row and every newer one (WordCraft's
    /// `edit.undoTo {count}`). The list has closed its popup.
    UndoThrough(usize),
}

/// What [`UndoList::show`] returns.
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the app undoes what the list reports"]
pub struct UndoListOutput {
    /// The list.
    pub response: Response,
    /// The pass's event, if any.
    pub event: Option<UndoListEvent>,
}

/// WordCraft's Undo drop-down: the actions that can be undone, newest first, in a popup the app
/// opens from its own drop-down arrow with `Popup::menu`.
///
/// ```ignore
/// Popup::menu(&arrow).show(|ui| {
///     if let Some(UndoListEvent::UndoThrough(count)) = UndoList::new("undo-list", &rows).show(ui).event {
///         app.run("edit.undoTo", json!({ "count": count }));
///     }
/// });
/// ```
///
/// - **Rows.** One row per past state, from the current state back to row 1 (row 0, "Open", is
///   not an action), labels only (no source column, no icons), `list.row_height` high, the
///   popup `history.undo_list_width` wide ([`HistoryPanel::style`](super::HistoryPanel::style)).
///   At most 18 rows show; more scroll, keeping the marked row in view.
/// - **Marking.** Hover or Down and Up mark rows `0..=i` (row 0 is the newest), filled with
///   `list.hover`. A click or Enter reports `UndoThrough(i + 1)` and closes the popup
///   (`Ui::close`).
/// - **Footer.** "Undo {n} Actions" (a plural word: "Undo 1 Action") while rows are marked; while
///   none is, a Cancel row that closes the popup without an event. Esc closes without an event.
/// - **AccessKit.** A `ListBox` named [`UndoListLabels::list`] ("Undo History"); rows named by
///   their labels; the footer text is a label node whose text follows the count; the Cancel row
///   is a `Button`.
#[must_use = "show the list with `show`"]
pub struct UndoList<'a> {
    id_salt: IdSalt,
    steps: &'a dyn StepRows,
    labels: Option<UndoListLabels>,
}

impl<'a> UndoList<'a> {
    /// A list over `steps`.
    pub fn new(id_salt: impl AsIdSalt, steps: &'a dyn StepRows) -> Self {
        Self { id_salt: IdSalt::new(id_salt), steps, labels: None }
    }

    /// The list's words (default English).
    pub fn labels(mut self, labels: UndoListLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Shows the list in the popup's `Ui`.
    pub fn show(self, ui: &mut Ui) -> UndoListOutput {
        // wordcraft: crates/ui-egui/src/chrome.rs:102-200 (`undo_list`: width :153 (`HistoryPanelStyle::undo_list_width`), row height :154 (`list.row_height`), 18 rows :159, footer :127-135, Cancel row :185-197)
        todo!()
    }
}
