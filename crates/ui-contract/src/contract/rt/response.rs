//! The bridge half of `contract::Response` (docs/contract-rt.md 15.3): the input facts of a run and
//! the build read log that makes a scope reading one interaction fact run again when that fact
//! flips.

use std::cell::RefCell;

use craft_ui_rt::{BuildCx, InteractFact, NodeId, Pos2, Vec2};

/// The input facts of one run, from the queue it consumed.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct InputFacts {
    pub(crate) clicked: bool,
    pub(crate) double_clicked: bool,
    pub(crate) secondary_clicked: bool,
    pub(crate) long_pressed: bool,
    pub(crate) drag_started: bool,
    pub(crate) drag_stopped: bool,
    pub(crate) drag_delta: Vec2,
    /// The last pointer position of the press, in window coordinates.
    pub(crate) pointer: Option<Pos2>,
    /// Key focus moved onto the node since its last run.
    pub(crate) gained_focus: bool,
    /// Key focus left the node since its last run.
    pub(crate) lost_focus: bool,
}

/// Logs a read of `fact` on `node` for the scope whose build runs.
pub(crate) fn log_read(node: NodeId, fact: InteractFact) {
    READS.with(|log| log.borrow_mut().push((node, fact)));
}

thread_local! {
    /// Interaction reads of the scope builds running on this thread, innermost scope last.
    static READS: RefCell<Vec<(NodeId, InteractFact)>> = const { RefCell::new(Vec::new()) };
}

/// The start of one scope's reads in the log.
pub(crate) struct ReadMark(usize);

/// Opens a scope's reads: the window root clears what a panicked build left; nested scopes stack.
pub(crate) fn begin_reads(root: bool) -> ReadMark {
    READS.with(|log| {
        let mut log = log.borrow_mut();
        if root {
            log.clear();
        }
        ReadMark(log.len())
    })
}

/// Closes a scope's reads: each fact read since `mark` becomes a build read of the scope's node
/// (`cx`), so the scope runs again when one of them flips.
pub(crate) fn end_reads(mark: ReadMark, cx: &mut BuildCx<'_>) {
    let reads: Vec<(NodeId, InteractFact)> = READS.with(|log| log.borrow_mut().split_off(mark.0));
    for (i, &(node, fact)) in reads.iter().enumerate() {
        if !reads[..i].contains(&(node, fact)) {
            cx.read_interaction(node, fact);
        }
    }
}
