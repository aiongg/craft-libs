//! [`Recording`]: steps the app's journal gives, appended to an action.

use super::list::{ActionList, ActionListError};
use super::model::ActionStep;

/// A recording under way: the action it records into and where the app's journal stood when it
/// started. The app keeps it (PhotoCraft's `ActionState::recording`), filters its journal past
/// `journal_start` for replayable commands (its own rule: not `actions.*` edits, not
/// `edit.transform.again`), and appends them with [`ActionList::append_recorded`] on Stop or
/// before an edit of that action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Recording {
    /// The action recorded into.
    pub action: usize,
    /// The app's journal length when the recording started or last flushed.
    pub journal_start: usize,
}

impl ActionList {
    /// Starts a recording: into the action at `into`, or into a new action with a
    /// [unique name](ActionList::unique_name) (in `set` when given) when `into` is `None`.
    /// `journal_start` is the app's journal length now.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn begin_recording(&mut self, into: Option<usize>, set: Option<String>, journal_start: usize) -> Result<Recording, ActionListError> {
        // pc: crates/engine/src/cmd/actions_cmds.rs:305 `record`.
        todo!()
    }

    /// Appends `steps` to the action at `index` (a flush of the journal while recording).
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn append_recorded(&mut self, index: usize, steps: impl IntoIterator<Item = ActionStep>) -> Result<(), ActionListError> {
        // pc: crates/engine/src/cmd/actions_cmds.rs:344 `flush_recording`.
        todo!()
    }
}
