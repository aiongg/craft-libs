//! The value core of chapter 10: the bound every value type meets, `Mixed`, edit events and how a pass's events
//! fold into one.

use crate::doc_key::DocKey;
use std::sync::Arc;

/// What a bound value must be: cloneable, comparable and shareable across threads.
pub trait Value: Clone + PartialEq + Send + Sync + 'static {}

impl<T: Clone + PartialEq + Send + Sync + 'static> Value for T {}

/// A value that may differ across a multi-selection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mixed<T> {
    /// Every selected item holds this value.
    Same(T),
    /// The selected items differ.
    Mixed,
}

impl<T> From<T> for Mixed<T> {
    fn from(value: T) -> Self {
        Mixed::Same(value)
    }
}

/// Names a gesture; ids grow in the order the gestures begin.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct GestureId(pub u64);

impl GestureId {
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
    Update {
        /// The gesture.
        id: GestureId,
        /// The proposed value.
        value: T,
    },
    /// The gesture ended with this value.
    Commit {
        /// The gesture.
        id: GestureId,
        /// The value when it began.
        before: Mixed<T>,
        /// The committed value.
        after: T,
    },
    /// The gesture ended without a commit.
    Cancel {
        /// The gesture.
        id: GestureId,
        /// Why it ended.
        reason: CancelReason,
        /// The value when it began.
        before: Option<Mixed<T>>,
    },
    /// A value that arrived as one step, or a gesture that began and committed in one pass. It names itself the way
    /// a `Begin` does.
    Set {
        /// The gesture the value belongs to; the last one when it arrived outside a gesture.
        id: GestureId,
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
    /// The gesture the event belongs to.
    pub fn id(&self) -> GestureId {
        match self {
            EditEvent::Begin { id, .. }
            | EditEvent::Update { id, .. }
            | EditEvent::Commit { id, .. }
            | EditEvent::Cancel { id, .. }
            | EditEvent::Set { id, .. } => *id,
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
            EditEvent::Set { id, before, after, label, doc_key } => EditEvent::Set { id, before: mixed(before), after: f(after), label, doc_key },
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
            (Some(EditEvent::Begin { id, before, label, doc_key }), EditEvent::Commit { after, .. }) => {
                Some(EditEvent::Set { id, before, after, label, doc_key })
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
pub struct Gesture<V> {
    /// The running or last gesture.
    pub id: GestureId,
    /// The value when it began; `Some` while a gesture runs.
    pub before: Option<Mixed<V>>,
    /// The latest proposed value.
    pub proposal: Option<V>,
    /// Last pass's given value, read while the gesture runs for the host-change comparison.
    pub last_given: Option<V>,
    /// The last value the pipeline applied or emitted: the value step 11 wrote to a write-through source, or for a
    /// given source `Begin`'s start value and then each `Update`.
    pub emitted: Option<V>,
    /// A given caller's `Update` that fell in the pass of its `Begin`, delivered in the next pass.
    pub held: Option<V>,
    /// A gesture was cancelled under the press that carried it: nothing is proposed until the release.
    pub dead: bool,
    /// The last `cancel_all` serial this gesture has seen.
    pub cancel_seen: u64,
}

impl<V> Default for Gesture<V> {
    fn default() -> Self {
        Gesture { id: GestureId(0), before: None, proposal: None, last_given: None, emitted: None, held: None, dead: false, cancel_seen: 0 }
    }
}

impl<V> Gesture<V> {
    /// The gesture after it ended: no value, the id kept, and `dead` while the press that carried it is held.
    pub fn ended(&self, dead: bool) -> Gesture<V> {
        Gesture { id: self.id, cancel_seen: self.cancel_seen, dead, ..Gesture::default() }
    }

    /// A gesture is running.
    pub fn is_running(&self) -> bool {
        self.before.is_some()
    }
}
