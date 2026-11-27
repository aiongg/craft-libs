//! The input events nodes receive, and the context they handle them in.
//!
//! Positions in events are window coordinates (logical points); [`EventCx::to_local`] maps them
//! into the handling node's space. Pointer, wheel, key and accessibility events bubble from the
//! node they reach first up through its ancestors until one returns [`Handled::Yes`]. The node
//! that handles a press's `Down` keeps every later phase of that press (pointer capture).

use std::ops::Range;

use craft_behavior::{PointerButton, PointerKind, PressCancel};
use craft_keymap::chord::{Key, Mods};
use emath::{Pos2, Rect, Vec2};

use crate::hit::{Hit, window_origin};
use crate::id::{NodeId, WindowId};
use crate::input::ClipboardRequest;
use crate::layer::DismissReason;
use crate::platform::WindowCommand;
use crate::runtime::Runtime;
use crate::state::{Access, Reads, Store, Writes};
use crate::window::Env;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handled {
    Yes,
    No,
}

/// A press's recognised phase. Slop is in logical points and larger for touch, so a tap with a
/// drifting finger stays a tap.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Gesture {
    /// The button or contact went down. `clicks` is 2 for the second press of a double click.
    Down {
        clicks: u32,
    },
    /// A touch or pen press held still for the long-press time. The press goes on: a move past
    /// slop now starts a drag with `long_pressed`, and the release is no tap.
    LongPress,
    /// The press moved past slop. `origin` is where it went down.
    DragStart {
        origin: Pos2,
        long_pressed: bool,
    },
    Drag {
        delta: Vec2,
    },
    /// The button or contact came up. `click` is a tap or click: released within slop, not
    /// after a drag or a long press.
    Up {
        click: bool,
        clicks: u32,
    },
    Cancel(PressCancel),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerInput {
    pub id: u64,
    pub kind: PointerKind,
    pub button: PointerButton,
    pub mods: Mods,
    pub gesture: Gesture,
    /// Window coordinates.
    pub pos: Pos2,
    /// The shape the press went down on.
    pub hit: Option<Hit>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WheelInput {
    pub pos: Pos2,
    /// Logical points.
    pub delta: Vec2,
    pub mods: Mods,
    pub hit: Option<Hit>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyInput {
    pub key: Key,
    pub mods: Mods,
    pub down: bool,
    pub repeat: bool,
}

/// An assistive technology's request, routed to the node it names. `sub` is set when it names
/// one of the node's sub-nodes ([`crate::AccessCx::sub`]).
#[derive(Clone, Debug, PartialEq)]
pub struct AccessInput {
    pub action: accesskit::Action,
    pub data: Option<accesskit::ActionData>,
    pub target: NodeId,
    pub sub: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Pointer(PointerInput),
    Wheel(WheelInput),
    Key(KeyInput),
    /// Committed typing outside an IME composition. Goes to the focused node only.
    Text(String),
    ImePreedit {
        text: String,
        cursor: Option<Range<usize>>,
    },
    ImeCommit(String),
    Paste(String),
    Access(AccessInput),
    /// The node gained (`true`) or lost key focus. Not bubbled.
    Focus(bool),
    /// The node, a layer child, is asked to go ([`crate::OutsidePress`]); it tells its owner
    /// through its output. Not bubbled.
    Dismiss(DismissReason),
}

/// The context of one node's event handler.
pub struct EventCx<'a> {
    pub(crate) rt: &'a mut Runtime,
    pub(crate) node: NodeId,
    pub(crate) target: NodeId,
}

impl Reads for EventCx<'_> {
    fn access(&mut self) -> Access<'_> {
        self.rt.store.access()
    }
}

impl Writes for EventCx<'_> {
    fn store_mut(&mut self) -> &mut Store {
        &mut self.rt.store
    }
}

impl EventCx<'_> {
    /// The node handling the event.
    pub fn node(&self) -> NodeId {
        self.node
    }

    /// The node the event reached first; differs from [`EventCx::node`] while it bubbles.
    pub fn target(&self) -> NodeId {
        self.target
    }

    pub fn env(&self) -> Env {
        self.rt.env_of(self.node)
    }

    pub fn window(&self) -> WindowId {
        self.env().window
    }

    /// The input time of the step, in seconds.
    pub fn time(&self) -> f64 {
        self.rt.time
    }

    /// The node's rect in its own coordinates.
    pub fn bounds(&self) -> Rect {
        self.rt.tree.nodes.get(&self.node).map_or(Rect::NOTHING, |n| Rect::from_min_size(Pos2::ZERO, n.frame.size))
    }

    /// A window position in this node's coordinates.
    pub fn to_local(&self, window: Pos2) -> Pos2 {
        window - window_origin(&self.rt.tree, self.node)
    }

    /// Any node's rect in window coordinates (an anchor).
    pub fn window_rect(&self, node: NodeId) -> Option<Rect> {
        self.rt.window_rect(node)
    }

    /// Asks the platform to change this node's window (a custom title bar's drag, close button).
    pub fn window_command(&mut self, cmd: WindowCommand) {
        let window = self.window();
        self.rt.window_command(window, cmd);
    }

    /// The tag ([`crate::Element::tag`]) of the nearest node from `from` up to, but not
    /// including, the handling node that has a non-zero tag. A list reads which row a bubbled
    /// event came from this way.
    pub fn tag_below(&self, from: NodeId) -> Option<(NodeId, u64)> {
        let mut cur = Some(from);
        while let Some(id) = cur {
            if id == self.node {
                return None;
            }
            let n = self.rt.tree.nodes.get(&id)?;
            let tag = n.element.as_ref().map_or(0, |e| e.tag());
            if tag != 0 {
                return Some((id, tag));
            }
            cur = n.parent;
        }
        None
    }

    /// This node's output for the parent's next build, created on first use. `O` must be the
    /// element's `Output` type; the parent's `show` returns it.
    pub fn output<O: Default + 'static>(&mut self) -> &mut O {
        if let Some(owner) = self.rt.tree.owner(self.node) {
            self.rt.tree.mark_build(owner);
        }
        let node = self.rt.tree.nodes.get_mut(&self.node).expect("the handling node exists");
        if !node.outbox.as_ref().is_some_and(|b| b.is::<O>()) {
            node.outbox = Some(Box::new(O::default()));
        }
        node.outbox.as_mut().and_then(|b| b.downcast_mut::<O>()).expect("outbox holds O")
    }

    /// For changes to the node's `Local` that no cell tracks.
    pub fn request_build(&mut self) {
        self.rt.tree.mark_build(self.node);
    }

    pub fn request_layout(&mut self) {
        self.rt.tree.mark_layout(self.node);
    }

    pub fn request_paint(&mut self) {
        self.rt.tree.mark_paint(self.node);
    }

    /// The node's access node is rebuilt in the next frame (its value changed in `Local`).
    pub fn request_access(&mut self) {
        let node = self.node;
        self.rt.access_dirty(node);
    }

    pub fn has_focus(&self) -> bool {
        self.focused() == Some(self.node)
    }

    /// The node with key focus in this node's window.
    pub fn focused(&self) -> Option<NodeId> {
        self.rt.focused(self.window())
    }

    pub fn focus(&mut self) {
        let node = self.node;
        self.focus_node(node);
    }

    /// Moves focus to `node` in its window.
    pub fn focus_node(&mut self, node: NodeId) {
        if let Some(window) = self.rt.window_of(node) {
            self.rt.set_focus(window, Some(node));
        }
    }

    pub fn blur(&mut self) {
        if self.has_focus() {
            let window = self.window();
            self.rt.set_focus(window, None);
        }
    }

    pub fn set_clipboard(&mut self, text: impl Into<String>) {
        let window = self.window();
        self.rt.clipboard.push((window, ClipboardRequest::Set(text.into())));
    }

    /// The text arrives as [`Event::Paste`] at the focused node of this node's window.
    pub fn request_paste(&mut self) {
        let window = self.window();
        self.rt.clipboard.push((window, ClipboardRequest::Read));
    }
}
