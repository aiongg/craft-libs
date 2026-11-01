//! `HistoryLabels`, `UndoListLabels`, `SnapshotFormLabels`, `HistoryOptionsLabels`: every user-visible word (`history.*`, `undoList.*`, `snapshots.*`).
//!
//! Menu rows and buttons are in Title Case, text only a screen reader reads in
//! sentence case.

use crate::localized::words::Counted;

crate::words! {
    /// The words of the [`HistoryPanel`](super::HistoryPanel): row descriptions, the source
    /// column, the footer buttons and the panel menu. `{row}` stands for a row's label.
    pub struct HistoryLabels {
        list: "history.list" = "History";
        current_state: "history.currentState" = "Current state";
        later_state: "history.laterState" = "Later state";
        snapshot: "history.snapshot" = "Snapshot";
        source: "history.source" = "History brush source: {row}";
        source_tooltip: "history.sourceTooltip" = "Sets the source for the history brush";
        name: "history.name" = "Name";
        footer: "history.footer" = "History actions";
        delete_button: "history.deleteButton" = "Delete current state";
        new_snapshot_button: "history.newSnapshotButton" = "Create new snapshot";
        new_document_button: "history.newDocumentButton" = "Create new document from current state";
        step_forward: "history.stepForward" = "Step Forward";
        step_backward: "history.stepBackward" = "Step Backward";
        new_snapshot: "history.newSnapshot" = "New Snapshot…";
        delete: "history.delete" = "Delete";
        delete_later: "history.deleteLater" = "Delete Redoable Actions";
        clear: "history.clear" = "Clear History";
        new_document: "history.newDocument" = "New Document";
        options: "history.options" = "History Options…";
        rename: "history.rename" = "Rename…";
    }
}

crate::words! {
    /// The words of the [`UndoList`](super::UndoList).
    pub struct UndoListLabels {
        list: "undoList.list" = "Undo History";
        cancel: "undoList.cancel" = "Cancel";
        undo_actions: "undoList.undoActions" = "Undo {n} Actions";
    }
}

/// The undo list's footer, "Undo 1 Action" or "Undo {n} Actions" (WordCraft `chrome.rs:130-132`).
pub(crate) const UNDO_ACTIONS: Counted = Counted::new("undoList.undoActions", "Undo {n} Action", "Undo {n} Actions");

crate::words! {
    /// The words of the New Snapshot form and the snapshot name suggestion. `{n}` stands for a
    /// number.
    pub struct SnapshotFormLabels {
        title: "snapshots.title" = "New Snapshot";
        name: "snapshots.name" = "Name";
        from: "snapshots.from" = "From";
        suggested: "snapshots.suggested" = "Snapshot {n}";
    }
}

crate::words! {
    /// The words of the History Options form.
    pub struct HistoryOptionsLabels {
        title: "historyOptions.title" = "History Options";
        non_linear: "historyOptions.nonLinear" = "Allow Non-Linear History";
    }
}
