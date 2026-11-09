//! The value core of chapter 10: the bound every value type meets, `Mixed`, edit events and how a pass's events
//! fold into one.

use crate::doc_key::DocKey;
use std::sync::Arc;

/// What a bound value must be: cloneable, comparable and shareable across threads.
pub trait Value: Clone + PartialEq + Send + Sync + 'static {}

impl<T: Clone + PartialEq + Send + Sync + 'static> Value for T {}

/// A value that may differ across a multi-selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mixed<T> {
    /// Every selected item holds this value.
    Same(T),
    /// The selected items differ.
    Mixed,
}

impl<T> Mixed<T> {
    /// The shared value, `None` while the items differ.
    pub const fn same(&self) -> Option<&T> {
        match self {
            Mixed::Same(value) => Some(value),
            Mixed::Mixed => None,
        }
    }

    /// The shared value, or `start` while the items differ.
    pub const fn or<'a>(&'a self, start: &'a T) -> &'a T {
        match self {
            Mixed::Same(value) => value,
            Mixed::Mixed => start,
        }
    }

    /// Every part of `values` as a shared value: the per-part form of a composite value no part of which differs.
    pub fn each<const N: usize>(values: [T; N]) -> [Mixed<T>; N] {
        values.map(Mixed::Same)
    }
}

impl<T, const N: usize> Mixed<[Mixed<T>; N]> {
    /// The per-part form of a composite value: a whole `Mixed` binding means every part differs.
    pub fn parts(self) -> [Mixed<T>; N] {
        match self {
            Mixed::Same(parts) => parts,
            Mixed::Mixed => std::array::from_fn(|_| Mixed::Mixed),
        }
    }
}

impl<T> From<T> for Mixed<T> {
    fn from(value: T) -> Self {
        Mixed::Same(value)
    }
}

/// Names a gesture; ids grow in the order the gestures begin.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[non_exhaustive]
pub struct GestureId(pub u64);

impl GestureId {
    /// The id numbered `n`.
    pub const fn new(n: u64) -> GestureId {
        GestureId(n)
    }

    /// The id of the gesture after this one.
    pub const fn next(self) -> GestureId {
        GestureId(self.0 + 1)
    }
}

/// Why a gesture ended without a commit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CancelReason {
    /// The check refused the commit.
    Refused,
    /// The user dismissed the overlay that carried the gesture.
    Dismissed,
    /// Escape.
    Escape,
    /// Focus left the component.
    Blur,
    /// The component was removed while the gesture ran.
    Removed,
    /// The app changed the value mid-gesture.
    HostChanged,
    /// An undo or redo command ran while the gesture was open.
    UndoDuringGesture,
}

impl CancelReason {
    /// The document changed under the gesture, so the gesture's own edits stay and a revert would also take back the
    /// change.
    pub const fn document_moved(self) -> bool {
        matches!(self, CancelReason::HostChanged | CancelReason::UndoDuringGesture)
    }
}

/// What applying an event did to a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Applied {
    /// The value changed.
    Changed,
    /// The value was already the event's.
    Unchanged,
    /// A cancel put the value from before the gesture back.
    Restored,
}

/// When a write-through source is written: on every proposal of a gesture or only on its commit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum EditPolicy {
    /// Every proposal.
    #[default]
    WriteThrough,
    /// The commit only.
    OnCommit,
}

/// What a check says about a value. `M` is the message type: craft-ui exports the alias with its own `Message`.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Validity<T, M> {
    /// Accepted as it is.
    Ok,
    /// Accepted as this value instead.
    Fix(T),
    /// Accepted with a warning.
    Warn(M),
    /// Not accepted; a commit is refused.
    Refuse(M),
}

/// Where a gesture stands in one pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum GesturePhase {
    /// The gesture began in this pass and has no value yet.
    Begin,
    /// The gesture runs.
    Update,
    /// The gesture committed in this pass.
    Commit,
    /// The gesture was cancelled in this pass.
    Cancel,
}

/// One step of an edit. A pass raises several; `fold` reduces them to the one the response carries.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum EditEvent<T> {
    /// A gesture began. It names the undo step.
    #[non_exhaustive]
    Begin {
        /// The gesture.
        id: GestureId,
        /// The value when it began.
        before: Mixed<T>,
        /// The resolved label of the component.
        label: Arc<str>,
        /// The component's document key.
        doc_key: Option<DocKey>,
    },
    /// A proposed value of the running gesture.
    #[non_exhaustive]
    Update {
        /// The gesture.
        id: GestureId,
        /// The proposed value.
        value: T,
    },
    /// The gesture ended with this value.
    #[non_exhaustive]
    Commit {
        /// The gesture.
        id: GestureId,
        /// The value when it began.
        before: Mixed<T>,
        /// The committed value.
        after: T,
    },
    /// The gesture ended without a commit.
    #[non_exhaustive]
    Cancel {
        /// The gesture.
        id: GestureId,
        /// Why it ended.
        reason: CancelReason,
        /// The value when it began.
        before: Option<Mixed<T>>,
    },
    /// A value that arrived as one step, or a gesture that began and committed in one pass. It names itself the way
    /// a `Begin` does. It belongs to no gesture: each `Set` is its own undo step.
    #[non_exhaustive]
    Set {
        /// The value before.
        before: Mixed<T>,
        /// The new value.
        after: T,
        /// The resolved label of the component.
        label: Arc<str>,
        /// The component's document key.
        doc_key: Option<DocKey>,
    },
}

impl<T> EditEvent<T> {
    /// A gesture began.
    pub fn begin(id: GestureId, before: Mixed<T>, label: Arc<str>, doc_key: Option<DocKey>) -> Self {
        EditEvent::Begin { id, before, label, doc_key }
    }

    /// A proposed value of the running gesture.
    pub fn update(id: GestureId, value: T) -> Self {
        EditEvent::Update { id, value }
    }

    /// The gesture committed `after`.
    pub fn commit(id: GestureId, before: Mixed<T>, after: T) -> Self {
        EditEvent::Commit { id, before, after }
    }

    /// The gesture ended without a commit.
    pub fn cancel(id: GestureId, reason: CancelReason, before: Option<Mixed<T>>) -> Self {
        EditEvent::Cancel { id, reason, before }
    }

    /// A one-step change, named the way a `Begin` names its gesture.
    pub fn set(before: Mixed<T>, after: T, label: Arc<str>, doc_key: Option<DocKey>) -> Self {
        EditEvent::Set { before, after, label, doc_key }
    }

    /// The gesture the event belongs to; a `Set` belongs to none.
    pub fn id(&self) -> Option<GestureId> {
        match self {
            EditEvent::Begin { id, .. } | EditEvent::Update { id, .. } | EditEvent::Commit { id, .. } | EditEvent::Cancel { id, .. } => Some(*id),
            EditEvent::Set { .. } => None,
        }
    }

    /// Maps the carried values and keeps the gesture.
    pub fn map<U>(self, f: impl Fn(T) -> U) -> EditEvent<U> {
        let mixed = |m: Mixed<T>| match m {
            Mixed::Same(value) => Mixed::Same(f(value)),
            Mixed::Mixed => Mixed::Mixed,
        };
        match self {
            EditEvent::Begin { id, before, label, doc_key } => EditEvent::Begin { id, before: mixed(before), label, doc_key },
            EditEvent::Update { id, value } => EditEvent::Update { id, value: f(value) },
            EditEvent::Commit { id, before, after } => EditEvent::Commit { id, before: mixed(before), after: f(after) },
            EditEvent::Cancel { id, reason, before } => EditEvent::Cancel { id, reason, before: before.map(mixed) },
            EditEvent::Set { before, after, label, doc_key } => EditEvent::Set { before: mixed(before), after: f(after), label, doc_key },
        }
    }

    /// The value the event carries: a proposal, a commit or a set.
    pub fn value(&self) -> Option<&T> {
        match self {
            EditEvent::Update { value, .. } => Some(value),
            EditEvent::Commit { after, .. } | EditEvent::Set { after, .. } => Some(after),
            EditEvent::Begin { .. } | EditEvent::Cancel { .. } => None,
        }
    }

    /// Where the gesture stands after this event; a `Set` is not a gesture.
    pub fn phase(&self) -> Option<GesturePhase> {
        match self {
            EditEvent::Begin { .. } => Some(GesturePhase::Begin),
            EditEvent::Update { .. } => Some(GesturePhase::Update),
            EditEvent::Commit { .. } => Some(GesturePhase::Commit),
            EditEvent::Cancel { .. } => Some(GesturePhase::Cancel),
            EditEvent::Set { .. } => None,
        }
    }
}

impl<T: Value> EditEvent<T> {
    /// Applies the event to `v`: a proposal, commit or set writes its value, a cancel restores the value from before
    /// the gesture.
    pub fn apply(&self, v: &mut T) -> Applied {
        match self {
            EditEvent::Update { value: new, .. } | EditEvent::Commit { after: new, .. } | EditEvent::Set { after: new, .. } => {
                if *v == *new {
                    Applied::Unchanged
                } else {
                    *v = new.clone();
                    Applied::Changed
                }
            }
            EditEvent::Cancel { before: Some(Mixed::Same(before)), .. } => {
                *v = before.clone();
                Applied::Restored
            }
            EditEvent::Begin { .. } | EditEvent::Cancel { .. } => Applied::Unchanged,
        }
    }

    /// Applies the event to a value that may be mixed: the written value is a concrete one.
    pub fn apply_mixed(&self, v: &mut Mixed<T>) -> Applied {
        match (self, self.value()) {
            (EditEvent::Cancel { before: Some(before), .. }, _) => {
                *v = before.clone();
                Applied::Restored
            }
            (_, Some(new)) if !matches!(v, Mixed::Same(held) if held == new) => {
                *v = Mixed::Same(new.clone());
                Applied::Changed
            }
            _ => Applied::Unchanged,
        }
    }

    /// Folds the events of one pass into the one the response carries: `Begin`..`Commit` is `Set`, `Begin`..`Cancel`
    /// is nothing, `Begin`..`Update` stays `Begin` (it carries no value; the source holds it), otherwise the later
    /// event.
    pub fn fold(prev: Option<EditEvent<T>>, next: EditEvent<T>) -> Option<EditEvent<T>> {
        match (prev, next) {
            (Some(EditEvent::Begin { before, label, doc_key, .. }), EditEvent::Commit { after, .. }) => {
                Some(EditEvent::Set { before, after, label, doc_key })
            }
            (Some(EditEvent::Begin { .. }), EditEvent::Cancel { .. }) => None,
            (Some(begin @ EditEvent::Begin { .. }), EditEvent::Update { .. }) => Some(begin),
            (_, next) => Some(next),
        }
    }
}

/// The running gesture of a value component, kept in shown memory under the component. Plain data; the pipeline
/// reads and writes it.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Gesture<V> {
    /// The running or last gesture.
    pub id: GestureId,
    /// The value when it began; `Some` while a gesture runs.
    pub before: Option<Mixed<V>>,
    /// The latest proposed value.
    pub proposal: Option<V>,
    /// Last pass's given value, read while the gesture runs for the host-change comparison.
    #[doc(hidden)]
    pub last_given: Option<V>,
    /// The last value the pipeline applied or emitted: the value step 11 wrote to a write-through source, or for a
    /// given source `Begin`'s start value and then each `Update`.
    #[doc(hidden)]
    pub emitted: Option<V>,
    /// A given caller's `Update` that fell in the pass of its `Begin`, delivered in the next pass.
    #[doc(hidden)]
    pub held: Option<V>,
    /// A gesture was cancelled under the press that carried it: nothing is proposed until the release.
    #[doc(hidden)]
    pub dead: bool,
    /// The last `cancel_all` serial this gesture has seen.
    #[doc(hidden)]
    pub cancel_seen: u64,
    /// The runtime's removal watch holds this gesture's id.
    #[doc(hidden)]
    pub watched: bool,
}

impl<V> Default for Gesture<V> {
    fn default() -> Self {
        Gesture {
            id: GestureId(0),
            before: None,
            proposal: None,
            last_given: None,
            emitted: None,
            held: None,
            dead: false,
            cancel_seen: 0,
            watched: false,
        }
    }
}

impl<V> Gesture<V> {
    /// The gesture after it ended: no value, the id kept, and `dead` while the press that carried it is held.
    pub fn ended(&self, dead: bool) -> Gesture<V> {
        Gesture { id: self.id, cancel_seen: self.cancel_seen, watched: self.watched, dead, ..Gesture::default() }
    }

    /// A gesture is running.
    pub fn is_running(&self) -> bool {
        self.before.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: GestureId = GestureId(1);

    fn begin() -> EditEvent<i32> {
        EditEvent::Begin { id: ID, before: Mixed::Same(0), label: "Gain".into(), doc_key: None }
    }

    fn fold(events: Vec<EditEvent<i32>>) -> Option<EditEvent<i32>> {
        events.into_iter().fold(None, EditEvent::fold)
    }

    #[test]
    fn a_pass_folds_to_the_event_the_response_carries() {
        let update = |value| EditEvent::Update { id: ID, value };
        let commit = EditEvent::Commit { id: ID, before: Mixed::Same(0), after: 5 };
        let cancel = EditEvent::Cancel { id: ID, reason: CancelReason::Escape, before: Some(Mixed::Same(0)) };
        let set = fold(vec![begin(), update(3), commit.clone()]);
        assert!(matches!(&set, Some(EditEvent::Set { after: 5, label, .. }) if &**label == "Gain"), "Begin..Commit is one Set: {set:?}");
        assert_eq!(fold(vec![begin(), update(3), cancel.clone()]), None, "Begin..Cancel is nothing");
        assert_eq!(fold(vec![begin(), update(3)]), Some(begin()), "Begin..Update stays Begin");
        assert_eq!(fold(vec![update(3), commit.clone()]), Some(commit), "Update..Commit is Commit");
        assert_eq!(fold(vec![update(3), cancel.clone()]), Some(cancel), "Update..Cancel is Cancel");
        assert_eq!(fold(vec![update(3), update(4)]), Some(update(4)), "otherwise the later event");
    }

    #[test]
    fn mixed_parts_read_and_expand_per_part() {
        assert_eq!(Mixed::Same(2).same(), Some(&2));
        assert_eq!(Mixed::<i32>::Mixed.same(), None);
        assert_eq!(*Mixed::<i32>::Mixed.or(&7), 7);
        assert_eq!(*Mixed::Same(2).or(&7), 2);
        assert_eq!(Mixed::each([1, 2]), [Mixed::Same(1), Mixed::Same(2)]);
        assert_eq!(Mixed::<[Mixed<i32>; 2]>::Mixed.parts(), [Mixed::Mixed, Mixed::Mixed]);
        assert_eq!(Mixed::Same([Mixed::Same(1), Mixed::Mixed]).parts(), [Mixed::Same(1), Mixed::Mixed]);
    }

    #[test]
    fn a_cancel_restores_and_a_mixed_value_becomes_concrete() {
        let cancel = EditEvent::Cancel { id: ID, reason: CancelReason::Escape, before: Some(Mixed::Same(7)) };
        let mut value = 9;
        assert_eq!(cancel.apply(&mut value), Applied::Restored);
        assert_eq!(value, 7);
        let mut field = Mixed::Mixed;
        let set = EditEvent::Update { id: ID, value: 2 };
        assert_eq!(set.apply_mixed(&mut field), Applied::Changed);
        assert_eq!(field, Mixed::Same(2));
        assert_eq!(set.apply_mixed(&mut field), Applied::Unchanged);
        let moved = EditEvent::<i32>::Cancel { id: ID, reason: CancelReason::HostChanged, before: None };
        assert_eq!(moved.apply(&mut value), Applied::Unchanged, "a cancel because the document moved restores nothing");
    }
}
