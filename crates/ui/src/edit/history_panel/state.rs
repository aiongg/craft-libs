//! `HistoryPanelState`: the snapshot selection the panel keeps between passes.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::edit::snapshots::SnapshotId;

/// What the History panel keeps per document between passes: the selected snapshot, the snapshot a
/// context menu was opened on and an in-place rename the menu asked for. Not persisted (snapshots
/// live for the session); one per document, beside its `StepList`.
///
/// Each state has its own instance serial, which the panel's list id and follow-scroll rule include:
/// snapshot ids restart in every document, so two documents' panels never share a rename draft or a
/// scroll offset, and showing another document's state reveals its current row. A `clone` keeps the
/// serial, so clone a state only to replace it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryPanelState {
    /// The instance serial: unique to this state within the process.
    pub(super) serial: u64,
    /// The selected snapshot and the `StepRows::revision` the app passed with it. The panel keeps
    /// the snapshot selected while the rows' revision equals it and the snapshot exists.
    pub(super) selected: Option<(SnapshotId, u64)>,
    /// The snapshot row a context menu was opened on.
    pub(super) menu_row: Option<SnapshotId>,
    /// A snapshot whose in-place rename the context menu's Rename… asked for; the next `show`
    /// starts it.
    pub(super) rename: Option<SnapshotId>,
}

impl Default for HistoryPanelState {
    fn default() -> Self {
        Self::new()
    }
}

impl HistoryPanelState {
    /// A state with nothing selected and a new instance serial.
    pub fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self { serial: NEXT.fetch_add(1, Ordering::Relaxed), selected: None, menu_row: None, rename: None }
    }

    /// Selects snapshot `id` after the app restored it, with the rows' revision after its record
    /// (`steps.revision()`). Any later change to the rows deselects it: one row is highlighted,
    /// as in Photoshop.
    pub fn snapshot_selected(&mut self, id: SnapshotId, revision: u64) {
        // pc: crates/ui-egui/src/panels/history.rs:214-255 (`(id, spot)` kept per document)
        self.selected = Some((id, revision));
    }

    /// The selected snapshot, as of the last pass (the panel drops a stale selection in `show`).
    pub fn selected_snapshot(&self) -> Option<SnapshotId> {
        self.selected.map(|(id, _)| id)
    }

    /// Deselects the snapshot (a click on the current state, the snapshot deleted).
    pub fn clear_snapshot(&mut self) {
        // pc: crates/ui-egui/src/panels/history.rs:187-190 (a click on the current row deselects the snapshot)
        self.selected = None;
    }

    /// Whether a snapshot selected at `revision` is still selected with the rows at `now`.
    pub(super) fn keeps(&self, now: u64) -> bool {
        // pc: crates/ui-egui/src/panels/history.rs:214-255
        self.selected.is_some_and(|(_, revision)| revision == now)
    }
}
