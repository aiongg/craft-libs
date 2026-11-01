//! [`ActionList`]: the flat list of actions with an optional set each, and every edit the panel
//! and the app's commands need.

use super::file::ActionsFile;
use super::model::{Action, ActionColor, ActionStep};

/// Why an [`ActionList`] edit was refused. The app shows the message; nothing changed.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ActionListError {
    /// No action at this index.
    NoSuchAction(usize),
    /// No step at this index of the action.
    NoSuchStep {
        /// The action.
        action: usize,
        /// The step.
        step: usize,
    },
    /// A destination past the end.
    OutOfRange,
    /// Another action (or set) already has the name.
    NameTaken(String),
    /// The name is empty or only white space.
    EmptyName,
}

impl std::fmt::Display for ActionListError {
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // pc: crates/engine/src/cmd/actions_cmds.rs:507, :636, :627 (the engine's messages: "destination is out of range",
        // "an action with this name already exists", "\"name\" is empty").
        todo!()
    }
}

impl std::error::Error for ActionListError {}

/// The recorded actions: one flat list (PhotoCraft's engine, journal and file are flat), each
/// action with an optional set name, plus the names of sets that hold no action yet, so New Set
/// works before anything is recorded into it (lead call L17; VectorCraft's nested sets map onto
/// it).
///
/// The list is app data shared by the panel, the app's commands, the CLI and the control channel:
/// the panel only reads it and reports events, and the app applies each event with these calls
/// (owner question 1, recommended answer). Every successful edit bumps [`revision`](Self::revision),
/// so a store can skip unchanged writes. Indices are into [`actions`](Self::actions).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ActionList {
    pub(super) actions: Vec<Action>,
    pub(super) sets: Vec<String>,
    pub(super) revision: u64,
}

impl ActionList {
    /// An empty list.
    pub fn new() -> Self {
        Self::default()
    }

    /// The actions in list order.
    pub fn actions(&self) -> &[Action] {
        &self.actions
    }

    /// The action at `index`.
    pub fn action(&self, index: usize) -> Option<&Action> {
        self.actions.get(index)
    }

    /// Every set name in display order: sets of actions in order of first appearance, then the
    /// empty sets in the order they were made.
    #[expect(clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn sets(&self) -> Vec<&str> {
        // pc: crates/ui-egui/src/panels/actions.rs:809 `display_order` (sets by first appearance); new: empty sets after.
        todo!()
    }

    /// The set of the action at `index`.
    pub fn set_of(&self, index: usize) -> Option<&str> {
        self.actions.get(index).and_then(|action| action.set.as_deref())
    }

    /// The panel's order of action indices: loose actions first, then each set's actions, sets
    /// in [`sets`](Self::sets) order.
    #[expect(clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn display_order(&self) -> Vec<usize> {
        // pc: crates/ui-egui/src/panels/actions.rs:809 `display_order`.
        todo!()
    }

    /// The number of steps of the action at `index` (0 when there is none).
    pub fn step_count(&self, index: usize) -> usize {
        self.actions.get(index).map_or(0, |action| action.steps.len())
    }

    /// `base` if no action has it, else `base 2`, `base 3` … the first free one. With an empty
    /// `base`, PhotoCraft's `Action n` from the list's length + 1 up.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn unique_name(&self, base: &str) -> String {
        // pc: crates/engine/src/cmd/actions_cmds.rs:313-318 (`Action {n}` from len + 1).
        todo!()
    }

    /// Adds an empty action named `name` at the end (in `set` when given); returns its index.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn new_action(&mut self, name: &str, set: Option<String>) -> Result<usize, ActionListError> {
        // pc: crates/engine/src/cmd/actions_cmds.rs:310-330 (`record` without `action`: empty or taken names refused).
        todo!()
    }

    /// Adds an empty set named `name`; refuses a taken or empty name.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn new_set(&mut self, name: &str) -> Result<(), ActionListError> {
        // vectorcraft: crates/ui-egui/src/panels/actions.rs:87-92 (a set made with New Set is current with none of its actions).
        todo!()
    }

    /// Renames the action at `index` to `name` (trimmed name not empty, not taken; the same name
    /// is a success that changes nothing). `calls` sees every step of every action with the old
    /// and new names, so the app rewrites its own calls (PhotoCraft: `actions.play`'s `action`,
    /// `actions.conditional`'s `then` and `else`); the app also moves its shortcut override and
    /// rewrites its pending journal itself.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn rename(&mut self, index: usize, name: &str, calls: impl FnMut(&mut ActionStep, &str, &str)) -> Result<(), ActionListError> {
        // pc: crates/engine/src/cmd/actions_cmds.rs:622 `rename`.
        todo!()
    }

    /// Removes and returns the action at `index`.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn delete_action(&mut self, index: usize) -> Result<Action, ActionListError> {
        // pc: crates/engine/src/cmd/actions_cmds.rs:533 `delete`.
        todo!()
    }

    /// Removes and returns step `step` of action `action`; `disabled` and `dialogs` follow (the
    /// removed index is dropped, later ones move down).
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn delete_step(&mut self, action: usize, step: usize) -> Result<ActionStep, ActionListError> {
        // pc: crates/engine/src/cmd/actions_cmds.rs:533 `delete` (with `step`).
        todo!()
    }

    /// Moves the action at `from` to `to` (an index in the list after the removal, PhotoCraft's
    /// `actions.move`).
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn move_action(&mut self, from: usize, to: usize) -> Result<(), ActionListError> {
        // pc: crates/engine/src/cmd/actions_cmds.rs:504-516 (`move_item` without `step`).
        todo!()
    }

    /// Puts the action at `index` into `set` (`None`: out of any set), as a drop into or out of a
    /// set folder does.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn assign_set(&mut self, index: usize, set: Option<String>) -> Result<(), ActionListError> {
        // pc: crates/engine/src/cmd/actions_edit_cmds.rs:112 `assign_set`.
        todo!()
    }

    /// Moves step `step` of action `from` to position `to` of action `into` (the same action or
    /// another). Within one action `disabled` and `dialogs` follow [`moved_index`]; between
    /// actions the step takes its include and dialog flags along.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn move_step(&mut self, from: usize, step: usize, into: usize, to: usize) -> Result<(), ActionListError> {
        // pc: crates/engine/src/cmd/actions_cmds.rs:488-502 (`move_item` with `step`, within one action);
        // new: between actions (spec Behaviour, Actions "Drag a row").
        todo!()
    }

    /// Switches step `step` of `action` on or off for playback; returns its new state.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn toggle_step(&mut self, action: usize, step: usize) -> Result<bool, ActionListError> {
        // pc: crates/engine/src/cmd/actions_cmds.rs:600-607 (`toggle` with `step`).
        todo!()
    }

    /// The include column on an action row: every step on unless all already are, in which case
    /// every step off. Returns whether the steps are now on.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn toggle_all(&mut self, action: usize) -> Result<bool, ActionListError> {
        // pc: crates/engine/src/cmd/actions_cmds.rs:608-609 (`toggle` without `step`).
        todo!()
    }

    /// Switches the dialog flag of step `step` of `action`; returns its new state.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn toggle_dialog(&mut self, action: usize, step: usize) -> Result<bool, ActionListError> {
        // pc: crates/engine/src/cmd/actions_edit_cmds.rs:78 `dialog`.
        todo!()
    }

    /// Sets the Button Mode colour of the action at `index`.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn set_color(&mut self, index: usize, color: ActionColor) -> Result<(), ActionListError> {
        // pc: crates/engine/src/cmd/actions_cmds.rs:667 `set_color`.
        todo!()
    }

    /// The list a file holds (its empty sets included).
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn from_file(file: ActionsFile) -> Self {
        // new: `actions` and `sets` taken over; revision 0.
        todo!()
    }

    /// The list as a file (version [`ActionsFile::VERSION`]; the empty sets in `sets`).
    #[expect(clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn to_file(&self) -> ActionsFile {
        // pc: crates/engine/src/preset_store.rs:1112 (`ActionsFile { version: 1, actions }`).
        todo!()
    }

    /// Bumped by every successful edit.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Marks the list changed by an edit made outside these calls (the app replaced an action's
    /// steps).
    pub fn touch(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }
}

/// Where the item at `index` ends up after the item at `from` moved to `to`: the moved item goes
/// to `to`, items between shift by one toward `from`, others stay. A selection or the recording
/// index follows a move with it.
#[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
pub fn moved_index(index: usize, from: usize, to: usize) -> usize {
    // pc: crates/engine/src/cmd/actions_cmds.rs:521 `moved_index`.
    todo!()
}
