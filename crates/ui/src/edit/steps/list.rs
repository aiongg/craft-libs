//! `StepList` and `StepRows`: record, go to, back, forward, delete, clear and restore, eviction, the pinned source.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::borrow::Cow;
use std::sync::atomic::AtomicU64;

use super::group::Run;
use super::{DeleteRefused, Deleted, HistoryStep, Jump, Recorded, StepId, StepRows};
use crate::icons::Icon;

/// The list of a document's history states, oldest first: ids, labels, icons, the current state,
/// later states, the state limit, linear or non-linear history, the history brush source and the
/// grouping runs. It holds no undo data: the app keeps its saved documents in a map keyed by
/// [`StepId`] and removes the ids [`take_dropped`](Self::take_dropped) reports.
///
/// - The list always holds at least one state (the oldest, "Open").
/// - States after the current one are later states (PhotoCraft's dimmed redo states).
/// - [`limit`](Self::limit) is the most past states held, not counting the current state or
///   later states.
/// - Every change bumps [`revision`](Self::revision): record, extend, go to, delete, clear,
///   restore, evict, `set_label`, `set_non_linear`, `set_limit` when it drops states,
///   `pin_source`. Reads never do.
/// - The grouping rules (one state per gesture, keyed runs) are in `group.rs`:
///   [`mark`](Self::mark), [`squash_since`](Self::squash_since), [`revert_to`](Self::revert_to),
///   [`record_keyed`](Self::record_keyed).
///
/// `StepList` is `Send + Sync`, so a document that holds one can live in an `Arc<Mutex<_>>`.
#[derive(Debug)]
pub struct StepList {
    /// The states, oldest first; never empty.
    pub(super) rows: Vec<Row>,
    /// The row of the current state.
    pub(super) current: usize,
    /// The most past states held.
    pub(super) limit: usize,
    /// Non-linear history: a record keeps the later states.
    pub(super) non_linear: bool,
    /// The number the next new id takes; never goes down.
    pub(super) next_id: u64,
    /// The state the history brush reads, pinned against eviction.
    pub(super) pinned: Option<StepId>,
    /// Ids removed since the last [`take_dropped`](Self::take_dropped), in the order removed.
    pub(super) dropped: Vec<StepId>,
    /// What [`clear`](Self::clear) set aside, until the next record, delete or go to.
    pub(super) cleared: Option<Cleared>,
    /// The open grouping run, if any (`group.rs`).
    pub(super) run: Option<Run>,
    /// The serial of the newest mark taken. [`mark`](Self::mark) takes `&self`, because
    /// `UndoMarks::undo_mark` does, so it bumps this atomic; the list stays `Send + Sync`.
    pub(super) marks: AtomicU64,
    /// The serial of the newest mark closed by `squash_since` or `revert_to` (or ended by a
    /// jump, delete or clear). A gesture run is open while `marks` is greater.
    pub(super) closed: u64,
    /// The change counter.
    pub(super) revision: u64,
}

/// One state of the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Row {
    pub(super) id: StepId,
    pub(super) label: String,
    pub(super) icon: Option<Icon>,
}

/// The states [`StepList::clear`] set aside: everything but the current state, with the current
/// state's row and the pinned source, so [`StepList::restore_cleared`] puts the list back as it
/// was.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Cleared {
    pub(super) rows: Vec<Row>,
    pub(super) current: usize,
    pub(super) pinned: Option<StepId>,
}

const _: fn() = || {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<StepList>();
};

impl StepList {
    /// The most past states a new list holds: PhotoCraft's Preferences › Performance › History
    /// States default (`crates/engine/src/prefs.rs:582`, `history_states` 50).
    pub const DEFAULT_LIMIT: usize = 50;

    /// A list holding one state, `first` ("Open"), current, with the limit
    /// [`DEFAULT_LIMIT`](Self::DEFAULT_LIMIT) and linear history. Its id is the first of the list's counter.
    pub fn new(first: HistoryStep) -> Self {
        // new: the list always holds the oldest state.
        todo!()
    }

    /// The most past states held.
    pub fn limit(&self) -> usize {
        // pc: crates/ops/src/lib.rs:51 (`max_states`)
        todo!()
    }

    /// Sets the limit, at least 1 (PhotoCraft's `max_states.max(1)`), and drops the oldest past
    /// states over it as [`record`](Self::record) does (sparing the pinned source).
    pub fn set_limit(&mut self, limit: usize) {
        // pc: crates/ops/src/lib.rs:114-119 (`new` clamps to 1), :172-176 (trim loop)
        todo!()
    }

    /// Whether history is non-linear (History Options › Allow Non-Linear History).
    pub fn is_non_linear(&self) -> bool {
        // pc: crates/engine/src/cmd/history_options_cmds.rs:29
        todo!()
    }

    /// Turns non-linear history on or off; the states stay where they are.
    pub fn set_non_linear(&mut self, non_linear: bool) {
        // pc: crates/engine/src/cmd/history_options_cmds.rs:29 (`allowNonLinearHistory`)
        todo!()
    }

    /// The number of states, at least 1.
    #[expect(clippy::len_without_is_empty, reason = "a step list always holds the oldest state")]
    pub fn len(&self) -> usize {
        // new: `rows.len()`.
        todo!()
    }

    /// The number of states before the current one.
    pub fn past_len(&self) -> usize {
        // new: `current`.
        todo!()
    }

    /// The number of states after the current one.
    pub fn later_len(&self) -> usize {
        // new: `rows.len() - current - 1`.
        todo!()
    }

    /// The row of the current state (0 is the oldest).
    pub fn current_index(&self) -> usize {
        // new: `current`.
        todo!()
    }

    /// The id of the current state.
    pub fn current_id(&self) -> StepId {
        // new: `rows[current].id`.
        todo!()
    }

    /// The id of row `at`, if the list has that row. PhotoCraft's engine commands keep `state`
    /// as a row index and map it with this and [`row_of`](Self::row_of).
    pub fn id_at(&self, at: usize) -> Option<StepId> {
        // pc: crates/engine/src/cmd/history_select_cmds.rs:1-9 (`state` is a row index)
        todo!()
    }

    /// The row of state `id`, if it is in the list.
    pub fn row_of(&self, id: StepId) -> Option<usize> {
        // new: a linear search (lists are at most a few hundred states).
        todo!()
    }

    /// The label of state `id`.
    pub fn label_of(&self, id: StepId) -> Option<&str> {
        // new: the row's label.
        todo!()
    }

    /// Changes the label of state `id` (an app that translates labels at display time sets them
    /// again after a language change). Returns false when `id` is not in the list. Bumps the
    /// revision when the label changed.
    pub fn set_label(&mut self, id: StepId, label: impl Into<String>) -> bool {
        // new: Lead call 13 (rows show the label text the app gives).
        todo!()
    }

    /// The id of the oldest state (row 0).
    pub fn oldest_id(&self) -> StepId {
        // new: `rows[0].id`.
        todo!()
    }

    /// Whether a past state exists (Step Backward is enabled).
    pub fn can_back(&self) -> bool {
        // pc: crates/engine/src/cmd/history_menu_cmds.rs:22-48 (`step`) (Step Backward errors without a past state)
        todo!()
    }

    /// Whether a later state exists (Step Forward is enabled).
    pub fn can_forward(&self) -> bool {
        // pc: crates/engine/src/cmd/history_menu_cmds.rs:22-48 (`step`)
        todo!()
    }

    /// Records one edit: a new state after the current one, made current. Ends a keyed run that
    /// this record does not extend, and forgets what [`clear`](Self::clear) set aside (those ids
    /// go to [`take_dropped`](Self::take_dropped) now).
    ///
    /// - Gesture runs: while a run is open (`marks` > `closed`, see [`mark`](Self::mark)) the
    ///   first record adds a state as below and keeps `Run::gesture(serial, id)` in `run`; every
    ///   later record of that run replaces nothing and returns that id with `extended: true`
    ///   (the state keeps the first record's label and icon; the revision still changes). If
    ///   the run's state has left the list, the record adds a state and the run continues on it.
    /// - Linear history (default): the later states are dropped and reported.
    /// - Non-linear history: the later states stay where they are and the new state goes to the
    ///   end.
    /// - Past the [`limit`](Self::limit), the oldest past state that is neither the current state
    ///   nor the pinned source is dropped, until the past count is at most the limit
    ///   (PhotoCraft: `while undo.len() > max_states`, the brush source spared and the next
    ///   oldest goes).
    pub fn record(&mut self, step: HistoryStep) -> Recorded {
        // pc: crates/ops/src/lib.rs:140-183 (`record`: linear drops redo, non-linear keeps it before the new step; limit loop :172-176)
        todo!()
    }

    /// Makes `id` current. No state is added or dropped; the skipped states stay redoable (one
    /// jump, never one undo per state: PhotoCraft #2966). Ends any run and forgets what `clear`
    /// set aside. `None` when `id` is unknown or already current.
    pub fn go_to(&mut self, id: StepId) -> Option<Jump> {
        // pc: crates/engine/src/cmd/history_select_cmds.rs:1-9
        todo!()
    }

    /// [`go_to`](Self::go_to) the state before the current one (Step Backward); `None` at the
    /// oldest state.
    pub fn back(&mut self) -> Option<Jump> {
        // pc: crates/engine/src/cmd/history_menu_cmds.rs:22-48 (`step`)
        todo!()
    }

    /// [`go_to`](Self::go_to) the state after the current one (Step Forward); `None` at the
    /// newest state.
    pub fn forward(&mut self) -> Option<Jump> {
        // pc: crates/engine/src/cmd/history_menu_cmds.rs:22-48 (`step`)
        todo!()
    }

    /// Deletes state `id`. Ends any run and forgets what `clear` set aside.
    ///
    /// - Linear history: drops the state and everything after it; the state before it becomes
    ///   current. Only a state at or before the current one can be deleted
    ///   ([`DeleteRefused::Later`] otherwise).
    /// - Non-linear history: drops only that state (a later one too); deleting the current state
    ///   makes the one before it current.
    /// - The oldest state is never deleted ([`DeleteRefused::Oldest`]); an unknown id is
    ///   [`DeleteRefused::Unknown`].
    ///
    /// A deleted pinned source is unpinned, so [`source`](Self::source) is `None` from then on.
    pub fn delete(&mut self, id: StepId) -> Result<Deleted, DeleteRefused> {
        // pc: crates/engine/src/cmd/history_cmds.rs:60-89 (linear), crates/engine/src/cmd/history_options_cmds.rs:61-110 (non-linear)
        todo!()
    }

    /// Clear History: keeps only the current state. The other states are set aside, not
    /// reported as dropped, until the next record, delete or go to;
    /// [`restore_cleared`](Self::restore_cleared) brings them back (Edit › Undo of Clear
    /// History). The pinned source is forgotten and comes back with the restore. Does nothing
    /// (and keeps the revision) when the list holds one state.
    pub fn clear(&mut self) {
        // pc: crates/ops/src/lib.rs:69-71 (`cleared`), :186-192, :229-230 (undo of the clear)
        todo!()
    }

    /// Whether [`restore_cleared`](Self::restore_cleared) has something to bring back.
    pub fn can_restore_cleared(&self) -> bool {
        // pc: crates/ops/src/lib.rs:186 (`can_undo` counts `cleared`)
        todo!()
    }

    /// Brings back the states [`clear`](Self::clear) set aside, with the current state and the
    /// pinned source as they were. Returns false when nothing was set aside.
    pub fn restore_cleared(&mut self) -> bool {
        // pc: crates/ops/src/lib.rs:229-230
        todo!()
    }

    /// Drops the oldest past state that is not the pinned source and returns its id, for a byte
    /// budget the app keeps (PhotoCraft's `trim_with`, which calls this while over budget). The
    /// id also goes to [`take_dropped`](Self::take_dropped). `None` when no past state is left to
    /// drop; the current state is always kept.
    pub fn evict_oldest(&mut self) -> Option<StepId> {
        // pc: crates/ops/src/lib.rs:580-600 (`trim`: oldest first, the newest state kept)
        todo!()
    }

    /// Pins the history brush source to state `id`, or unpins with `None`. A pinned state is
    /// never evicted. An id not in the list unpins. Bumps the revision when the pin changed.
    pub fn pin_source(&mut self, id: Option<StepId>) {
        // pc: crates/ops/src/lib.rs:76 (`brush_source: Option<u64>`), :313-331
        todo!()
    }

    /// The pinned state, if any.
    pub fn pinned(&self) -> Option<StepId> {
        // pc: crates/ops/src/lib.rs:76
        todo!()
    }

    /// The state the history brush reads: the pinned state, else the oldest state. `None` once a
    /// pinned state has left the list (deleted or dropped): there is no source until one is set.
    pub fn source(&self) -> Option<StepId> {
        // pc: crates/ops/src/lib.rs:325-331 (`brush_source`: row 0 for the default, none after its state is gone)
        todo!()
    }

    /// [`pin_source`](Self::pin_source)`(Some(id))`.
    pub fn set_source(&mut self, id: StepId) {
        // new: shorthand used by `HistorySourceChoice::set`.
        todo!()
    }

    /// Every id removed since the last call, once, in the order removed. Ids set aside by
    /// [`clear`](Self::clear) are not in it until the clear can no longer be undone.
    pub fn take_dropped(&mut self) -> Vec<StepId> {
        // new: `std::mem::take(&mut self.dropped)`.
        todo!()
    }

    /// The change counter: different after every change to the list, the same after reads.
    pub fn revision(&self) -> u64 {
        // new: PhotoCraft's `HistorySpot` (`panels/history.rs:218-231`) made exact.
        todo!()
    }
}

/// The rows of the list as the panel shows them: `depth` 0, `is_later` after the current row,
/// `can_delete` every row but the oldest, `is_non_linear` from the list.
impl StepRows for StepList {
    fn len(&self) -> usize {
        // new: as `StepList::len`.
        todo!()
    }

    fn id(&self, at: usize) -> StepId {
        // new: `rows[at].id`.
        todo!()
    }

    fn label(&self, at: usize) -> Cow<'_, str> {
        // new: `Cow::Borrowed(&rows[at].label)`.
        todo!()
    }

    fn icon(&self, at: usize) -> Option<Icon> {
        // new: `rows[at].icon`.
        todo!()
    }

    fn current(&self) -> usize {
        // new: `current`.
        todo!()
    }

    fn revision(&self) -> u64 {
        // new: `revision`.
        todo!()
    }

    fn is_non_linear(&self) -> bool {
        // new: `non_linear`.
        todo!()
    }
}
