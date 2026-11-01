//! `HistoryPanelState`: the snapshot selection the panel keeps between passes.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use crate::edit::snapshots::SnapshotId;

/// What the History panel keeps per document between passes: the selected snapshot and the
/// revision the panel last scrolled for. Not persisted (snapshots live for the session); one per
/// document, beside its `StepList`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HistoryPanelState {
    /// The selected snapshot and the `StepRows::revision` the app passed with it. The panel keeps
    /// the snapshot selected while the rows' revision equals it and the snapshot exists.
    pub(super) selected: Option<(SnapshotId, u64)>,
    /// The `StepRows::revision` of the last pass, for the follow-scroll rule (scroll only when it
    /// moved).
    pub(super) seen: Option<u64>,
}

impl HistoryPanelState {
    /// A state with nothing selected.
    pub fn new() -> Self {
        Self::default()
    }

    /// Selects snapshot `id` after the app restored it, with the rows' revision after its record
    /// (`steps.revision()`). Any later change to the rows deselects it: one row is highlighted,
    /// as in Photoshop.
    pub fn snapshot_selected(&mut self, id: SnapshotId, revision: u64) {
        // pc: crates/ui-egui/src/panels/history.rs:214-255 (`(id, spot)` kept per document)
        todo!()
    }

    /// The selected snapshot, as of the last pass (the panel drops a stale selection in `show`).
    pub fn selected_snapshot(&self) -> Option<SnapshotId> {
        // new: `selected.map(|(id, _)| id)`.
        todo!()
    }

    /// Deselects the snapshot (a click on the current state, the snapshot deleted).
    pub fn clear_snapshot(&mut self) {
        // pc: crates/ui-egui/src/panels/history.rs:187-190 (a click on the current row deselects the snapshot)
        todo!()
    }

    /// Whether a snapshot selected at `revision` is still selected with the rows at `now`.
    pub(super) fn keeps(&self, now: u64) -> bool {
        // pc: crates/ui-egui/src/panels/history.rs:214-255
        todo!()
    }
}
