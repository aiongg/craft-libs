//! `EditHistory<T>`: the undo and redo of a dialog, a tool session or the keymap editor, fed by edit events.

use crate::value::{EditEvent, GestureId, Value};
use std::sync::Arc;

/// Most steps a history keeps.
const LIMIT: usize = 100;

/// Which way [`EditHistory::step`] goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Step {
    /// Back to the value before the latest step.
    Undo,
    /// Forward again after an undo.
    Redo,
}

/// One step: the value it leaves and the name the undo entry shows.
#[derive(Clone, Debug)]
struct Entry<T> {
    value: T,
    name: Arc<str>,
}

/// The gesture under way: its name, taken from its `Begin`.
#[derive(Clone, Debug)]
struct Open {
    id: GestureId,
    name: Arc<str>,
}

/// Undo and redo stacks of a value `T`, at most 100 steps. A gesture is one step: `Begin` opens it, `Commit` closes it,
/// `Cancel` drops it, and a `Set` is one step by itself. A commit that leaves the value as it was records nothing.
#[derive(Clone, Debug)]
pub struct EditHistory<T> {
    undo: Vec<Entry<T>>,
    redo: Vec<Entry<T>>,
    /// The value as of the latest step.
    settled: T,
    open: Option<Open>,
}

impl<T: Value> EditHistory<T> {
    /// A history that starts at `now`.
    pub fn new(now: &T) -> Self {
        EditHistory { undo: Vec::new(), redo: Vec::new(), settled: now.clone(), open: None }
    }

    /// Takes in one event. Events of a gesture other than the open one are ignored.
    pub fn record(&mut self, event: &EditEvent<T>) {
        match event {
            EditEvent::Begin { id, label, .. } => self.open = Some(Open { id: *id, name: label.clone() }),
            EditEvent::Update { .. } => {}
            EditEvent::Commit { id, after, .. } => {
                if let Some(open) = self.open.take_if(|open| open.id == *id) {
                    self.push(after, open.name);
                }
            }
            EditEvent::Cancel { id, .. } => {
                if self.open.as_ref().is_some_and(|open| open.id == *id) {
                    self.open = None;
                }
            }
            EditEvent::Set { after, label, .. } => {
                self.open = None;
                self.push(after, label.clone());
            }
        }
    }

    /// A gesture is open: a caller that restores a draft waits for it to end.
    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    fn push(&mut self, after: &T, name: Arc<str>) {
        if *after == self.settled {
            return;
        }
        if self.undo.len() >= LIMIT {
            self.undo.remove(0);
        }
        self.undo.push(Entry { value: std::mem::replace(&mut self.settled, after.clone()), name });
        self.redo.clear();
    }

    /// Steps `now` back or forward. False when there is nowhere to go. An open gesture is abandoned.
    pub fn step(&mut self, now: &mut T, step: Step) -> bool {
        self.open = None;
        let (from, to) = match step {
            Step::Undo => (&mut self.undo, &mut self.redo),
            Step::Redo => (&mut self.redo, &mut self.undo),
        };
        let Some(entry) = from.pop() else { return false };
        to.push(Entry { value: std::mem::replace(now, entry.value.clone()), name: entry.name });
        self.settled = entry.value;
        true
    }

    /// The name of the step an undo would take back ("Opacity" for "Undo Opacity").
    pub fn undo_name(&self) -> Option<&str> {
        self.undo.last().map(|entry| &*entry.name)
    }

    /// The name of the step a redo would apply.
    pub fn redo_name(&self) -> Option<&str> {
        self.redo.last().map(|entry| &*entry.name)
    }

    /// An undo would change something.
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// A redo would change something.
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}
