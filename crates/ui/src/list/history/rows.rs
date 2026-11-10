//! What the History panel shows: the rows of the states, the named snapshots and the history brush source.

use craft_behavior::IconId;
use std::borrow::Cow;

/// One history state, unique within its rows. An app that keeps its states in its own engine mints the ids with
/// [`StepId::new`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StepId(u64);

impl StepId {
    /// The id `value`.
    pub const fn new(value: u64) -> Self {
        StepId(value)
    }

    /// The number inside.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// The read-only view of a history that the panel takes. Row 0 is the oldest state.
///
/// An app with its own engine implements it over its undo and redo lists. A non-linear history (a tree of branches)
/// gives the rows in tree order with their [`depth`](Self::depth), and marks the rows off the current line with
/// [`is_later`](Self::is_later).
#[expect(clippy::len_without_is_empty, reason = "rows always hold the oldest state")]
pub trait StepRows {
    /// The number of rows, at least 1.
    fn len(&self) -> usize;
    /// The id of row `at`.
    fn id(&self, at: usize) -> StepId;
    /// The label of row `at`.
    fn label(&self, at: usize) -> Cow<'_, str>;
    /// The icon of row `at`; `None` leaves the slot empty.
    fn icon(&self, at: usize) -> Option<IconId>;
    /// The row of the current state.
    fn current(&self) -> usize;
    /// A counter that changes on every change to the rows and never on a read: the panel follows the current state
    /// and keeps a selected snapshot by it.
    fn revision(&self) -> u64;
    /// The row's tree depth; the panel indents tree rows by it.
    fn depth(&self, _at: usize) -> usize {
        0
    }
    /// Whether row `at` is a later (undone) state, drawn dimmed.
    fn is_later(&self, at: usize) -> bool {
        at > self.current()
    }
    /// Whether row `at` may be deleted, by the trash or a drop on it.
    fn can_delete(&self, at: usize) -> bool {
        at != 0
    }
    /// Whether the history is non-linear: a later state may be dragged to the trash.
    fn is_non_linear(&self) -> bool {
        false
    }
}

/// One snapshot, unique within its list; ids are never reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SnapshotId(u64);

impl SnapshotId {
    /// The id `value`.
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
#[non_exhaustive]
pub struct Snapshot {
    /// Identifies the snapshot within its list.
    pub id: SnapshotId,
    /// The name: trimmed, never blank.
    pub name: String,
}

/// A document's named snapshots in creation order. A snapshot is not a history state: taking one adds no state.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct SnapshotList {
    snapshots: Vec<Snapshot>,
    next: u64,
}

impl Default for SnapshotList {
    fn default() -> Self {
        Self::new()
    }
}

impl SnapshotList {
    /// An empty list.
    pub fn new() -> Self {
        SnapshotList { snapshots: Vec::new(), next: 1 }
    }

    /// Adds a snapshot at the end and returns its id. `name` is trimmed; a blank name takes `suggested`.
    pub fn add(&mut self, name: &str, suggested: &str) -> SnapshotId {
        let name = Some(name.trim()).filter(|name| !name.is_empty()).unwrap_or(suggested);
        let id = SnapshotId(self.next);
        self.next += 1;
        self.snapshots.push(Snapshot { id, name: name.to_owned() });
        id
    }

    /// The number the next snapshot's suggested name takes; it never goes down.
    pub fn next_number(&self) -> u64 {
        self.next
    }

    /// Renames snapshot `id` to `name`, trimmed. A blank name and an unknown id change nothing.
    pub fn rename(&mut self, id: SnapshotId, name: &str) -> bool {
        let name = name.trim();
        let Some(snapshot) = self.snapshots.iter_mut().find(|snapshot| snapshot.id == id).filter(|_| !name.is_empty()) else { return false };
        name.clone_into(&mut snapshot.name);
        true
    }

    /// Removes snapshot `id`; false for an unknown id.
    pub fn remove(&mut self, id: SnapshotId) -> bool {
        let before = self.snapshots.len();
        self.snapshots.retain(|snapshot| snapshot.id != id);
        self.snapshots.len() != before
    }

    /// Whether snapshot `id` is in the list.
    pub fn contains(&self, id: SnapshotId) -> bool {
        self.snapshots.iter().any(|snapshot| snapshot.id == id)
    }

    /// The snapshots in creation order, the panel's order from the top.
    pub fn iter(&self) -> std::slice::Iter<'_, Snapshot> {
        self.snapshots.iter()
    }

    /// The number of snapshots.
    pub fn len(&self) -> usize {
        self.snapshots.len()
    }

    /// Whether the list has no snapshot.
    pub fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }
}

/// What the history brush reads: a state or a snapshot. A document has one source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HistorySource {
    /// A history state.
    Step(StepId),
    /// A named snapshot.
    Snapshot(SnapshotId),
}
