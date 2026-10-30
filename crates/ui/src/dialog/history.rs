//! [`LocalHistory`]: undo and redo inside a dialog, so Cmd/Ctrl+Z steps the open dialog's own
//! settings instead of reaching the document beneath it.
//!
//! A dialog keeps a [`LocalHistory`] of its settings and calls [`settle`](LocalHistory::settle)
//! every pass. A change becomes one step once no edit is in progress, so a slider drag is one
//! step rather than one per frame. [`Dialog::history`](super::Dialog::history) does this for a
//! dialog; workspaces with their own stacks (Liquify, Filter Gallery, Wide Angle, Camera Raw) use
//! `LocalHistory` and [`take_step`] directly.

use egui::{Context, Key, Modifiers};

/// Most steps a history keeps.
const LIMIT: usize = 100;

/// Undo and redo stacks of a dialog's settings `T`.
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

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

/// Cmd/Ctrl+Z (undo) or Shift+Cmd/Ctrl+Z (redo) pressed this pass, consumed so it never reaches
/// the document. The caller checks first that the dialog is the top modal and no text field is
/// editing (a focused field keeps the keys for its typing).
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
