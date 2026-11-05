//! `SnapshotId`, `Snapshot` and `SnapshotList`: ids never reused, "Snapshot N" suggestions, rename and remove.

use crate::edit::history_panel::SnapshotFormLabels;
use crate::localized::LocaleNumbers;

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
    /// Identifies the snapshot within its list.
    pub id: SnapshotId,
    /// The name: trimmed, never blank.
    pub name: String,
}

/// A document's named snapshots, in creation order. A snapshot is not a history state: taking
/// one adds no state, and selecting one makes the document that snapshot as one new state named
/// after it (the app calls `StepList::record`). Snapshots live for the session (Photoshop and
/// PhotoCraft keep none in files).
#[derive(Clone, Debug, PartialEq, Eq)]
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

impl Default for SnapshotList {
    fn default() -> Self {
        Self::new()
    }
}

impl SnapshotList {
    /// An empty list; the first suggestion is "Snapshot 1".
    pub fn new() -> Self {
        // pc: crates/engine/src/cmd/history_cmds.rs:118-121 (`next_snapshot`, "Snapshot N")
        SnapshotList { snapshots: Vec::new(), next: 1, dropped: Vec::new(), revision: 0 }
    }

    /// The name the next snapshot takes when it gets none: `labels.suggested` ("Snapshot {n}")
    /// with the list's counter, which never goes down (removing "Snapshot 2" does not make the
    /// next one "Snapshot 2" again).
    pub fn suggested_name(&self, labels: &SnapshotFormLabels, numbers: &LocaleNumbers) -> String {
        // pc: crates/engine/src/cmd/history_cmds.rs:118-121
        craft_i18n::fmt(&labels.suggested, &[("n", &numbers.count(self.next))])
    }

    /// Adds a snapshot at the end and returns its id. `name` is trimmed; `None` or a blank name
    /// takes [`suggested_name`](Self::suggested_name). Bumps the counter and the revision.
    pub fn add(&mut self, name: Option<&str>, labels: &SnapshotFormLabels, numbers: &LocaleNumbers) -> SnapshotId {
        // pc: crates/engine/src/cmd/history_cmds.rs:136-170 (`new_snapshot`: name defaults to "Snapshot N")
        let name = name.map(str::trim).filter(|name| !name.is_empty()).map_or_else(|| self.suggested_name(labels, numbers), str::to_owned);
        let number = self.next;
        self.next += 1;
        self.revision += 1;
        let id = SnapshotId(number);
        self.snapshots.push(Snapshot { id, name });
        id
    }

    /// Renames snapshot `id` to `name`, trimmed. Refuses a blank name and an unknown id (returns
    /// false, nothing changes). Two snapshots may share a name (Photoshop allows it).
    pub fn rename(&mut self, id: SnapshotId, name: &str) -> bool {
        // pc: crates/ui-egui/src/panels/history.rs:292-313 (rename in place; a blank name keeps the old one)
        let name = name.trim();
        if name.is_empty() {
            return false;
        }
        let Some(snapshot) = self.snapshots.iter_mut().find(|snapshot| snapshot.id == id) else { return false };
        name.clone_into(&mut snapshot.name);
        self.revision += 1;
        true
    }

    /// Removes snapshot `id` and reports it through [`take_dropped`](Self::take_dropped).
    /// Returns false for an unknown id.
    pub fn remove(&mut self, id: SnapshotId) -> bool {
        // pc: crates/engine/src/cmd/history_cmds.rs:208-215 (`history.deleteSnapshot`)
        let Some(at) = self.snapshots.iter().position(|snapshot| snapshot.id == id) else { return false };
        self.snapshots.remove(at);
        self.dropped.push(id);
        self.revision += 1;
        true
    }

    /// Snapshot `id`.
    pub fn get(&self, id: SnapshotId) -> Option<&Snapshot> {
        // new: a linear search.
        self.snapshots.iter().find(|snapshot| snapshot.id == id)
    }

    /// Whether snapshot `id` is in the list.
    pub fn contains(&self, id: SnapshotId) -> bool {
        // new: `get(id).is_some()`.
        self.get(id).is_some()
    }

    /// The snapshots in creation order (the panel's order, top to bottom).
    pub fn iter(&self) -> std::slice::Iter<'_, Snapshot> {
        // new: `snapshots.iter()`.
        self.snapshots.iter()
    }

    /// The number of snapshots.
    pub fn len(&self) -> usize {
        // new: `snapshots.len()`.
        self.snapshots.len()
    }

    /// Whether the list has no snapshot.
    pub fn is_empty(&self) -> bool {
        // new: `snapshots.is_empty()`.
        self.snapshots.is_empty()
    }

    /// Every id removed since the last call, once; the app drops the snapshot's document and
    /// thumbnail.
    pub fn take_dropped(&mut self) -> Vec<SnapshotId> {
        // new: `std::mem::take(&mut self.dropped)`.
        std::mem::take(&mut self.dropped)
    }

    /// The change counter: different after every add, rename and remove.
    pub fn revision(&self) -> u64 {
        // new: as `StepList::revision`.
        self.revision
    }
}
