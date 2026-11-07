//! Stories: named ways to show one registered type, and the data and output the runner reads from them.

use super::Fixture;
use crate::contract::{EditResponse, Inner};
use craft_behavior::{EditEvent, PartKey, Value};
use egui::{Id, Response, Ui};
use std::any::{Any, TypeId};
use std::collections::HashMap;

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
    /// Open, then Down and Enter on the row.
    Pick(usize),
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
}

impl Story {
    /// A story that shows with `show`.
    pub const fn new(name: &'static str, show: fn(&mut Ui, &mut StoryData) -> StoryOutput) -> Self {
        Story { name, covers: &[], show, drive: Drive::Auto }
    }

    /// The states the story shows.
    pub const fn covers(mut self, covers: &'static [Required]) -> Self {
        self.covers = covers;
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

/// The values a story shows, by name. Each is a `Value`, so the runner compares them before and after a drive.
#[derive(Default)]
pub struct StoryData {
    values: HashMap<(&'static str, TypeId), Box<dyn Shown>>,
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

/// What a story showed, type-erased: the response id and the kinds of its edit events.
#[non_exhaustive]
#[derive(Clone, Debug, Default)]
pub struct StoryOutput {
    /// The shown component's id.
    pub id: Option<Id>,
    /// The edit events' kinds, in order.
    pub edits: Vec<EditKind>,
    /// The pass reported a click, by pointer or by keyboard activation.
    pub clicked: bool,
}

impl From<Response> for StoryOutput {
    fn from(response: Response) -> Self {
        StoryOutput { id: Some(response.id), edits: Vec::new(), clicked: super::contract::clicked(&response) }
    }
}

impl<K, E> From<crate::contract::CollectionResponse<K, E>> for StoryOutput {
    fn from(response: crate::contract::CollectionResponse<K, E>) -> Self {
        StoryOutput { id: Some(response.response.id), edits: Vec::new(), clicked: super::contract::clicked(&response.response) }
    }
}

impl<T> From<EditResponse<T>> for StoryOutput {
    fn from(response: EditResponse<T>) -> Self {
        StoryOutput {
            id: Some(response.response.id),
            edits: response.event.iter().filter_map(EditKind::of).collect(),
            clicked: super::contract::clicked(&response.response),
        }
    }
}

impl<O: Into<StoryOutput>, R> From<Inner<O, R>> for StoryOutput {
    fn from(shown: Inner<O, R>) -> Self {
        shown.output.into()
    }
}
