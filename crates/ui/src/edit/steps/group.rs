//! Grouping: `StepList::{mark, revert_to, squash_since, record_keyed}`, `StepMark` and `QuietMarks`.
//!
//! A gesture is at most one state, enforced while it runs rather than squashed afterwards:
//! PhotoCraft's limit is 50 states, so one state per `Update` of a long drag would evict real
//! history before the `Commit` could fold it (Lead call 5).

use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::atomic::Ordering;

use super::list::StepList;
use super::{HistoryStep, Recorded, StepId};
use crate::edit::UndoMarks;

/// A point in a [`StepList`], taken by [`StepList::mark`]: the state that was current and the
/// mark's serial. A document's `UndoMarks::Mark`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StepMark {
    /// The state current when the mark was taken.
    pub(super) at: StepId,
    /// The mark's serial (`StepList::marks` after the bump), 1 for the first mark of a list.
    pub(super) serial: u64,
}

/// The open grouping run of a [`StepList`] (its `run` field). `list.rs`'s `record` reads and
/// writes it; every other change to the list (`go_to`, `delete`, `clear`, a `record` that does
/// not extend it) ends it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Run {
    pub(super) kind: RunKind,
    /// The state the run made: the one later records extend.
    pub(super) made: StepId,
    /// The list's revision right after the run's last record: a keyed run extends only while the
    /// revision is unchanged ("nothing else changed the list since").
    pub(super) revision: u64,
}

/// What opened a [`Run`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RunKind {
    /// A gesture's mark, by serial.
    Gesture { serial: u64 },
    /// [`StepList::record_keyed`] with this key's hash.
    Keyed { key: u64 },
}

impl Run {
    /// The run a gesture's first record opens.
    pub(super) fn gesture(serial: u64, made: StepId, revision: u64) -> Run {
        Run { kind: RunKind::Gesture { serial }, made, revision }
    }
}

impl StepList {
    /// Whether `mark` is the newest one taken and not yet closed.
    fn is_newest_open(&self, mark: StepMark) -> bool {
        mark.serial == self.marks.load(Ordering::Relaxed) && mark.serial > self.closed
    }

    /// Notes the current state and opens a gesture run: until the mark is closed, the first
    /// [`record`](Self::record) adds a state and every later one extends it (same id, the first
    /// record's label). Takes `&self` because `UndoMarks::undo_mark` does: it bumps the `marks`
    /// atomic, which `record` reads. A new mark replaces an open one (the `GestureHost` ends a
    /// gesture before the next begins).
    pub fn mark(&self) -> StepMark {
        // filmcraft: crates/engine/src/lib.rs:969-990 (`undo_mark`); new: the run inside the list (Lead call 5).
        let serial = self.marks.fetch_add(1, Ordering::Relaxed) + 1;
        StepMark { at: self.current_id(), serial }
    }

    /// Closes the run of `mark` (a gesture's `Commit` that changed the value): the state the run
    /// made stays, with the label of its first record. Does nothing for a mark that is not the
    /// newest or whose state has left the list.
    pub fn squash_since(&mut self, mark: StepMark) {
        // vectorcraft: crates/engine/src/cmd/menucmds.rs:213 (`squash`); new: the run is one state already, so this only closes it.
        if self.is_newest_open(mark) {
            self.end_run();
        }
    }

    /// Closes the run of `mark` and takes back the state it made (a `Cancel`, or a `Commit` with
    /// `changed: false`): that state is dropped and reported, and the marked state is current
    /// again. In linear history the later states the run's first record dropped stay dropped
    /// (PhotoCraft's `rollback`). A mark whose state has left the list does nothing. Bumps the
    /// revision only when a state was taken back.
    pub fn revert_to(&mut self, mark: StepMark) {
        // pc: crates/ops/src/lib.rs:427-455 (`rollback`); filmcraft: crates/engine/src/lib.rs:978 (`revert_to`)
        if !self.is_newest_open(mark) {
            return;
        }
        let made = self.run.filter(|run| run.kind == (RunKind::Gesture { serial: mark.serial })).map(|run| run.made);
        self.end_run();
        let (Some(made), Some(_)) = (made.and_then(|id| self.row_of(id)), self.row_of(mark.at)) else {
            return;
        };
        self.remove_row(made);
        self.current = self.row_of(mark.at).unwrap_or(0);
        self.revision += 1;
    }

    /// [`record`](Self::record), but extends the previous state when the previous change to the
    /// list was a `record_keyed` with an equal `key` (by hash) and nothing else changed the list
    /// since (`Run::revision`): typing runs, nudges, and external callers' merge keys
    /// (PhotoCraft's `coalesce` param, FilmCraft's and EffectCraft's `merge_key`). Inside an open
    /// gesture run it behaves as `record`.
    pub fn record_keyed(&mut self, step: HistoryStep, key: impl Hash) -> Recorded {
        // pc: crates/engine/src/session/execute.rs:253-269 (same key and `can_undo` updates the current step)
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let key = hasher.finish();
        if self.marks.load(Ordering::Relaxed) > self.closed {
            return self.record(step);
        }
        if let Some(run) = self.run
            && run.kind == (RunKind::Keyed { key })
            && run.revision == self.revision
            && self.row_of(run.made) == Some(self.current)
        {
            self.revision += 1;
            self.run = Some(Run { revision: self.revision, ..run });
            return Recorded { id: run.made, extended: true };
        }
        let recorded = self.record(step);
        self.run = Some(Run { kind: RunKind::Keyed { key }, made: recorded.id, revision: self.revision });
        recorded
    }
}

/// The [`UndoMarks`] type for tool and app options (brush size, hardness, tool options): it
/// wraps the value, its mark is a copy of the value at `Begin`, `revert_to` puts that copy back,
/// and `squash_since` does nothing. It holds no step list, so an option control's gesture
/// (`GestureHost<QuietMarks<BrushSettings>>`) behaves as a document edit — live preview, Esc to
/// cancel, an unchanged commit — and no history state is ever added (Lead call 6).
///
/// ```ignore
/// let mut brush_host: GestureHost<QuietMarks<BrushSettings>> = GestureHost::new();
/// if let Some(event) = size.event { brush_host.handle(ctx, &mut brush_options, event, |o, v| o.0.size = v); }
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct QuietMarks<T>(pub T);

impl<T: Clone> UndoMarks for QuietMarks<T> {
    type Mark = T;

    fn undo_mark(&self) -> T {
        // pc: crates/ui-egui/src/panels/brush_panel.rs:187-210 (`GESTURE`, `gesture()`: the hand-made gesture numbers this replaces)
        self.0.clone()
    }

    fn revert_to(&mut self, mark: T) {
        // new: `self.0 = mark`.
        self.0 = mark;
    }

    fn squash_since(&mut self, _mark: T) {
        // new: nothing to fold; no list.
    }
}
