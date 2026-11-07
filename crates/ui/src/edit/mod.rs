//! Undo adapters over `EditHistory`, panel events and `DialogValue`.
//!
//! The adapter turns the events of a value component into undo steps of an app that keeps its own history, behind one
//! sink trait. Dialogs, tool sessions and the keymap editor use `craft_behavior::EditHistory` instead.

use crate::runtime::edit::cancel_all;
use craft_behavior::{CancelReason, DocKey, EditEvent, GestureId, Mixed, Value};
use egui::Context;
use std::any::Any;
use std::sync::Arc;

/// The name of an undo step: the component's resolved label and its document key, both carried by the event that begins
/// the step.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct StepName {
    /// The resolved label ("Opacity" for "Undo Opacity").
    pub label: Arc<str>,
    /// The component's document key.
    pub doc_key: Option<DocKey>,
}

/// An undo history that can name where it stands, go back there, and fold what came after into one named step.
///
/// The app applies every edit as it would any other and records the steps that edit makes; [`GestureHost`] takes back
/// or folds the steps a gesture made.
pub trait UndoMarks {
    /// A point in the history.
    type Mark;

    /// Where the history stands now.
    fn undo_mark(&self) -> Self::Mark;

    /// Takes back every edit made since `mark`: the document returns to how it was, and neither undo nor redo keeps the
    /// edits. Nothing happens when `mark` has left the history.
    fn revert_to(&mut self, mark: Self::Mark);

    /// Folds every edit made since `mark` into one undo step called `name`: the document stays as it is, and one undo
    /// returns it to how it was at `mark`. Nothing happens when `mark` has left the history or nothing was edited since.
    fn squash_since(&mut self, mark: Self::Mark, name: &StepName);
}

/// The gesture under way.
struct Under<U: UndoMarks> {
    id: GestureId,
    mark: U::Mark,
    name: StepName,
    /// The last value applied for the gesture, to apply a commit's value only when it differs.
    last: Option<Arc<dyn Any + Send + Sync>>,
}

/// Turns the events of value components into undo steps for a document: one gesture at a time.
///
/// - `Begin` takes an undo mark and names the step.
/// - `Update` applies the preview value as an ordinary edit.
/// - `Commit` applies its value when the last update did not carry it, then folds the gesture's edits into one named
///   step; a commit that leaves the value as it was reverts to the mark, leaving no step.
/// - `Cancel` reverts to the mark. A cancel because the document moved under the gesture (`HostChanged`,
///   `UndoDuringGesture`) leaves the document as it is and folds the gesture's steps, so later edits do not join them.
/// - `Set` applies the value as one named step.
/// - A `Begin` or `Set` while another gesture is under way first reverts that gesture.
///
/// The `Update`, `Commit` and `Cancel` of a gesture the host has ended are ignored.
pub struct GestureHost<U: UndoMarks> {
    active: Option<Under<U>>,
    /// The newest gesture the host ended before its component did. Ids grow in the order gestures begin and one gesture
    /// is under way at a time, so its remaining events and those of every earlier gesture are stale.
    ended: Option<GestureId>,
}

impl<U: UndoMarks> Default for GestureHost<U> {
    fn default() -> Self {
        GestureHost { active: None, ended: None }
    }
}

impl<U: UndoMarks> GestureHost<U> {
    /// A host with no gesture under way.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether a gesture is under way.
    pub fn is_active(&self) -> bool {
        self.active.is_some()
    }

    /// Applies one event; `apply` makes the edit for a value and records its undo step.
    pub fn handle<T: Value>(&mut self, doc: &mut U, event: EditEvent<T>, apply: impl Fn(&mut U, T)) {
        let stale = matches!(event, EditEvent::Update { .. } | EditEvent::Commit { .. } | EditEvent::Cancel { .. })
            && self.ended.is_some_and(|ended| event.id() <= ended);
        if stale {
            return;
        }
        match event {
            EditEvent::Begin { id, label, doc_key, .. } => {
                self.revert(doc);
                self.active = Some(Under { id, mark: doc.undo_mark(), name: StepName { label, doc_key }, last: None });
            }
            EditEvent::Update { id, value, .. } => {
                if let Some(under) = self.active.as_mut().filter(|under| under.id == id) {
                    under.last = Some(Arc::new(value.clone()));
                    apply(doc, value);
                }
            }
            EditEvent::Commit { id, before, after, .. } => {
                if let Some(under) = self.take(id) {
                    if matches!(&before, Mixed::Same(held) if *held == after) {
                        doc.revert_to(under.mark);
                    } else {
                        if under.last.as_ref().and_then(|last| last.downcast_ref::<T>()) != Some(&after) {
                            apply(doc, after);
                        }
                        doc.squash_since(under.mark, &under.name);
                    }
                }
            }
            EditEvent::Cancel { id, reason, .. } => {
                if let Some(under) = self.take(id) {
                    if reason.document_moved() {
                        doc.squash_since(under.mark, &under.name);
                    } else {
                        doc.revert_to(under.mark);
                    }
                }
            }
            EditEvent::Set { after, label, doc_key, .. } => {
                self.revert(doc);
                let mark = doc.undo_mark();
                apply(doc, after);
                doc.squash_since(mark, &StepName { label, doc_key });
            }
            _ => {}
        }
    }

    /// The app's undo or redo command: cancels every running gesture of the UI and takes back the one under way, then the
    /// app changes the document. The component's later `Cancel` is ignored.
    pub fn before_undo(&mut self, ctx: &Context, doc: &mut U) {
        cancel_all(ctx, CancelReason::UndoDuringGesture);
        self.revert(doc);
    }

    /// Reverts the gesture under way, as a gesture that began elsewhere does.
    fn revert(&mut self, doc: &mut U) {
        if let Some(under) = self.active.take() {
            doc.revert_to(under.mark);
            self.ended = Some(under.id);
        }
    }

    fn take(&mut self, id: GestureId) -> Option<Under<U>> {
        match self.active.take() {
            Some(under) if under.id == id => Some(under),
            other => {
                self.active = other;
                None
            }
        }
    }
}
