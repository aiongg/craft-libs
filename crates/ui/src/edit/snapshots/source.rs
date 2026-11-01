//! `HistorySource` and `HistorySourceChoice`: the one history brush source of a document.

use super::list::{SnapshotId, SnapshotList};
use crate::edit::steps::{StepId, StepList};

/// What the history brush (and Fill › History, step 7) reads: a state or a snapshot. The two
/// are separate namespaces, and a document has exactly one source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HistorySource {
    /// A history state.
    Step(StepId),
    /// A named snapshot.
    Snapshot(SnapshotId),
}

/// The user's choice of history brush source, one per document: the default (the oldest state
/// held), a state, or a snapshot. A state choice pins the state against eviction
/// ([`StepList::pin_source`]); a choice whose state or snapshot is gone resolves to `None`
/// (no source until one is set).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct HistorySourceChoice {
    /// `None` is the default.
    pub(super) chosen: Option<HistorySource>,
}

impl HistorySourceChoice {
    /// Chooses `source` (the panel's `HistoryEvent::SetSource`). A state is also pinned in
    /// `steps`; a snapshot unpins any pinned state.
    pub fn set(&mut self, source: HistorySource, steps: &mut StepList) {
        // pc: crates/engine/src/cmd/history_source_cmds.rs:36-41 (`HistorySource::{State, Snapshot}`, one per document)
        self.chosen = Some(source);
        steps.pin_source(match source {
            HistorySource::Step(id) => Some(id),
            HistorySource::Snapshot(_) => None,
        });
    }

    /// Returns to the default and unpins (Edit › Purge › Histories).
    pub fn reset(&mut self, steps: &mut StepList) {
        // pc: crates/engine/src/cmd/history_source_cmds.rs:1-9 (default: the oldest state held)
        self.chosen = None;
        steps.pin_source(None);
    }

    /// The source to show and use: the chosen snapshot if `snapshots` holds it; the chosen state
    /// if `steps` holds it; [`StepList::source`] for the default (the pinned state, else the
    /// oldest); `None` when the chosen state or snapshot is gone.
    pub fn resolve(&self, steps: &StepList, snapshots: &SnapshotList) -> Option<HistorySource> {
        // pc: crates/ops/src/lib.rs:325-331 (`brush_source`: none after its state is gone)
        match self.chosen {
            None => steps.source().map(HistorySource::Step),
            Some(HistorySource::Snapshot(id)) => snapshots.contains(id).then_some(HistorySource::Snapshot(id)),
            Some(HistorySource::Step(id)) => steps.row_of(id).is_some().then_some(HistorySource::Step(id)),
        }
    }
}
