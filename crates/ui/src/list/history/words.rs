//! Built-in words of the History panel.

craft_ui::words! {
    /// The words of the History panel: row descriptions, the source column, the footer buttons and the panel menu.
    /// `{row}` stands for a row's label.
    pub struct HistoryWords {
        list: "history.list" = "History";
        source: "history.source" = "History brush source: {row}";
        source_tooltip: "history.sourceTooltip" = "Sets the source for the history brush";
        source_column: "history.sourceColumn" = "History brush source";
        name_column: "history.nameColumn" = "Name";
        footer: "history.footer" = "History actions";
        delete_button: "history.deleteButton" = "Delete current state";
        new_snapshot_button: "history.newSnapshotButton" = "Create new snapshot";
        new_document_button: "history.newDocumentButton" = "Create new document from current state";
        menu: "history.menu" = "History menu";
        step_forward: "history.stepForward" = "Step Forward";
        step_backward: "history.stepBackward" = "Step Backward";
        new_snapshot: "history.newSnapshot" = "New Snapshot\u{2026}";
        delete: "history.delete" = "Delete";
        delete_later: "history.deleteLater" = "Delete Redoable Actions";
        clear: "history.clear" = "Clear History";
        new_document: "history.newDocument" = "New Document";
        options: "history.options" = "History Options\u{2026}";
        rename: "history.rename" = "Rename\u{2026}";
    }
}
