//! [`LocalHistory`]: undo and redo inside a dialog or a tool session, so Cmd/Ctrl+Z steps the
//! open dialog's own settings, or the session's draft, instead of reaching the document beneath
//! it.
//!
//! A dialog keeps a [`LocalHistory`] of its settings and calls [`settle`](LocalHistory::settle)
//! every pass. A change becomes one step once no edit is in progress, so a slider drag is one
//! step rather than one per frame. [`Dialog::history`](super::Dialog::history) does this for a
//! dialog; workspaces with their own stacks (Liquify, Filter Gallery, Wide Angle, Camera Raw) use
//! `LocalHistory` and [`FullWindowFx::take_step`](super::FullWindowFx::take_step) directly.
//!
//! Tool sessions use it the same way: a draft the app owns while a tool is open (Free Transform,
//! Puppet Warp, Perspective Warp, Crop) and the session of every full-window editor (Liquify,
//! Select and Mask, Content-Aware Fill, Filter Gallery, Wide Angle, Vanishing Point) keep a
//! `LocalHistory` of their draft value. Such a session settles after each `apply` with `busy`
//! true while a gesture runs and during an arrow-key nudge run (the nudge pause), so a drag or a
//! run of nudges is one step.
//!
//! The undo keys go to whoever owns the keyboard ([`key_owner::owner`](crate::focus::key_owner::owner)),
//! the same rule for every session:
//! - A session on the app's canvas (Free Transform, Puppet Warp, Perspective Warp, Crop, the Pen
//!   draft) reads no undo key. The app's shortcuts run there, so its keymap runs
//!   Edit › Undo and Redo, and the app routes them to the open session (`undo`, `redo`; the Pen
//!   draft's `remove_last`) before the document.
//! - A dialog or full-window editor is modal: while it is the top modal layer no app shortcut
//!   reaches the app ([`KeyOwner::Dialog`](crate::focus::key_owner::KeyOwner::Dialog),
//!   [`KeyOwner::Blocked`](crate::focus::key_owner::KeyOwner::Blocked)), so it reads Undo and Redo
//!   itself: [`take_step`] for a dialog (not while a text field is editing),
//!   `FullWindowFx::take_step` for a full-window editor (from any focus, while it is the top
//!   modal). A full-window editor also offers `step(Step)` for a native Edit menu, which still
//!   reaches the app while the modal is open.
//!
//! [`step`](LocalHistory::step) restores the whole value, so a session keeps its view state (zoom,
//! pan, the picked part, the current tool) outside the value it records: Undo never moves the
//! view.

use egui::{Context, Key, Modifiers};

/// Most steps a history keeps.
const LIMIT: usize = 100;

/// Undo and redo stacks of a dialog's settings or a tool session's draft `T`, at most 100 steps.
#[derive(Clone, Debug)]
pub struct LocalHistory<T> {
    undo: Vec<T>,
    redo: Vec<T>,
    /// The settings as of the latest step.
    settled: T,
}

/// Which way [`LocalHistory::step`] goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Undo,
    Redo,
}

impl<T: Clone + PartialEq> LocalHistory<T> {
    /// A history whose starting point is `now`.
    pub fn new(now: &T) -> Self {
        Self { undo: Vec::new(), redo: Vec::new(), settled: now.clone() }
    }

    /// Records a change since the latest step as a new step, unless `busy` (a drag or a typing run
    /// is in progress). A new step drops what was undone.
    pub fn settle(&mut self, now: &T, busy: bool) {
        if busy || *now == self.settled {
            return;
        }
        if self.undo.len() >= LIMIT {
            self.undo.remove(0);
        }
        self.undo.push(std::mem::replace(&mut self.settled, now.clone()));
        self.redo.clear();
    }

    /// Steps `now` back ([`Step::Undo`]) or forward ([`Step::Redo`]); an unfinished edit counts as
    /// the latest step. False when there is nowhere to go.
    pub fn step(&mut self, now: &mut T, step: Step) -> bool {
        self.settle(now, false);
        let (from, to) = match step {
            Step::Undo => (&mut self.undo, &mut self.redo),
            Step::Redo => (&mut self.redo, &mut self.undo),
        };
        let Some(next) = from.pop() else { return false };
        to.push(std::mem::replace(now, next.clone()));
        self.settled = next;
        true
    }

    /// The value as of the latest step: what a session compares its draft with to know whether
    /// it has changed since the last [`settle`](Self::settle).
    pub fn settled(&self) -> &T {
        &self.settled
    }

    /// Drops the latest step and everything undone after it, and sets `now` to the value before
    /// that step: an edit the host refused leaves no step and no redo. An unfinished edit counts
    /// as the latest step, as in [`step`](Self::step). False, and nothing changed, when there is
    /// no step to drop.
    pub fn drop_last(&mut self, now: &mut T) -> bool {
        self.settle(now, false);
        let Some(previous) = self.undo.pop() else { return false };
        self.redo.clear();
        *now = previous.clone();
        self.settled = previous;
        true
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

/// Cmd/Ctrl+Z (undo) or Shift+Cmd/Ctrl+Z (redo) pressed this pass, consumed so it never reaches
/// the document. The caller checks first that the dialog is the top modal and no text field is
/// editing (a focused field keeps the keys for its typing). A full-window workspace asks
/// [`FullWindowFx::take_step`](super::FullWindowFx::take_step) instead: it checks the top modal
/// layer itself and reads the keys with any focus, text field included.
pub fn take_step(ctx: &Context) -> Option<Step> {
    if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z)) {
        Some(Step::Redo)
    } else if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Z)) {
        Some(Step::Undo)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_drag_is_one_step_and_a_new_edit_drops_redo() {
        let mut h = LocalHistory::new(&0);
        for v in 1..=5 {
            h.settle(&v, true);
        }
        h.settle(&5, false);
        h.settle(&7, false);
        let mut now = 7;
        assert!(h.step(&mut now, Step::Undo));
        assert_eq!(now, 5);
        assert!(h.step(&mut now, Step::Undo));
        assert_eq!(now, 0, "the whole drag was one step");
        assert!(!h.step(&mut now, Step::Undo));
        assert!(h.step(&mut now, Step::Redo));
        assert_eq!(now, 5);
        now = 9;
        assert!(!h.step(&mut now, Step::Redo), "a new edit drops what was undone");
        assert!(h.step(&mut now, Step::Undo));
        assert_eq!(now, 5, "an unsettled edit is undone first");
    }

    #[test]
    fn drop_last_removes_the_latest_step_and_the_redo_and_reverts_the_value() {
        let mut h = LocalHistory::new(&0);
        h.settle(&1, false);
        h.settle(&2, false);
        let mut now = 2;
        assert!(h.step(&mut now, Step::Undo));
        assert_eq!((now, h.can_redo()), (1, true));
        now = 5;
        assert!(h.drop_last(&mut now), "the unfinished edit is the step that is dropped");
        assert_eq!((now, *h.settled(), h.can_redo()), (1, 1, false));
        assert!(h.drop_last(&mut now));
        assert_eq!((now, h.can_undo()), (0, false));
        assert!(!h.drop_last(&mut now));
        assert_eq!(now, 0);
    }

    #[test]
    fn the_history_keeps_at_most_100_steps() {
        let mut h = LocalHistory::new(&0);
        for v in 1..=150 {
            h.settle(&v, false);
        }
        let mut now = 150;
        let mut steps = 0;
        while h.step(&mut now, Step::Undo) {
            steps += 1;
        }
        assert_eq!((steps, now), (100, 50));
    }
}
