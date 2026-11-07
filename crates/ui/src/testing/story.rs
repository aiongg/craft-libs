//! Stories: named ways to show one registered type, and the data and output the runner reads from them.

use super::Fixture;
use crate::contract::{CollectionEvent, Inner, Outcome, TooltipSpec};
use craft_behavior::{EditEvent, PartKey, Value};
use egui::{Id, Response, Ui};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::ops::Range;

/// A state a registered type must show in some story.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Required {
    /// The ordinary look.
    Default,
    /// A value that differs across a selection.
    Mixed,
    /// A value the check refuses.
    Invalid,
    /// Text longer than its room.
    LongText,
    /// An open popup.
    Open,
    /// A collection with no rows.
    Empty,
    /// A collection with one row.
    One,
    /// A collection with many rows.
    Many,
}

/// How the runner changes the value a story shows.
#[non_exhaustive]
#[derive(Clone, Copy)]
pub enum Drive {
    /// Chosen from the type's facts.
    Auto,
    /// Primary click, Enter, Space and AccessKit `Click`, each in its own run.
    Activate,
    /// AccessKit Increment, then the forward key.
    Step,
    /// A drag of the part by twice the slop along the value axis.
    Drag(PartKey),
    /// Typed text.
    Type(&'static str),
    /// Open, then Down and Enter on the row; a second run clicks the row.
    #[non_exhaustive]
    Pick {
        /// The row to pick.
        row: usize,
    },
    /// The story's own drive.
    Custom(fn(&mut Fixture)),
    /// The story shows a state and changes nothing.
    Never,
}

/// A named way to show one registered type.
#[non_exhaustive]
#[derive(Clone, Copy)]
pub struct Story {
    /// The story's name.
    pub name: &'static str,
    /// The required states it shows.
    pub covers: &'static [Required],
    /// Shows the type.
    pub show: fn(&mut Ui, &mut StoryData) -> StoryOutput,
    /// How the runner changes the shown value.
    pub drive: Drive,
    /// The reason a disabled control shows (`disabled_text` of its tooltip), when the story gives its control one.
    pub disabled_text: Option<&'static str>,
}

impl Story {
    /// A story that shows with `show`.
    pub const fn new(name: &'static str, show: fn(&mut Ui, &mut StoryData) -> StoryOutput) -> Self {
        Story { name, covers: &[], show, drive: Drive::Auto, disabled_text: None }
    }

    /// The states the story shows.
    pub const fn covers(mut self, covers: &'static [Required]) -> Self {
        self.covers = covers;
        self
    }

    /// The reason the story's control shows while disabled. The story gives its tooltip this text through
    /// [`StoryData::tooltip`], and the `disabled` case then checks that the control stays a Tab stop and shows it.
    pub const fn disabled_text(mut self, text: &'static str) -> Self {
        self.disabled_text = Some(text);
        self
    }

    /// The drive the story names.
    pub const fn drive(mut self, drive: Drive) -> Self {
        self.drive = drive;
        self
    }
}

/// A story registered by `#[story]`, linked to its type by the type's registered name.
#[doc(hidden)]
#[non_exhaustive]
pub struct StoryRegistration {
    /// The package that declares the story.
    pub crate_name: &'static str,
    /// The crate target that declares it.
    pub crate_target: &'static str,
    /// The registered name of the type the story shows.
    pub type_name: fn() -> &'static str,
    /// The story.
    pub story: Story,
}

impl StoryRegistration {
    /// The registration `#[story]` writes.
    pub const fn new(crate_name: &'static str, crate_target: &'static str, type_name: fn() -> &'static str, story: Story) -> Self {
        StoryRegistration { crate_name, crate_target, type_name, story }
    }
}

/// Every story linked into the running binary.
#[doc(hidden)]
#[linkme::distributed_slice]
pub static STORIES: [StoryRegistration];

/// A value of a story, its type and how it prints itself.
type Printed = (&'static str, TypeId, fn(&dyn Any) -> String);

/// The values a story shows, by name. Each is a `Value`, so the runner compares them before and after a drive.
#[derive(Default)]
pub struct StoryData {
    values: HashMap<(&'static str, TypeId), Box<dyn Shown>>,
    /// The values that print themselves for the parity replay, in the order they were first used.
    printed: Vec<Printed>,
    /// The story's `disabled_text`.
    disabled_text: Option<&'static str>,
}

/// A story value with its type erased: it copies and compares itself.
trait Shown {
    fn copy(&self) -> Box<dyn Shown>;
    fn same(&self, other: &dyn Shown) -> bool;
    fn any(&self) -> &dyn Any;
    fn any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Value> Shown for T {
    fn copy(&self) -> Box<dyn Shown> {
        Box::new(self.clone())
    }

    fn same(&self, other: &dyn Shown) -> bool {
        other.any().downcast_ref::<T>() == Some(self)
    }

    fn any(&self) -> &dyn Any {
        self
    }

    fn any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl StoryData {
    /// The data of `story`'s runs.
    pub(crate) fn of(story: &Story) -> Self {
        StoryData { disabled_text: story.disabled_text, ..StoryData::default() }
    }

    /// A tooltip of `text`, with the story's `disabled_text` as the text it shows while disabled.
    pub fn tooltip(&self, text: &'static str) -> TooltipSpec {
        let spec = TooltipSpec::new(text);
        match self.disabled_text {
            Some(reason) => spec.disabled_text(reason),
            None => spec,
        }
    }

    /// The value named `name` of type `T`, created from `init` on first use. A name used with two types names two
    /// values.
    #[allow(clippy::unreachable, reason = "the slot key holds the type, so the downcast cannot fail")]
    pub fn value<T: Value>(&mut self, name: &'static str, init: T) -> &mut T {
        let slot = self.values.entry((name, TypeId::of::<T>())).or_insert_with(|| Box::new(init));
        match slot.any_mut().downcast_mut::<T>() {
            Some(value) => value,
            None => unreachable!("a slot's value has the type its key names"),
        }
    }

    /// Like [`StoryData::value`], and the value is one of the story's args in the parity replay, shown as its `Debug`
    /// text in the order the story first uses its values.
    pub fn shown<T: Value + std::fmt::Debug>(&mut self, name: &'static str, init: T) -> &mut T {
        let type_id = TypeId::of::<T>();
        if !self.printed.iter().any(|(printed, id, _)| *printed == name && *id == type_id) {
            self.printed.push((name, type_id, |value| value.downcast_ref::<T>().map_or_else(String::new, |value| format!("{value:?}"))));
        }
        self.value(name, init)
    }

    /// The `shown` values as `(name, Debug text)`, in first-use order.
    pub(crate) fn shown_args(&self) -> Vec<(String, String)> {
        self.printed
            .iter()
            .filter_map(|(name, id, print)| self.values.get(&(*name, *id)).map(|value| ((*name).to_owned(), print(value.any()))))
            .collect()
    }

    /// The names of the values that differ from `before`; a value added since counts as changed.
    pub fn changed_since(&self, before: &StorySnapshot) -> Vec<&'static str> {
        let mut names: Vec<_> = self
            .values
            .iter()
            .filter(|(key, now)| !before.values.iter().any(|(old, was)| old == *key && now.same(&**was)))
            .map(|((name, _), _)| *name)
            .collect();
        names.sort_unstable();
        names
    }

    /// A copy of every value.
    pub fn snapshot(&self) -> StorySnapshot {
        StorySnapshot { values: self.values.iter().map(|(key, value)| (*key, value.copy())).collect() }
    }
}

type Held = ((&'static str, TypeId), Box<dyn Shown>);

/// The values of a [`StoryData`] at one moment.
pub struct StorySnapshot {
    values: Vec<Held>,
}

/// The kind of an edit event.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditKind {
    /// A gesture began.
    Begin,
    /// A gesture proposed a value.
    Update,
    /// A gesture ended with its value.
    Commit,
    /// A gesture was abandoned.
    Cancel,
    /// A one-step change.
    Set,
}

impl EditKind {
    /// The kind of `event`.
    pub fn of<T>(event: &EditEvent<T>) -> Option<EditKind> {
        Some(match event {
            EditEvent::Begin { .. } => EditKind::Begin,
            EditEvent::Update { .. } => EditKind::Update,
            EditEvent::Commit { .. } => EditKind::Commit,
            EditEvent::Cancel { .. } => EditKind::Cancel,
            EditEvent::Set { .. } => EditKind::Set,
            _ => return None,
        })
    }
}

/// The kind of a collection event, with the edit kind of the events that carry one.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollectionKind {
    /// The selection changed.
    Selection,
    /// The lead moved.
    LeadMoved,
    /// Enter or a double click on an item.
    Activate,
    /// A context menu was asked for.
    ContextMenu,
    /// A tree row opened or closed.
    Open,
    /// An item moved to another place.
    Reorder(EditKind),
    /// The selected items of a free layout moved.
    Move(EditKind),
    /// A drop onto an item.
    Dropped,
    /// Files from the OS dropped on the collection.
    FilesDropped,
    /// A rename started, committed or was cancelled.
    Rename(EditKind),
    /// A toggle column's cell flipped.
    Toggle(EditKind),
    /// A column changed.
    Column,
    /// A column header's context menu was asked for.
    HeaderMenu,
    /// An item reported an event.
    Item,
}

impl CollectionKind {
    /// The kind of `event`, `None` for an edit event of a kind the runner does not know.
    pub fn of<K, E>(event: &CollectionEvent<K, E>) -> Option<CollectionKind> {
        Some(match event {
            CollectionEvent::Selection(_) => CollectionKind::Selection,
            CollectionEvent::LeadMoved(_) => CollectionKind::LeadMoved,
            CollectionEvent::Activate(_) => CollectionKind::Activate,
            CollectionEvent::ContextMenu(_) => CollectionKind::ContextMenu,
            CollectionEvent::Open(_) => CollectionKind::Open,
            CollectionEvent::Reorder(edit) => CollectionKind::Reorder(EditKind::of(edit)?),
            CollectionEvent::Move(edit) => CollectionKind::Move(EditKind::of(edit)?),
            CollectionEvent::Dropped { .. } => CollectionKind::Dropped,
            CollectionEvent::FilesDropped { .. } => CollectionKind::FilesDropped,
            CollectionEvent::Rename(edit) => CollectionKind::Rename(EditKind::of(edit)?),
            CollectionEvent::Toggle(edit) => CollectionKind::Toggle(EditKind::of(edit)?),
            CollectionEvent::Column(_) => CollectionKind::Column,
            CollectionEvent::HeaderMenu(_) => CollectionKind::HeaderMenu,
            CollectionEvent::Item(..) => CollectionKind::Item,
        })
    }
}

/// What a story showed, type-erased: the response id, the kinds of its edit events and of its collection events.
#[non_exhaustive]
#[derive(Clone, Debug, Default)]
pub struct StoryOutput {
    /// The shown component's id.
    pub id: Option<Id>,
    /// The edit events' kinds, in order.
    pub edits: Vec<EditKind>,
    /// The pass reported a click, by pointer or by keyboard activation.
    pub clicked: bool,
    /// The collection events' kinds, in drawing order.
    pub collection: Vec<CollectionKind>,
    /// The positions a collection showed.
    pub visible: Option<Range<usize>>,
}

impl StoryOutput {
    pub(crate) fn from_response(response: &Response) -> Self {
        StoryOutput { id: Some(response.id), clicked: super::contract::clicked(response), ..StoryOutput::default() }
    }

    pub(crate) fn of_edit<T>(response: &Response, event: Option<&EditEvent<T>>) -> Self {
        StoryOutput { edits: event.into_iter().filter_map(EditKind::of).collect(), ..Self::from_response(response) }
    }

    pub(crate) fn of_collection<K, E>(response: &Response, events: &[CollectionEvent<K, E>], visible: &Range<usize>) -> Self {
        StoryOutput {
            collection: events.iter().filter_map(CollectionKind::of).collect(),
            visible: Some(visible.clone()),
            ..Self::from_response(response)
        }
    }
}

impl<O: Outcome> From<O> for StoryOutput {
    fn from(output: O) -> Self {
        output.story_facts()
    }
}

impl<O: Outcome, R> From<Inner<O, R>> for StoryOutput {
    fn from(shown: Inner<O, R>) -> Self {
        shown.output.story_facts()
    }
}

/// Shows `row` as the only item of a collection: the item step gives an item its id and its place in the set, which
/// a stand-alone story of an item type would otherwise lack.
pub fn show_as_only_item<C>(ui: &mut Ui, row: C) -> C::Output
where
    C: crate::contract::Component + crate::contract::Props + crate::contract::Show,
{
    let id = ui.scope_id().with("only item");
    let facts =
        crate::runtime::env::ItemFacts { item: id, flags: crate::style::Flags::empty(), position: crate::runtime::access::SetPosition::new(0).of(1) };
    let installed =
        crate::runtime::env::Env::install_for(ui, crate::runtime::env::Env::of(ui).derive_local(|scoped| scoped.item_facts = Some(facts)));
    let shown = row.id(id).show(ui);
    crate::runtime::env::Env::restore(ui.ctx(), installed);
    shown
}
