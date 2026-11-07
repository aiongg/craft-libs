//! Values and edits (chapter 10): the binding, the output of a value component and the types that carry a value
//! between the app and the pipeline.

use crate::runtime::env::Message;
use crate::runtime::memory::{MemoryStage, Savable, SaveKey};
use craft_behavior::{Applied, EditEvent, EditPolicy, Mixed, Value};
use egui::Response;
use std::sync::Arc;

/// What a check says about a value, with the message type of craft-ui.
pub type Validity<T> = craft_behavior::Validity<T, Message>;

/// The value type of a component with no value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NoValue;

/// What `show` returns: the component's response and the type of value it edits.
pub trait Outcome {
    /// The value type; `NoValue` for a component with none.
    type Value: Value;

    /// The component's response.
    fn response(&self) -> &Response;

    /// The facts a story reports about this output: its id, click, edit kinds, collection kinds and visible range.
    /// Outputs that hold an `EditResponse` or a `CollectionResponse` forward to it.
    #[cfg(feature = "testing")]
    #[doc(hidden)]
    fn story_facts(&self) -> crate::testing::StoryOutput {
        crate::testing::StoryOutput::from_response(self.response())
    }
}

impl Outcome for Response {
    type Value = NoValue;

    fn response(&self) -> &Response {
        self
    }
}

/// Whether `response` reports a click, by pointer or by keyboard activation.
pub fn clicked(response: &Response) -> bool {
    crate::runtime::egui::response::clicked(response)
}

/// The value type of `C`'s output.
pub type ValueOf<C> = <<C as super::Component>::Output as Outcome>::Value;

/// `held` when it equals `value`, else a new copy of it.
pub(crate) fn reuse_arc<V: Value>(held: &Option<Arc<V>>, value: &V) -> Arc<V> {
    match held {
        Some(held) if **held == *value => held.clone(),
        _ => Arc::new(value.clone()),
    }
}

/// The shown copy a step 4 comparison keeps.
type Held<T> = Option<Arc<T>>;

/// What `Bind::saved` captures while `T: Savable` is known, so the generic pipeline reaches the saved tier without
/// the bound.
struct SaveSpec<T> {
    key: SaveKey,
    /// Reads the saved value, reusing the held copy when it is equal.
    load: fn(&mut MemoryStage, &SaveKey, &Held<T>) -> Option<Arc<T>>,
    store: fn(&mut MemoryStage, &SaveKey, T),
}

type Check<'a, T> = dyn Fn(&T) -> Validity<T> + 'a;

/// Where a binding's value lives.
enum Source<'a, T> {
    /// Value in, event out.
    Given(Mixed<T>),
    /// Write-through.
    Mut(&'a mut T),
    /// Write-through for a multi-selection field.
    MutMixed(&'a mut Mixed<T>),
    /// No caller value: session memory under the component id, or the saved tier.
    Kept,
}

/// The binding of a value component: a value the app owns, the check on it and the value a reset restores. The same
/// binding produces the same events whichever source it has.
pub struct Bind<'a, T> {
    source: Source<'a, T>,
    policy: EditPolicy,
    check: Option<Box<Check<'a, T>>>,
    save: Option<SaveSpec<T>>,
    reset: Option<T>,
}

impl<'a, T: Value> Bind<'a, T> {
    /// A value in; every edit comes out as an event.
    pub fn given(v: impl Into<Mixed<T>>) -> Self {
        Self::from_source(Source::Given(v.into()))
    }

    /// Edits write through to `v`.
    pub fn bound(v: &'a mut T) -> Self {
        Self::from_source(Source::Mut(v))
    }

    /// Edits write through to a field that may be mixed; an edit makes it concrete.
    pub fn bound_mixed(v: &'a mut Mixed<T>) -> Self {
        Self::from_source(Source::MutMixed(v))
    }

    /// No caller value: the component keeps it in session memory under its id.
    pub fn kept() -> Self {
        Self::from_source(Source::Kept)
    }

    fn from_source(source: Source<'a, T>) -> Self {
        Bind { source, policy: EditPolicy::default(), check: None, save: None, reset: None }
    }

    /// A kept value that is also saved across runs under `key`. The `Savable` bound sits here; the pipeline reaches
    /// the saved tier through the captured load and store functions.
    pub fn saved(mut self, key: SaveKey) -> Self
    where
        T: Savable,
    {
        self.source = Source::Kept;
        self.save = Some(SaveSpec {
            key,
            load: |stage, key, held| stage.saved::<Option<T>>(key).as_ref().map(|value| reuse_arc(held, value)),
            store: |stage, key, value| *stage.saved::<Option<T>>(key) = Some(value),
        });
        self
    }

    /// When a write-through source is written.
    pub fn policy(mut self, policy: EditPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// The check on a value: `Fix` replaces it, `Warn` and `Refuse` set the component's state.
    pub fn check(mut self, check: impl Fn(&T) -> Validity<T> + 'a) -> Self {
        self.check = Some(Box::new(check));
        self
    }

    /// The value a reset gesture restores.
    pub fn reset(mut self, default: T) -> Self {
        self.reset = Some(default);
        self
    }

    /// The binding as the pipeline reads and writes it.
    pub fn reborrow(&mut self) -> BindMut<'_, T> {
        let source = match &mut self.source {
            Source::Given(value) => SourceMut::Given(value),
            Source::Mut(value) => SourceMut::Mut(&mut **value),
            Source::MutMixed(value) => SourceMut::MutMixed(&mut **value),
            Source::Kept => SourceMut::Kept,
        };
        BindMut { source, policy: self.policy, check: self.check.as_deref(), save: self.save.as_ref(), reset: self.reset.as_ref() }
    }
}

enum SourceMut<'a, T> {
    Given(&'a Mixed<T>),
    Mut(&'a mut T),
    MutMixed(&'a mut Mixed<T>),
    Kept,
}

/// A reborrow of a [`Bind`] the pipeline reads at steps 4, 11, 15 and 17. `Component::bind` returns it, and returns the
/// same binding on every call.
pub struct BindMut<'a, T> {
    source: SourceMut<'a, T>,
    policy: EditPolicy,
    check: Option<&'a Check<'a, T>>,
    save: Option<&'a SaveSpec<T>>,
    reset: Option<&'a T>,
}

impl<T: Value> BindMut<'_, T> {
    /// The binding's value: `Mixed` for a multi-selection field, none for a kept binding.
    pub(crate) fn get(&self) -> Option<Mixed<T>> {
        match &self.source {
            SourceMut::Given(value) => Some((*value).clone()),
            SourceMut::Mut(value) => Some(Mixed::Same((**value).clone())),
            SourceMut::MutMixed(value) => Some((**value).clone()),
            SourceMut::Kept => None,
        }
    }

    pub(crate) fn is_kept(&self) -> bool {
        matches!(self.source, SourceMut::Kept)
    }

    pub(crate) fn is_given(&self) -> bool {
        matches!(self.source, SourceMut::Given(_))
    }

    /// The binding holds one value equal to `shown`: a compare that clones nothing, so step 4 copies the value only
    /// when it changed.
    pub(crate) fn shown_matches(&self, shown: &T) -> bool {
        match &self.source {
            SourceMut::Given(Mixed::Same(value)) | SourceMut::MutMixed(Mixed::Same(value)) => value == shown,
            SourceMut::Mut(value) => **value == *shown,
            _ => false,
        }
    }

    /// The check's verdict on `value`.
    pub(crate) fn validity(&self, value: &T) -> Validity<T> {
        self.check.map_or(Validity::Ok, |check| check(value))
    }

    /// The saved tier of a kept binding that has a `SaveKey`: `None` without one, `Some(None)` when nothing was saved.
    pub(crate) fn saved_value(&self, stage: &mut MemoryStage, held: &Option<Arc<T>>) -> Option<Option<Arc<T>>> {
        self.save.as_ref().map(|save| (save.load)(stage, &save.key, held))
    }

    /// Stores into the saved tier; false when the binding has no `SaveKey`.
    pub(crate) fn saved_put(&self, stage: &mut MemoryStage, value: T) -> bool {
        self.save.as_ref().map(|save| (save.store)(stage, &save.key, value)).is_some()
    }

    /// The value a reset restores.
    pub(crate) fn reset_value(&self) -> Option<&T> {
        self.reset
    }

    /// Whether a write-through source takes `event`: every value under `WriteThrough`, commits and sets under `OnCommit`.
    pub(crate) fn writes(&self, event: &EditEvent<T>) -> bool {
        match self.source {
            SourceMut::Given(_) | SourceMut::Kept => false,
            SourceMut::Mut(_) | SourceMut::MutMixed(_) => self.policy == EditPolicy::WriteThrough || !matches!(event, EditEvent::Update { .. }),
        }
    }

    /// Applies `event` to a write-through source; a given or kept binding has nothing to write.
    pub(crate) fn apply(&mut self, event: &EditEvent<T>) -> Applied {
        if !self.writes(event) {
            return Applied::Unchanged;
        }
        match &mut self.source {
            SourceMut::Mut(value) => event.apply(&mut **value),
            SourceMut::MutMixed(value) => event.apply_mixed(&mut **value),
            SourceMut::Given(_) | SourceMut::Kept => Applied::Unchanged,
        }
    }
}

/// The output of a component that edits a value. Only `cx.edit_response()` builds it.
#[derive(Debug)]
#[non_exhaustive]
pub struct EditResponse<T> {
    /// The component's response.
    pub response: Response,
    /// The pass's events folded into one (`EditEvent::fold`); `None` when nothing was edited.
    pub event: Option<EditEvent<T>>,
    /// The value was submitted (Enter in a text part).
    pub submitted: bool,
    /// The edit was abandoned with Escape.
    pub escaped: bool,
    /// What the binding's check says about the shown value.
    pub validity: Validity<T>,
}

impl<T> EditResponse<T> {
    pub(crate) fn new(response: Response, event: Option<EditEvent<T>>, validity: Validity<T>) -> Self {
        EditResponse { response, event, submitted: false, escaped: false, validity }
    }
}

impl<T: Value> Outcome for EditResponse<T> {
    type Value = T;

    fn response(&self) -> &Response {
        &self.response
    }

    #[cfg(feature = "testing")]
    fn story_facts(&self) -> crate::testing::StoryOutput {
        crate::testing::StoryOutput::of_edit(&self.response, self.event.as_ref())
    }
}

/// What an item's output may report beyond its response.
pub trait ItemOutput<E>: Outcome {
    /// The event the collection reports for this item, `None` when the item reports nothing.
    fn into_event(self) -> Option<E>;

    /// The rect of the item's label, where a rename editor goes; `None` puts it over the whole row.
    fn label_rect(&self) -> Option<egui::Rect> {
        None
    }
}

impl<E> ItemOutput<E> for Response {
    fn into_event(self) -> Option<E> {
        None
    }
}
