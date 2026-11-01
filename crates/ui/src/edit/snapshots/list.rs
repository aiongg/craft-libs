//! `SnapshotId`, `Snapshot` and `SnapshotList`: ids never reused, "Snapshot N" suggestions, rename and remove.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use crate::edit::history_panel::SnapshotFormLabels;

/// One snapshot, unique within its list; ids are never reused. The app keeps the snapshot's
/// document and thumbnail keyed by it (a [`ThumbSource<SnapshotId>`](crate::canvas::textures::ThumbSource)).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SnapshotId(u64);

impl SnapshotId {
    /// The id `value`, for an app that keeps its own snapshot ids (PhotoCraft's engine commands
    /// take `snapshot` as a number).
    pub const fn new(value: u64) -> Self {
        SnapshotId(value)
    }

    /// The number inside.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// A named snapshot as the panel lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub id: SnapshotId,
    /// The name: trimmed, never blank.
    pub name: String,
}

/// A document's named snapshots, in creation order. A snapshot is not a history state: taking
/// one adds no state, and selecting one makes the document that snapshot as one new state named
/// after it (the app calls `StepList::record`). Snapshots live for the session (Photoshop and
/// PhotoCraft keep none in files).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SnapshotList {
    /// The snapshots, in creation order.
    pub(super) snapshots: Vec<Snapshot>,
    /// The number of the next id and of the next "Snapshot N" suggestion; never goes down.
    pub(super) next: u64,
    /// Ids removed since the last [`take_dropped`](SnapshotList::take_dropped).
    pub(super) dropped: Vec<SnapshotId>,
    /// The change counter.
    pub(super) revision: u64,
}

impl SnapshotList {
    /// An empty list; the first suggestion is "Snapshot 1".
    pub fn new() -> Self {
        // pc: crates/engine/src/cmd/history_cmds.rs:118-121 (`next_snapshot`, "Snapshot N")
        todo!()
    }

    /// The name the next snapshot takes when it gets none: `labels.suggested` ("Snapshot {n}")
    /// with the list's counter, which never goes down (removing "Snapshot 2" does not make the
    /// next one "Snapshot 2" again).
    pub fn suggested_name(&self, labels: &SnapshotFormLabels) -> String {
        // pc: crates/engine/src/cmd/history_cmds.rs:118-121
        todo!()
    }

    /// Adds a snapshot at the end and returns its id. `name` is trimmed; `None` or a blank name
    /// takes [`suggested_name`](Self::suggested_name). Bumps the counter and the revision.
    pub fn add(&mut self, name: Option<&str>, labels: &SnapshotFormLabels) -> SnapshotId {
        // pc: crates/engine/src/cmd/history_cmds.rs:136-170 (`new_snapshot`: name defaults to "Snapshot N")
        todo!()
    }

    /// Renames snapshot `id` to `name`, trimmed. Refuses a blank name and an unknown id (returns
    /// false, nothing changes). Two snapshots may share a name (Photoshop allows it).
    pub fn rename(&mut self, id: SnapshotId, name: &str) -> bool {
        // pc: crates/ui-egui/src/panels/history.rs:292-313 (rename in place; a blank name keeps the old one)
        todo!()
    }

    /// Removes snapshot `id` and reports it through [`take_dropped`](Self::take_dropped).
    /// Returns false for an unknown id.
    pub fn remove(&mut self, id: SnapshotId) -> bool {
        // pc: crates/engine/src/cmd/history_cmds.rs:208-215 (`history.deleteSnapshot`)
        todo!()
    }

    /// Snapshot `id`.
    pub fn get(&self, id: SnapshotId) -> Option<&Snapshot> {
        // new: a linear search.
        todo!()
    }

    /// Whether snapshot `id` is in the list.
    pub fn contains(&self, id: SnapshotId) -> bool {
        // new: `get(id).is_some()`.
        todo!()
    }

    /// The snapshots in creation order (the panel's order, top to bottom).
    pub fn iter(&self) -> std::slice::Iter<'_, Snapshot> {
        // new: `snapshots.iter()`.
        todo!()
    }

    /// The number of snapshots.
    pub fn len(&self) -> usize {
        // new: `snapshots.len()`.
        todo!()
    }

    /// Whether the list has no snapshot.
    pub fn is_empty(&self) -> bool {
        // new: `snapshots.is_empty()`.
        todo!()
    }

    /// Every id removed since the last call, once; the app drops the snapshot's document and
    /// thumbnail.
    pub fn take_dropped(&mut self) -> Vec<SnapshotId> {
        // new: `std::mem::take(&mut self.dropped)`.
        todo!()
    }

    /// The change counter: different after every add, rename and remove.
    pub fn revision(&self) -> u64 {
        // new: as `StepList::revision`.
        todo!()
    }
}
