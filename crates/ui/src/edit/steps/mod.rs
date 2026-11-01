//! The list of history states every History panel and undo list shows: when a state is added,
//! merged, dropped or deleted, linear and non-linear history, the state limit, Clear History, and
//! the grouping marks that make a gesture one state (`StepList`, `StepRows`, `StepMark`,
//! `QuietMarks`).
//!
//! The library owns the list of states; the app owns the document and its undo data, kept in a
//! map keyed by [`StepId`]. It saves the document from before an edit when
//! [`StepList::record`] adds a state, and removes the entries [`StepList::take_dropped`] reports.
//!
//! ```ignore
//! // Recording one edit (inside the app's edit function).
//! let before = doc.steps.current_id();
//! let recorded = doc.steps.record(HistoryStep::new("Brush Tool").icon(Icon::Brush));
//! if !recorded.extended { doc.states.insert(before, State::of(&old_doc)); }
//! for id in doc.steps.take_dropped() { doc.states.remove(&id); }
//!
//! // A gesture reaches the list through `GestureHost`; the document implements `UndoMarks`.
//! impl UndoMarks for Doc {
//!     type Mark = StepMark;
//!     fn undo_mark(&self) -> StepMark { self.steps.mark() }
//!     fn revert_to(&mut self, mark: StepMark) { self.restore(mark); self.steps.revert_to(mark); self.drop_unused(); }
//!     fn squash_since(&mut self, mark: StepMark) { self.steps.squash_since(mark); }
//! }
//!
//! // Tool options never become states: their host is a different type.
//! let mut brush_host: GestureHost<QuietMarks<BrushSettings>> = GestureHost::new();
//! ```
//!
//! Task H2 turns the example into a running doc test
//! (`a_host_over_a_document_makes_one_state_per_gesture_and_one_per_set`).

mod group;
#[cfg(test)]
mod group_tests;
mod list;
#[cfg(test)]
mod tests;

use std::borrow::Cow;

use crate::icons::Icon;

pub use group::{QuietMarks, StepMark};
pub use list::StepList;

/// One history state, unique within its list: ids come from a counter inside the [`StepList`]
/// and are never reused, so an app keys its saved documents by them. An app that mints its own
/// ids (EffectCraft's state addresses, VectorCraft's entries) makes them with [`StepId::new`] to
/// show its states through [`StepRows`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StepId(u64);

impl StepId {
    /// The id `value`.
    pub const fn new(value: u64) -> Self {
        StepId(value)
    }

    /// The number inside, for an app's own commands and logs.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// What the app records: a state's label and its tool icon.
///
/// The label is the text the panel shows, as the app gives it: an app that translates step names
/// at display time (VectorCraft's `entry_name`, WordCraft's `tl!`) sets it again with
/// [`StepList::set_label`] or shows its own rows through [`StepRows`]. A step without an icon
/// leaves an empty icon slot, so labels stay aligned; the library never picks an icon from the
/// label (PhotoCraft's label sniffing, `panels/history.rs:65-75`, is dropped).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryStep {
    /// The state's label: "Brush Tool", "Layer Opacity".
    pub label: String,
    /// The tool's icon at the row's start, if any.
    pub icon: Option<Icon>,
}

impl HistoryStep {
    /// A step labelled `label`, without an icon.
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into(), icon: None }
    }

    /// The tool's icon (`Icon::Brush`; `Icon::Clock` is the History Brush).
    #[must_use]
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }
}

/// What [`StepList::record`] did.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Recorded {
    /// The state the record made current: a new id, or the id of the state an open run made.
    pub id: StepId,
    /// The record replaced the state of an open gesture or keyed run instead of adding one. The
    /// app keeps the document it saved for the run's first record and saves nothing now (it may
    /// update its current document).
    pub extended: bool,
}

/// A move of the current state ([`StepList::go_to`], [`back`](StepList::back),
/// [`forward`](StepList::forward)).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Jump {
    /// The state that was current.
    pub from: StepId,
    /// The state that is current now.
    pub to: StepId,
}

/// What [`StepList::delete`] did.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Deleted {
    /// The current state after the delete; the app shows its document.
    pub now: StepId,
}

/// Why [`StepList::delete`] refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DeleteRefused {
    /// The oldest state is never deleted.
    Oldest,
    /// A later state can be deleted only in non-linear history.
    Later,
    /// No state has that id.
    Unknown,
}

/// The read-only view of a history that the History panel and the undo list take. Row 0 is the
/// oldest state. [`StepList`] implements it; an app with its own engine implements it over its
/// undo and redo lists, so its states show without a `StepList`:
///
/// - VectorCraft: row 0 "Open", then `undo`, then `redo` reversed; `label` returns the
///   translated `entry_name`.
/// - EffectCraft: rows from `history_tree()`; `depth` from the node's depth, `is_later` from
///   `future || !line`, `id` from the node's address id.
/// - FilmCraft, SoundCraft: an "Open" row, then `undo` and `redo`.
/// - LightCraft: the catalog's steps and cursor.
/// - WordCraft: the rows of `undo_history()`, for the
///   [`UndoList`](super::history_panel::UndoList).
///
/// `revision` is required: the panel compares it to follow the current state and to keep a
/// selected snapshot. An app without a counter returns a hash of (`len`, current id, newest id).
#[expect(clippy::len_without_is_empty, reason = "rows always hold the oldest state")]
pub trait StepRows {
    /// The number of rows, at least 1.
    fn len(&self) -> usize;
    /// The id of row `at`.
    fn id(&self, at: usize) -> StepId;
    /// The label of row `at`, as the panel shows it.
    fn label(&self, at: usize) -> Cow<'_, str>;
    /// The icon of row `at`; `None` leaves the slot empty.
    fn icon(&self, at: usize) -> Option<Icon>;
    /// The row of the current state.
    fn current(&self) -> usize;
    /// A counter that changes on every change to the rows and never on a read.
    fn revision(&self) -> u64;
    /// The row's tree depth (EffectCraft's branches); the panel indents tree rows by it.
    fn depth(&self, at: usize) -> usize {
        let _ = at;
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
