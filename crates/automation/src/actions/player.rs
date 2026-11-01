//! [`ActionPlayer`], [`ActionRunner`], [`StepOutcome`], [`PlaybackBudget`] and
//! [`PlaybackReport`].

use serde_json::Value;

use super::list::ActionList;
use super::model::{Action, ActionStep};

/// The most steps one top-level playback runs, nested calls included (PhotoCraft's).
// pc: crates/engine/src/cmd/actions_cmds.rs:84 `MAX_PLAYBACK_STEPS`.
pub const MAX_PLAYBACK_STEPS: u64 = 100_000;

/// The deepest nesting of actions playing actions (PhotoCraft's).
// pc: crates/engine/src/cmd/actions_cmds.rs:190 (`playback_stack.len() >= 16`).
pub const MAX_PLAYBACK_DEPTH: usize = 16;

/// What one step did.
#[derive(Clone, Debug, PartialEq)]
pub enum StepOutcome {
    /// It ran; the command's result (a nested call's result is checked by the runner).
    Done(Value),
    /// It stopped for a dialog (a step with its dialog flag, an Insert Stop): playback stops here
    /// and the app resumes later with `from` = this step + 1.
    Paused,
    /// It failed with this message: playback stops.
    Failed(String),
}

/// The app's side of playback: running one step. The player decides the order, skipping and
/// limits; the runner decides what a step means. Nested `actions.play` and
/// `actions.conditional` steps call [`ActionPlayer::play`] again from `run_step` with the same
/// budget.
pub trait ActionRunner {
    /// Runs `step` (index `index` of `action`); `show_dialog` is the step's dialog flag.
    fn run_step(&mut self, action: &Action, index: usize, step: &ActionStep, show_dialog: bool, budget: &mut PlaybackBudget) -> StepOutcome;

    /// The untrusted session's gate, asked before every step, nested ones included: an `Err`
    /// stops playback with that message before the step runs. Allows every step by default.
    fn authorize(&mut self, step: &ActionStep) -> Result<(), String> {
        let _ = step;
        Ok(())
    }

    /// A step Photoshop plays as a silent no-op (Deselect with nothing selected): it counts as run
    /// without calling [`run_step`](Self::run_step). None by default.
    fn is_noop(&self, step: &ActionStep) -> bool {
        let _ = step;
        false
    }
}

/// Why playback did not start or stopped, besides a step's own failure.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PlaybackRefusal {
    /// The action called is already playing further up (a loop).
    Recursive(String),
    /// More than [`MAX_PLAYBACK_DEPTH`] actions are playing inside one another.
    TooDeep,
    /// The [`MAX_PLAYBACK_STEPS`] limit was reached.
    StepLimit,
}

/// The step count and call stack shared by one top-level playback and every action it calls.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaybackBudget {
    pub(super) steps: u64,
    pub(super) limit: u64,
    pub(super) stack: Vec<String>,
}

impl PlaybackBudget {
    /// A budget of [`MAX_PLAYBACK_STEPS`] for a new top-level playback.
    pub fn new() -> Self {
        Self { steps: 0, limit: MAX_PLAYBACK_STEPS, stack: Vec::new() }
    }

    /// Charges one step; `false` once the limit is reached (the step must not run). A script host
    /// with its own limit charges that one itself.
    #[expect(clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn charge(&mut self) -> bool {
        // pc: crates/engine/src/cmd/actions_cmds.rs:158 `consume_playback_step`.
        todo!()
    }

    /// Enters the action named `name`: refused when it is already on the stack (a call back to a
    /// running action) or the stack is [`MAX_PLAYBACK_DEPTH`] deep.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn enter(&mut self, name: &str) -> Result<(), PlaybackRefusal> {
        // pc: crates/engine/src/cmd/actions_cmds.rs:187-192 (`playback_plan`: recursive call, nesting over 16).
        todo!()
    }

    /// Leaves the innermost action.
    pub fn leave(&mut self) {
        self.stack.pop();
    }

    /// Steps charged so far.
    pub fn steps(&self) -> u64 {
        self.steps
    }
}

impl Default for PlaybackBudget {
    fn default() -> Self {
        Self::new()
    }
}

/// A step that stopped playback.
#[derive(Clone, Debug, PartialEq)]
pub struct StepFailure {
    /// The step's index in its action.
    pub step: usize,
    /// Its command id.
    pub command: String,
    /// The message (the runner's, the gate's, or the limit's: "action playback exceeded the
    /// 100000-step limit").
    pub message: String,
}

/// What one [`ActionPlayer::play`] did.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlaybackReport {
    /// Steps run (no-ops included).
    pub ran: u64,
    /// The step that failed, if one did.
    pub failed: Option<StepFailure>,
    /// The step that paused for a dialog, if one did; the app resumes from the next step.
    pub paused_at: Option<usize>,
    /// Playback did not start or stopped for a limit or a loop.
    pub refused: Option<PlaybackRefusal>,
}

/// Plays recorded actions.
pub struct ActionPlayer;

impl ActionPlayer {
    /// Plays the action at `index` of `list` from step `from`: enters it on `budget` (refusing a
    /// loop or depth), then for each included step at or after `from`, in order: charges the
    /// budget (stops with the step-limit failure when spent), asks
    /// [`authorize`](ActionRunner::authorize) (a denial stops without running or pausing), counts
    /// a [no-op](ActionRunner::is_noop) as run, else runs the step; stops at the first `Failed`
    /// and at `Paused` (reporting `paused_at`). Skipped (disabled) steps are not charged. Leaves
    /// the action on `budget` before returning. An index past the list reports
    /// `failed` with the message "no such action". Playing the action being recorded into is the
    /// app's to refuse (PhotoCraft's `playback_plan` does), since the list holds no recording.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn play(list: &ActionList, index: usize, from: usize, runner: &mut dyn ActionRunner, budget: &mut PlaybackBudget) -> PlaybackReport {
        // pc: crates/engine/src/cmd/actions_cmds.rs:384 `run_recorded`; crates/ui-egui/src/panels/actions.rs:542-615 (pause and resume).
        todo!()
    }
}
