//! `StepList` and `StepRows`: record, go to, back, forward, delete, clear and restore, eviction, the pinned source.

use std::borrow::Cow;
use std::sync::atomic::{AtomicU64, Ordering};

use super::group::{Run, RunKind};
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
        Self {
            rows: vec![Row { id: StepId(0), label: first.label, icon: first.icon }],
            current: 0,
            limit: Self::DEFAULT_LIMIT,
            non_linear: false,
            next_id: 1,
            pinned: None,
            dropped: Vec::new(),
            cleared: None,
            run: None,
            marks: AtomicU64::new(0),
            closed: 0,
            revision: 0,
        }
    }

    /// The most past states held.
    pub fn limit(&self) -> usize {
        self.limit
    }

    /// Sets the limit, at least 1 (PhotoCraft's `max_states.max(1)`), and drops the oldest past
    /// states over it as [`record`](Self::record) does (sparing the pinned source).
    pub fn set_limit(&mut self, limit: usize) {
        self.limit = limit.max(1);
        if self.trim_to_limit() {
            self.revision += 1;
        }
    }

    /// Whether history is non-linear (History Options › Allow Non-Linear History).
    pub fn is_non_linear(&self) -> bool {
        self.non_linear
    }

    /// Turns non-linear history on or off; the states stay where they are.
    pub fn set_non_linear(&mut self, non_linear: bool) {
        if self.non_linear != non_linear {
            self.non_linear = non_linear;
            self.revision += 1;
        }
    }

    /// The number of states, at least 1.
    #[expect(clippy::len_without_is_empty, reason = "a step list always holds the oldest state")]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// The number of states before the current one.
    pub fn past_len(&self) -> usize {
        self.current
    }

    /// The number of states after the current one.
    pub fn later_len(&self) -> usize {
        self.rows.len() - self.current - 1
    }

    /// The row of the current state (0 is the oldest).
    pub fn current_index(&self) -> usize {
        self.current
    }

    /// The id of the current state.
    pub fn current_id(&self) -> StepId {
        self.rows[self.current].id
    }

    /// The id of row `at`, if the list has that row. PhotoCraft's engine commands keep `state`
    /// as a row index and map it with this and [`row_of`](Self::row_of).
    pub fn id_at(&self, at: usize) -> Option<StepId> {
        self.rows.get(at).map(|row| row.id)
    }

    /// The row of state `id`, if it is in the list.
    pub fn row_of(&self, id: StepId) -> Option<usize> {
        self.rows.iter().position(|row| row.id == id)
    }

    /// The label of state `id`.
    pub fn label_of(&self, id: StepId) -> Option<&str> {
        self.rows.iter().find(|row| row.id == id).map(|row| row.label.as_str())
    }

    /// Changes the label of state `id` (an app that translates labels at display time sets them
    /// again after a language change). Returns false when `id` is not in the list. Bumps the
    /// revision when the label changed.
    pub fn set_label(&mut self, id: StepId, label: impl Into<String>) -> bool {
        let Some(at) = self.row_of(id) else { return false };
        let label = label.into();
        if self.rows[at].label != label {
            self.rows[at].label = label;
            self.revision += 1;
        }
        true
    }

    /// The id of the oldest state (row 0).
    pub fn oldest_id(&self) -> StepId {
        self.rows[0].id
    }

    /// Whether a past state exists (Step Backward is enabled).
    pub fn can_back(&self) -> bool {
        self.current > 0
    }

    /// Whether a later state exists (Step Forward is enabled).
    pub fn can_forward(&self) -> bool {
        self.current + 1 < self.rows.len()
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
        let serial = self.marks.load(Ordering::Relaxed);
        let gesture_open = serial > self.closed;
        if gesture_open
            && let Some(run) = self.run
            && run.kind == (RunKind::Gesture { serial })
            && self.row_of(run.made).is_some()
        {
            self.revision += 1;
            self.run = Some(Run { revision: self.revision, ..run });
            return Recorded { id: run.made, extended: true };
        }
        self.forget_cleared();
        if !self.non_linear {
            while self.rows.len() > self.current + 1 {
                self.remove_row(self.current + 1);
            }
        }
        let id = StepId(self.next_id);
        self.next_id += 1;
        self.rows.push(Row { id, label: step.label, icon: step.icon });
        self.current = self.rows.len() - 1;
        self.trim_to_limit();
        self.revision += 1;
        self.run = gesture_open.then(|| Run::gesture(serial, id, self.revision));
        Recorded { id, extended: false }
    }

    /// Makes `id` current. No state is added or dropped; the skipped states stay redoable (one
    /// jump, never one undo per state: PhotoCraft #2966). Ends any run and forgets what `clear`
    /// set aside. `None` when `id` is unknown or already current.
    pub fn go_to(&mut self, id: StepId) -> Option<Jump> {
        let to = self.row_of(id)?;
        if to == self.current {
            return None;
        }
        let from = self.current_id();
        self.end_run();
        self.forget_cleared();
        self.current = to;
        self.revision += 1;
        Some(Jump { from, to: id })
    }

    /// [`go_to`](Self::go_to) the state before the current one (Step Backward); `None` at the
    /// oldest state.
    pub fn back(&mut self) -> Option<Jump> {
        let id = self.id_at(self.current.checked_sub(1)?)?;
        self.go_to(id)
    }

    /// [`go_to`](Self::go_to) the state after the current one (Step Forward); `None` at the
    /// newest state.
    pub fn forward(&mut self) -> Option<Jump> {
        let id = self.id_at(self.current + 1)?;
        self.go_to(id)
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
        let at = self.row_of(id).ok_or(DeleteRefused::Unknown)?;
        if at == 0 {
            return Err(DeleteRefused::Oldest);
        }
        if !self.non_linear && at > self.current {
            return Err(DeleteRefused::Later);
        }
        self.end_run();
        self.forget_cleared();
        if self.non_linear {
            self.remove_row(at);
        } else {
            while self.rows.len() > at {
                self.remove_row(at);
            }
        }
        self.revision += 1;
        Ok(Deleted { now: self.current_id() })
    }

    /// Clear History: keeps only the current state. The other states are set aside, not
    /// reported as dropped, until the next record, delete or go to;
    /// [`restore_cleared`](Self::restore_cleared) brings them back (Edit › Undo of Clear
    /// History). The pinned source is forgotten and comes back with the restore. Does nothing
    /// (and keeps the revision) when the list holds one state.
    pub fn clear(&mut self) {
        if self.rows.len() == 1 {
            return;
        }
        // A clear set aside earlier is final now.
        self.forget_cleared();
        self.end_run();
        let current = self.rows[self.current].clone();
        let rows = std::mem::replace(&mut self.rows, vec![current]);
        self.cleared = Some(Cleared { rows, current: self.current, pinned: self.pinned.take() });
        self.current = 0;
        self.revision += 1;
    }

    /// Whether [`restore_cleared`](Self::restore_cleared) has something to bring back.
    pub fn can_restore_cleared(&self) -> bool {
        self.cleared.is_some()
    }

    /// Brings back the states [`clear`](Self::clear) set aside, with the current state and the
    /// pinned source as they were. Returns false when nothing was set aside.
    pub fn restore_cleared(&mut self) -> bool {
        let Some(cleared) = self.cleared.take() else { return false };
        self.end_run();
        self.rows = cleared.rows;
        self.current = cleared.current;
        self.pinned = cleared.pinned;
        self.revision += 1;
        true
    }

    /// Drops the oldest past state that is not the pinned source and returns its id, for a byte
    /// budget the app keeps (PhotoCraft's `trim_with`, which calls this while over budget). The
    /// id also goes to [`take_dropped`](Self::take_dropped). `None` when no past state is left to
    /// drop; the current state is always kept.
    pub fn evict_oldest(&mut self) -> Option<StepId> {
        let at = self.oldest_unpinned_past()?;
        let id = self.rows[at].id;
        self.remove_row(at);
        self.revision += 1;
        Some(id)
    }

    /// Pins the history brush source to state `id`, or unpins with `None`. A pinned state is
    /// never evicted. An id not in the list unpins. Bumps the revision when the pin changed.
    pub fn pin_source(&mut self, id: Option<StepId>) {
        let id = id.filter(|id| self.row_of(*id).is_some());
        if self.pinned != id {
            self.pinned = id;
            self.revision += 1;
        }
    }

    /// The pinned state, if any.
    pub fn pinned(&self) -> Option<StepId> {
        self.pinned.filter(|id| self.row_of(*id).is_some())
    }

    /// The state the history brush reads: the pinned state, else the oldest state. `None` once a
    /// pinned state has left the list (deleted or dropped): there is no source until one is set.
    pub fn source(&self) -> Option<StepId> {
        match self.pinned {
            Some(id) => self.row_of(id).map(|_| id),
            None => Some(self.oldest_id()),
        }
    }

    /// [`pin_source`](Self::pin_source)`(Some(id))`.
    pub fn set_source(&mut self, id: StepId) {
        self.pin_source(Some(id));
    }

    /// Every id removed since the last call, once, in the order removed. Ids set aside by
    /// [`clear`](Self::clear) are not in it until the clear can no longer be undone.
    pub fn take_dropped(&mut self) -> Vec<StepId> {
        std::mem::take(&mut self.dropped)
    }

    /// The change counter: different after every change to the list, the same after reads.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Ends the open grouping run, gesture or keyed.
    pub(super) fn end_run(&mut self) {
        self.run = None;
        self.closed = self.marks.load(Ordering::Relaxed);
    }

    /// Reports the states a clear set aside as dropped: the clear can no longer be undone. The
    /// pin is not restored, so a pin that was set aside stays unpinned.
    fn forget_cleared(&mut self) {
        if let Some(cleared) = self.cleared.take() {
            let keep = self.current_id();
            self.dropped.extend(cleared.rows.iter().map(|row| row.id).filter(|id| *id != keep));
        }
    }

    /// Removes row `at`, reports its id and keeps `current` on the same state (or the row before,
    /// when the current state itself goes). A removed pinned state stays recorded as the pin, so
    /// [`source`](Self::source) is `None` until a new one is set.
    pub(super) fn remove_row(&mut self, at: usize) {
        let row = self.rows.remove(at);
        self.dropped.push(row.id);
        if at < self.current || (at == self.current && at > 0) {
            self.current -= 1;
        }
    }

    /// The oldest past row that is not the pinned source.
    fn oldest_unpinned_past(&self) -> Option<usize> {
        (0..self.current).find(|&at| Some(self.rows[at].id) != self.pinned)
    }

    /// Drops the oldest past states until at most `limit` are left. Returns whether any went.
    fn trim_to_limit(&mut self) -> bool {
        let mut dropped = false;
        while self.current > self.limit {
            let Some(at) = self.oldest_unpinned_past() else { break };
            self.remove_row(at);
            dropped = true;
        }
        dropped
    }
}

/// The rows of the list as the panel shows them: `depth` 0, `is_later` after the current row,
/// `can_delete` every row but the oldest, `is_non_linear` from the list.
impl StepRows for StepList {
    fn len(&self) -> usize {
        self.rows.len()
    }

    fn id(&self, at: usize) -> StepId {
        self.rows[at].id
    }

    fn label(&self, at: usize) -> Cow<'_, str> {
        Cow::Borrowed(&self.rows[at].label)
    }

    fn icon(&self, at: usize) -> Option<Icon> {
        self.rows[at].icon
    }

    fn current(&self) -> usize {
        self.current
    }

    fn revision(&self) -> u64 {
        self.revision
    }

    fn is_non_linear(&self) -> bool {
        self.non_linear
    }
}
