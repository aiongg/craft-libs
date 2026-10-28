//! [`GestureHost`]: undo steps from [`EditEvent`]s for an application with undo marks.

use std::fmt;

use super::{EditEvent, GestureId};

/// An undo history that can name where it stands and go back there, as FilmCraft's
/// `Session::undo_mark` and `Session::revert_to` do.
///
/// Every edit the application applies records its own undo step; [`GestureHost`] takes back the
/// steps a gesture made.
pub trait UndoMarks {
    /// A point in the history.
    type Mark;

    /// Where the history stands now.
    fn undo_mark(&self) -> Self::Mark;

    /// Takes back every edit made since `mark`: the document returns to how it was, and neither
    /// undo nor redo keeps the edits. Nothing happens when `mark` has left the history.
    fn revert_to(&mut self, mark: Self::Mark);
}

/// Turns a control's events into undo steps, for an application without interaction types of
/// its own: one per document, one gesture at a time.
///
/// - `Begin` takes an undo mark.
/// - `Update` applies the preview value as an ordinary edit (it may record steps).
/// - `Commit` reverts to the mark and applies the final value once: one undo step, none when the
///   value didn't change.
/// - `Cancel` reverts to the mark.
/// - `Set` applies the value: one undo step.
///
/// On undo during a gesture, and before switching documents, the host cancels the
/// [active](GestureHost::active) gesture against its document and calls
/// [`cancel_all`](super::cancel_all) so the control stops; the control's later `Cancel` finds
/// nothing under way.
pub struct GestureHost<U: UndoMarks> {
    active: Option<(GestureId, U::Mark)>,
}

impl<U: UndoMarks> GestureHost<U> {
    pub fn new() -> Self {
        Self { active: None }
    }

    /// The gesture under way.
    pub fn active(&self) -> Option<GestureId> {
        self.active.as_ref().map(|(id, _)| *id)
    }

    /// Whether a gesture is under way.
    pub fn is_active(&self) -> bool {
        self.active.is_some()
    }

    /// Applies one event; `apply` makes the edit for a value and records its undo step.
    pub fn handle<T>(&mut self, doc: &mut U, event: EditEvent<T>, mut apply: impl FnMut(&mut U, T)) {
        match event {
            EditEvent::Begin { id } => {
                let mark = doc.undo_mark();
                self.begin(id, mark);
            }
            EditEvent::Update { value, .. } | EditEvent::Set { value } => apply(doc, value),
            EditEvent::Commit { id, value, changed } => self.commit(doc, id, changed.then_some(value), |doc, value| apply(doc, value)),
            EditEvent::Cancel { id, .. } => {
                self.cancel(doc, id);
            }
        }
    }

    /// Starts gesture `id` at `mark`. A gesture still under way ends where it stands: its edits
    /// stay.
    pub fn begin(&mut self, id: GestureId, mark: U::Mark) {
        self.active = Some((id, mark));
    }

    /// Ends gesture `id`: reverts its edits and applies `value`, when given, as one step. For a
    /// gesture this host didn't see begin, only applies `value`.
    pub fn commit<T>(&mut self, doc: &mut U, id: GestureId, value: Option<T>, apply: impl FnOnce(&mut U, T)) {
        if let Some(mark) = self.take(id) {
            doc.revert_to(mark);
        }
        if let Some(value) = value {
            apply(doc, value);
        }
    }

    /// Cancels gesture `id`: reverts its edits. Returns whether it was under way.
    pub fn cancel(&mut self, doc: &mut U, id: GestureId) -> bool {
        let mark = self.take(id);
        let found = mark.is_some();
        if let Some(mark) = mark {
            doc.revert_to(mark);
        }
        found
    }

    fn take(&mut self, id: GestureId) -> Option<U::Mark> {
        match self.active.take() {
            Some((active, mark)) if active == id => Some(mark),
            other => {
                self.active = other;
                None
            }
        }
    }
}

impl<U: UndoMarks> Default for GestureHost<U> {
    fn default() -> Self {
        Self::new()
    }
}

impl<U: UndoMarks<Mark: Clone>> Clone for GestureHost<U> {
    fn clone(&self) -> Self {
        Self { active: self.active.clone() }
    }
}

impl<U: UndoMarks<Mark: fmt::Debug>> fmt::Debug for GestureHost<U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GestureHost").field("active", &self.active).finish()
    }
}
