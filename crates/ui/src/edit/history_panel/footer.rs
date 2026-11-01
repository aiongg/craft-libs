//! The History footer: Delete (with the trash drop), New Snapshot (Alt opens the dialog), New Document.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::Ui;

use super::labels::HistoryLabels;
use super::panel::{HistoryEvent, HistoryFeatures, HistoryKey};
use crate::edit::steps::StepId;

/// What the footer is drawn from; built by `HistoryPanel::show` each pass.
pub(super) struct FooterSpec<'a> {
    pub(super) features: &'a HistoryFeatures,
    pub(super) labels: &'a HistoryLabels,
    /// What a click on the trash deletes: the selected snapshot, else the current state unless it
    /// is the oldest; `None` disables the button (a drop can still be taken, see `accepts`).
    pub(super) trash: Option<HistoryKey>,
    /// Whether a dragged row may be dropped on the trash: not the oldest state; in linear history
    /// not a later state; every snapshot.
    pub(super) accepts: &'a dyn Fn(&HistoryKey) -> bool,
    /// The current state, for New Document.
    pub(super) current: StepId,
}

/// Draws the footer (`PanelFooter`) with the buttons whose feature is on, from the reading end:
/// Delete (`Icon::Trash`, named [`HistoryLabels::delete_button`]), New Snapshot (`Icon::Scan`,
/// [`HistoryLabels::new_snapshot_button`]), New Document (`Icon::NewFile`,
/// [`HistoryLabels::new_document_button`]); each is a Tab stop. With no button left the footer is
/// not drawn and nothing is reported.
///
/// - A click on the trash reports `DeleteSnapshot` or `DeleteStep` for `spec.trash`.
/// - A drop on the trash: right after the button,
///   `PanelFooter::drop_target::<ListDrag<HistoryKey>>(ui, &trash, |drag| drag.items.first().is_some_and(spec.accepts))`;
///   a payload reports `DeleteSnapshot` or `DeleteStep` for its first item (the panel's list is
///   single-selection, so a drag carries one row).
/// - The camera reports `NewSnapshot { dialog: false }`, or `dialog: true` with Alt held.
/// - New Document reports `NewDocument(spec.current)`.
pub(super) fn show_footer(ui: &mut Ui, spec: &FooterSpec<'_>) -> Vec<HistoryEvent> {
    // pc: crates/ui-egui/src/panels/history.rs:117-160 (trash, camera with Alt :149, file-plus; drop on the trash :138-146)
    todo!()
}
