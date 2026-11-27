//! What `show` returns on craft-ui-rt (docs/contract-rt.md section 4): `Response` with the egui
//! method names, `EditResponse`, and the build read log that makes a scope reading interaction state
//! run again when that state flips.
//!
//! A response's input facts (`clicked`, `drag_delta`, ...) come from the input the node's run
//! consumed, so they hold only in the frame of that run; a kept node's response has none. Its
//! interaction facts (`hovered`, `is_pointer_button_down_on`, `has_focus`) are the node's state at
//! the `show`, and reading one logs the read for the enclosing scope.

use std::cell::RefCell;
use std::sync::Arc;

use craft_behavior::{EditEvents, Mixed, Value};
use craft_ui_rt::{BuildCx, Interact, InteractFact, NodeId, Pos2, Rect, Vec2};
use egui::Id;

use crate::contract::NoValue;

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
}

/// A shown component's response: its rect, the input its run consumed and its interaction state.
#[derive(Clone, Debug, PartialEq)]
pub struct Response {
    /// The component's craft `Id`.
    pub id: Id,
    /// The component's rect in this frame, in window coordinates.
    pub rect: Rect,
    /// The node behind the response; `None` in a measuring run, which reaches no node.
    node: Option<NodeId>,
    interact: Interact,
    /// A drag of the node's press runs (kept across frames until the press ends).
    dragging: bool,
    facts: InputFacts,
}

impl Response {
    pub(crate) fn new(id: Id, rect: Rect, node: Option<NodeId>, interact: Interact, dragging: bool, facts: InputFacts) -> Response {
        Response { id, rect, node, interact, dragging, facts }
    }

    /// A primary click or an assistive technology's click reached the component.
    pub fn clicked(&self) -> bool {
        self.facts.clicked
    }

    /// The second click of a double click.
    pub fn double_clicked(&self) -> bool {
        self.facts.double_clicked
    }

    /// A secondary-button click.
    pub fn secondary_clicked(&self) -> bool {
        self.facts.secondary_clicked
    }

    /// A touch or pen press held still for the long-press time.
    pub fn long_pressed(&self) -> bool {
        self.facts.long_pressed
    }

    /// The press moved past slop in this frame.
    pub fn drag_started(&self) -> bool {
        self.facts.drag_started
    }

    /// A drag of the component's press runs.
    pub fn dragged(&self) -> bool {
        self.dragging || self.facts.drag_started
    }

    /// The drag ended in this frame (a release or a cancel after it started).
    pub fn drag_stopped(&self) -> bool {
        self.facts.drag_stopped
    }

    /// How far the drag moved in this frame.
    pub fn drag_delta(&self) -> Vec2 {
        self.facts.drag_delta
    }

    /// Where the pointer of the component's press is, in window coordinates.
    pub fn interact_pointer_pos(&self) -> Option<Pos2> {
        self.facts.pointer
    }

    /// The pointer is over the component. The enclosing scope runs again when this flips.
    pub fn hovered(&self) -> bool {
        self.read(InteractFact::Hovered)
    }

    /// The pointer is over the component, whatever holds the press.
    pub fn contains_pointer(&self) -> bool {
        self.read(InteractFact::Hovered)
    }

    /// A press went down on the component and is held. The enclosing scope runs again when this flips.
    pub fn is_pointer_button_down_on(&self) -> bool {
        self.read(InteractFact::Pressed)
    }

    /// The component has key focus. The enclosing scope runs again when this flips.
    pub fn has_focus(&self) -> bool {
        self.read(InteractFact::Focused)
    }

    fn read(&self, fact: InteractFact) -> bool {
        if let Some(node) = self.node {
            READS.with(|log| log.borrow_mut().push((node, fact)));
        }
        fact.of(self.interact)
    }
}

thread_local! {
    /// Interaction reads of the scope builds running on this thread, innermost scope last.
    static READS: RefCell<Vec<(NodeId, InteractFact)>> = const { RefCell::new(Vec::new()) };
}

/// The start of one scope's reads in the log.
pub(crate) struct ReadMark(usize, usize);

/// Opens a scope's reads: the window root clears what a panicked build left; nested scopes stack.
pub(crate) fn begin_reads(root: bool) -> ReadMark {
    READS.with(|log| {
        let mut log = log.borrow_mut();
        if root {
            log.clear();
        }
        ReadMark(log.len(), crate::contract::context::begin_cell_reads(root))
    })
}

/// Closes a scope's reads: each fact read since `mark` becomes a build read of the scope's node
/// (`cx`), so the scope runs again when one of them flips.
pub(crate) fn end_reads(mark: ReadMark, cx: &mut BuildCx<'_>) {
    crate::contract::context::end_cell_reads(mark.1, cx);
    let reads: Vec<(NodeId, InteractFact)> = READS.with(|log| log.borrow_mut().split_off(mark.0));
    for (i, &(node, fact)) in reads.iter().enumerate() {
        if !reads[..i].contains(&(node, fact)) {
            cx.read_interaction(node, fact);
        }
    }
}

/// What `show` returns: the component's response and the type of value it edits.
pub trait Outcome {
    /// The value type; `NoValue` for a component with none.
    type Value: Value;

    /// The component's response.
    fn response(&self) -> &Response;
}

impl Outcome for Response {
    type Value = NoValue;

    fn response(&self) -> &Response {
        self
    }
}

/// The output of a component that edits a value. Only `cx.edit_response()` builds it.
#[derive(Debug)]
#[non_exhaustive]
pub struct EditResponse<T> {
    /// The component's response.
    pub response: Response,
    /// The run's events in order; empty when nothing was edited (and for a kept node).
    pub events: EditEvents<T>,
    /// The value the component showed after the run, with its edits applied.
    pub shown: Option<Mixed<Arc<T>>>,
}

impl<T> EditResponse<T> {
    /// The latest value the events carry: a proposal, a commit or a set.
    pub fn value(&self) -> Option<&T> {
        self.events.value()
    }
}

impl<T: Value> Outcome for EditResponse<T> {
    type Value = T;

    fn response(&self) -> &Response {
        &self.response
    }
}
