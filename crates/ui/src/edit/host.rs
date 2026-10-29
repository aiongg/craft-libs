//! [`GestureHost`]: undo steps from [`EditEvent`]s for an application with undo marks.

use std::any::Any;
use std::fmt;
use std::sync::Arc;

use egui::{Context, Modifiers};

use super::{CancelReason, EditEvent, Ending, GestureId, OuterGesture, live};

/// An undo history that can name where it stands, go back there, and fold what came after into
/// one step, as FilmCraft's `Session::undo_mark` and `Session::revert_to` do.
///
/// Every edit the application applies records its own undo step; [`GestureHost`] takes back or
/// folds the steps a gesture made.
pub trait UndoMarks {
    /// A point in the history.
    type Mark;

    /// Where the history stands now.
    fn undo_mark(&self) -> Self::Mark;

    /// Takes back every edit made since `mark`: the document returns to how it was, and neither
    /// undo nor redo keeps the edits. Nothing happens when `mark` has left the history.
    fn revert_to(&mut self, mark: Self::Mark);

    /// Folds every edit made since `mark` into one undo step: the document stays as it is, and
    /// one undo returns it to how it was at `mark`. Nothing happens when `mark` has left the
    /// history or nothing was edited since.
    fn squash_since(&mut self, mark: Self::Mark);
}

/// Turns the controls' events into undo steps, for an application without interaction types of
/// its own: one per document, one gesture at a time.
///
/// - `Begin` takes an undo mark, and keeps the event's `apply` for the gesture's value.
/// - `Update` applies the preview value as an ordinary edit (it may record steps).
/// - `Commit` applies its value when the last update didn't carry it, then folds the gesture's
///   edits into one undo step ([`UndoMarks::squash_since`]); an unchanged commit reverts to the
///   mark instead, leaving no step.
/// - `Cancel` reverts to the mark. A cancel for an [external](CancelReason::External) change
///   leaves the document as it is, the gesture's steps included: reverting would also take
///   back the change.
/// - `Set` applies the value: one undo step.
/// - A `Begin` or `Set` while another gesture is under way first ends that gesture as its
///   control recorded ([`Ending`]): a typing run is kept as one step, a drag is reverted. A
///   typing run's value that no update reported ([`unreported`](super::unreported): a debounce
///   still waiting, [`on_release_only`](super::EditPolicy::on_release_only)) is applied first,
///   with the `apply` given with its `Begin`, so the step holds everything typed.
/// - [`end_frame`](GestureHost::end_frame), called every pass after the controls, ends a gesture
///   whose control was not drawn, the same way.
/// - A dialog's [`OuterGesture`] goes through [`begin_outer`](GestureHost::begin_outer),
///   [`commit`](GestureHost::commit) (OK) and [`cancel`](GestureHost::cancel) (Cancel, Esc).
///
/// Once the host has ended a gesture itself, later events for it are ignored. Its control learns
/// of the end on its next load and carries the input on in a new gesture, which begins at its
/// next change ([`Gesture`](super::Gesture)): a drag or a pen draft that goes on after another
/// control's change reaches the document again.
///
/// On undo during a gesture, and before switching documents, the host cancels the
/// [active](GestureHost::active) gesture against its document and calls
/// [`cancel_all`](super::cancel_all) so the control stops; the control's later `Cancel` is
/// ignored. A command that changes a value during a gesture goes through [`handle`] as a `Set`,
/// so the gesture ends first.
///
/// [`handle`]: GestureHost::handle
pub struct GestureHost<U: UndoMarks> {
    active: Option<Under<U>>,
    /// The newest gesture the host ended before its control did. Ids grow in the order the
    /// gestures' `Begin`s go out ([`GestureId`]), and one gesture is under way at a time, so events
    /// for it and every earlier gesture are stale.
    ended: Option<GestureId>,
}

/// The gesture under way.
struct Under<U: UndoMarks> {
    id: GestureId,
    mark: U::Mark,
    /// The value of its last update the host applied.
    last: Option<Arc<dyn Any + Send + Sync>>,
    /// The `apply` given with its `Begin`, for a value the host holds without its type; `None`
    /// for a dialog's gesture.
    apply: Option<Apply<U>>,
}

/// Applies a value of the gesture's value type; ignores one of another type.
type Apply<U> = Arc<dyn Fn(&mut U, &(dyn Any + Send + Sync)) + Send + Sync>;

impl<U: UndoMarks> GestureHost<U> {
    pub fn new() -> Self {
        Self { active: None, ended: None }
    }

    /// The gesture under way.
    pub fn active(&self) -> Option<GestureId> {
        self.active.as_ref().map(|a| a.id)
    }

    /// Whether a gesture is under way.
    pub fn is_active(&self) -> bool {
        self.active.is_some()
    }

    /// Applies one event; `apply` makes the edit for a value and records its undo step. The host
    /// keeps the `apply` of a `Begin` until the gesture ends, to apply a value of an interrupted
    /// typing run that no update reported, so `apply` is `'static` and reaches what it changes
    /// through the document.
    pub fn handle<T: Clone + PartialEq + Send + Sync + 'static>(
        &mut self,
        ctx: &Context,
        doc: &mut U,
        event: EditEvent<T>,
        apply: impl Fn(&mut U, T) + Send + Sync + 'static,
    ) where
        U: 'static,
    {
        if event.id().is_some_and(|id| self.ended.is_some_and(|ended| id <= ended)) {
            return;
        }
        match event {
            EditEvent::Begin { id } => {
                self.interrupt(ctx, doc);
                let apply: Apply<U> = Arc::new(move |doc, value| {
                    if let Some(value) = value.downcast_ref::<T>() {
                        apply(doc, value.clone());
                    }
                });
                self.active = Some(Under { id, mark: doc.undo_mark(), last: None, apply: Some(apply) });
            }
            EditEvent::Set { value, .. } => {
                self.interrupt(ctx, doc);
                apply(doc, value);
            }
            EditEvent::Update { id, value, .. } => {
                if let Some(a) = self.active.as_mut().filter(|a| a.id == id) {
                    a.last = Some(Arc::new(value.clone()));
                }
                apply(doc, value);
            }
            EditEvent::Commit { id, value, changed, .. } => match self.take(id) {
                Some(a) if changed => {
                    if a.last.as_ref().and_then(|last| last.downcast_ref::<T>()) != Some(&value) {
                        apply(doc, value);
                    }
                    doc.squash_since(a.mark);
                }
                Some(a) => doc.revert_to(a.mark),
                // A gesture this host didn't see begin: its value is one step.
                None if changed => apply(doc, value),
                None => {}
            },
            EditEvent::Cancel { id, reason } => {
                if let Some(a) = self.take(id)
                    && reason != CancelReason::External
                {
                    doc.revert_to(a.mark);
                }
            }
        }
    }

    /// Starts a dialog's gesture at the document's current mark, after ending the gesture under
    /// way as its control recorded.
    pub fn begin_outer(&mut self, ctx: &Context, doc: &mut U, outer: OuterGesture) {
        self.interrupt(ctx, doc);
        self.active = Some(Under { id: outer.id(), mark: doc.undo_mark(), last: None, apply: None });
    }

    /// Ends gesture `id`, keeping its edits as one undo step (a dialog's OK). Returns whether it
    /// was under way.
    pub fn commit(&mut self, doc: &mut U, id: GestureId) -> bool {
        let Some(a) = self.take(id) else { return false };
        doc.squash_since(a.mark);
        self.ended = Some(id);
        true
    }

    /// Cancels gesture `id`: reverts its edits. Returns whether it was under way.
    pub fn cancel(&mut self, doc: &mut U, id: GestureId) -> bool {
        let Some(a) = self.take(id) else { return false };
        doc.revert_to(a.mark);
        self.ended = Some(id);
        true
    }

    /// Call once per pass of every viewport, after its controls: when the active gesture belongs
    /// to this viewport and its control was not drawn ([`is_live`](super::is_live)), ends it as
    /// its control recorded ([`Ending`]) and returns what it applied: a `Commit` (with the
    /// control's last value, applied first when no update reported it) or a `Cancel` with
    /// [`CancelReason::Removed`]. The events carry no value.
    pub fn end_frame(&mut self, ctx: &Context, doc: &mut U) -> Option<EditEvent<()>> {
        let id = self.active()?;
        let live::Judged::Removed(ending) = live::judge(ctx, id) else { return None };
        self.end(ctx, doc, ending);
        live::forget(ctx, id);
        Some(match ending {
            Ending::Commit { changed } => EditEvent::Commit { id, value: (), changed, modifiers: Modifiers::NONE },
            Ending::Cancel => EditEvent::Cancel { id, reason: CancelReason::Removed },
        })
    }

    /// Ends the gesture under way as its control recorded, before another one starts.
    fn interrupt(&mut self, ctx: &Context, doc: &mut U) {
        if let Some(id) = self.active() {
            self.end(ctx, doc, super::ending(ctx, id));
        }
    }

    fn end(&mut self, ctx: &Context, doc: &mut U, ending: Ending) {
        let Some(a) = self.active.take() else { return };
        match ending {
            Ending::Commit { changed: true } => {
                if let (Some(apply), Some(value)) = (&a.apply, live::unreported(ctx, a.id)) {
                    apply(doc, &*value);
                }
                doc.squash_since(a.mark);
            }
            Ending::Commit { changed: false } | Ending::Cancel => doc.revert_to(a.mark),
        }
        live::end(ctx, a.id);
        self.ended = Some(a.id);
    }

    fn take(&mut self, id: GestureId) -> Option<Under<U>> {
        match self.active.take() {
            Some(a) if a.id == id => Some(a),
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
        let active = self.active.as_ref().map(|a| Under { id: a.id, mark: a.mark.clone(), last: a.last.clone(), apply: a.apply.clone() });
        Self { active, ended: self.ended }
    }
}

impl<U: UndoMarks<Mark: fmt::Debug>> fmt::Debug for GestureHost<U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let active = self.active.as_ref().map(|a| (a.id, &a.mark));
        f.debug_struct("GestureHost").field("active", &active).field("ended", &self.ended).finish()
    }
}
