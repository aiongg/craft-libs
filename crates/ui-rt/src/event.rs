//! The input events nodes receive, and the context they handle them in.
//!
//! Positions in events are window coordinates (logical points); [`EventCx::to_local`] maps them
//! into the handling node's space. Pointer, wheel, key and accessibility events bubble from the
//! node they reach first up through its ancestors until one returns [`Handled::Yes`]. The node
//! that handles a press's `Down` keeps every later phase of that press (pointer capture).

use std::ops::Range;

use craft_behavior::{CopyKeys, DropOperation, PointerButton, PointerKind, PointerSample, PressCancel, WheelPhase, WheelSource, WheelUnit};
use craft_keymap::chord::{Key, Mods};
use emath::{Pos2, Rect, Vec2};

use crate::drag::{DragInfo, DragOutcome, DragPointer, DragStart, DropInput};
use crate::element::Element;
use crate::hit::{Hit, window_origin};
use crate::id::{NodeId, WindowId};
use crate::input::ClipboardRequest;
use crate::layer::DismissReason;
use crate::platform::{WindowCommand, WindowSpec};
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
    /// The pen's reading (or a touch contact's force) for this sample.
    pub sample: Option<PointerSample>,
}

/// A wheel step, a touchpad pinch, or a two-finger touch gesture. A node takes the scroll, the
/// zoom or both; which modifiers zoom and which Ctrl+wheel steps are a pinch is decided before it
/// arrives ([`crate::WheelOptions`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WheelInput {
    pub pos: Pos2,
    /// Scroll in `unit` (logical points for a touch pan); zero when the step zooms.
    pub delta: Vec2,
    pub unit: WheelUnit,
    pub phase: WheelPhase,
    pub source: WheelSource,
    /// The step's zoom factor, 1 for none: a touchpad pinch, a Ctrl+wheel pinch, a wheel step
    /// under the zoom modifiers, or two fingers moving apart.
    pub zoom: f32,
    /// Radians, clockwise on screen.
    pub rotation: f32,
    /// Two touch contacts drive it: `zoom`, `rotation` and `delta` (their centre's movement) come
    /// from the fingers, and every step of the gesture goes to the node that took its start.
    pub touch: bool,
    pub mods: Mods,
    pub hit: Option<Hit>,
}

impl WheelInput {
    /// Whether the step zooms or rotates rather than only scrolling.
    pub fn pinches(&self) -> bool {
        self.zoom != 1.0 || self.rotation != 0.0
    }
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
#[non_exhaustive]
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
    /// A drag is over one of the node's drop shapes, left it, or dropped on it. `Over` bubbles
    /// from the shape's node until a node accepts ([`EventCx::accept_drop`] or [`Handled::Yes`]);
    /// that node then gets `Leave` or `Drop`, not bubbled.
    Drop(DropInput),
    /// The drag this node started ([`EventCx::start_drag`]) ended. Not bubbled.
    DragEnd(DragOutcome),
    /// The window this node asked for ([`EventCx::open_window`]) is open. Not bubbled.
    WindowOpened(WindowId),
    /// The pointer entered (`true`) or left the node or the nodes under it. Only nodes that asked
    /// ([`crate::BuildCx::hover_events`]) get it. Not bubbled.
    Hover(bool),
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

    /// Asks the platform to change another window (a floating panel window that follows a drag).
    pub fn window_command_to(&mut self, window: WindowId, cmd: WindowCommand) {
        self.rt.window_command(window, cmd);
    }

    /// Asks for a new window showing `root`. The driver opens it after this step's events and
    /// this node then gets [`Event::WindowOpened`] with its id.
    pub fn open_window<E: Element>(&mut self, spec: WindowSpec, root: E) {
        let node = self.node;
        self.rt.queue_window(node, spec, root);
    }

    /// A position in this node's window, on the virtual desktop in physical pixels; `None` while
    /// the platform has not told where the window is.
    pub fn to_desktop(&self, window_pos: Pos2) -> Option<Pos2> {
        self.rt.to_desktop(self.window(), window_pos)
    }

    /// Starts a drag session from the press this node owns, carrying `start`'s payload until a
    /// drop target takes it, the press ends over nothing that takes it, or Escape aborts it. The
    /// press goes on as before: this node keeps getting its phases. Returns false (and starts
    /// nothing) when no press of this node's window runs or a drag already runs.
    pub fn start_drag(&mut self, start: DragStart) -> bool {
        let node = self.node;
        self.rt.start_drag(node, start)
    }

    /// The running drag, without recording a read (build and paint read `Env::drag`).
    pub fn drag(&self) -> Option<DragInfo> {
        self.rt.drag_info()
    }

    /// Where the running drag's pointer is.
    pub fn drag_pointer(&self) -> Option<DragPointer> {
        self.rt.drag_pointer()
    }

    /// Accepts the drag this [`Event::Drop`] `Over` offers, as `op` (the cursor shows it). Returns
    /// [`Handled::Yes`] for the handler to return.
    pub fn accept_drop(&mut self, op: DropOperation) -> Handled {
        let node = self.node;
        self.rt.accept_drop(node, op);
        Handled::Yes
    }

    /// The drop operation the running drag's held keys pick over a target that copies with `copy`.
    pub fn drop_operation(&self, copy: CopyKeys) -> DropOperation {
        self.rt.drop_operation(copy)
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

    /// The node's owner builds in the next frame: a layer child tells the node that showed it
    /// that it was dismissed.
    pub fn request_owner_build(&mut self) {
        if let Some(owner) = self.rt.tree.owner(self.node) {
            self.rt.tree.mark_build(owner);
        }
    }

    /// The node builds at frame time `at`, and no frame runs for it before then; it replaces an
    /// earlier build wake of the node. `None` cancels it.
    pub fn request_build_at(&mut self, at: Option<f64>) {
        let node = self.node;
        self.rt.set_build_wake(node, at);
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
