//! Input routing: hit testing, the press lifecycle with gesture recognition, hover, key focus and
//! key bubbling, IME sessions and accessibility actions, each per window.
//!
//! One press per window at a time runs through craft-behavior's [`PressMachine`], stepped once per
//! pointer event (every pen sample is its own event, with its reading). A second touch contact
//! turns the press into a pinch ([`crate::gesture`]); other pointers that go down during a press
//! press nothing. Its press winner is the topmost shape that takes the press; the node that handles the
//! `Down` (the shape's node or an ancestor) keeps every later phase. Hover, pressed and focus
//! state live in per-node cells that only paint reads, so a hover change repaints the nodes whose
//! state changed and nothing else. No behaviour depends on hover: touch delivers none.

use std::collections::VecDeque;

use craft_behavior::{
    PassInput, PointerButton, PointerEvent as PressEvent, PointerKind, PressCancel, PressMachine, PressPhase, PressPick, Sense, count_clicks,
};
use craft_keymap::chord::{Key, Mods};
use emath::{Pos2, Rect, Vec2};

use crate::cursor::{PointerShown, ShapeCursor};
use crate::event::{AccessInput, Event, EventCx, FocusCause, Gesture, Handled, KeyInput, PointerInput, WheelInput};
use crate::gesture::Touches;
use crate::hit::{Hit, HitKind, chain, contains, hit_records, hit_test, hit_test_through, window_origin};
use crate::id::{NodeId, WindowId};
use crate::layer::{DismissReason, Layer, OutsidePress};
use crate::platform::{Cursor, ImePurpose, ImeRequest, PointerEvent, PointerPhase, WindowEvent};
use crate::runtime::Runtime;
use crate::state::State;

/// How long a touch or pen press must stay within slop to be a long press, in seconds.
pub const LONG_PRESS_TIME: f64 = 0.5;

/// The least slop of a touch press, in logical points: a finger drifts further than a mouse.
pub const TOUCH_SLOP: f32 = 8.0;

/// A node's interaction state, as paint reads it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Interact {
    /// A mouse or pen pointer is over the node or one of its descendants, and no drag runs.
    pub hovered: bool,
    /// The node is on the path from the pressed shape's node up to the node that took the press.
    pub pressed: bool,
    pub focused: bool,
    /// The node has key focus and shows it: keyboard use since the last pointer press.
    pub focus_visible: bool,
    /// The node or one of its descendants has key focus.
    pub focus_within: bool,
}

/// One fact of [`Interact`], for a build that reads only that fact: the reader reruns when the
/// fact flips, not when another fact of the node changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InteractFact {
    Hovered,
    Pressed,
    Focused,
    FocusVisible,
    FocusWithin,
}

impl InteractFact {
    pub(crate) const COUNT: usize = 5;

    /// The fact's value in `state`.
    pub fn of(self, state: Interact) -> bool {
        match self {
            InteractFact::Hovered => state.hovered,
            InteractFact::Pressed => state.pressed,
            InteractFact::Focused => state.focused,
            InteractFact::FocusVisible => state.focus_visible,
            InteractFact::FocusWithin => state.focus_within,
        }
    }

    fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ClipboardRequest {
    Set(String),
    Read,
}

/// The open press: which pointer, and who took it.
#[derive(Clone, Copy, Debug)]
struct Press {
    pointer: u64,
    kind: PointerKind,
    button: PointerButton,
    mods: Mods,
    owner: Option<NodeId>,
    /// The pointer's last position, for drag deltas.
    pos: Pos2,
    long_pressed: bool,
}

/// One window's input state: its press, hover, key focus and IME session.
#[derive(Default)]
pub(crate) struct InputState {
    machine: PressMachine<Hit>,
    press: Option<Press>,
    last_press: Option<(Pos2, f64, u32, Mods)>,
    /// A mouse or pen pointer inside the surface, and which.
    hover_pos: Option<Pos2>,
    hover_pointer: Option<u64>,
    /// The shape under the hovering pointer, with the cursor it names.
    hover_hit: Option<(Hit, Option<ShapeCursor>)>,
    /// The shape the open press went down on, with the cursor it names.
    press_hit: Option<(Hit, Option<ShapeCursor>)>,
    hovered: Vec<NodeId>,
    pressed: Vec<NodeId>,
    pub(crate) focused: Option<NodeId>,
    /// The focused node and its ancestors when focus moved there: where focus falls back to when
    /// the node goes.
    focus_chain: Vec<NodeId>,
    /// Focus events, delivered after the handler that caused them returns.
    queue: VecDeque<(NodeId, Event)>,
    ime: ImeSession,
    /// Focus moved since the last frame: the access focus and the IME session follow.
    pub(crate) focus_moved: bool,
    /// Modality, as craft keeps it: a key press or typed text shows the focus ring, a pointer
    /// press or an accessibility click hides it.
    focus_shown: bool,
    /// The last focus move came from a pointer press on the focused node (`Runtime::focus_pressed`).
    focus_pressed: bool,
    /// The handler of the last key press asked that the text the platform sends with it not type
    /// (`EventCx::withhold_text`): the next text event of the window is dropped.
    withhold_text: bool,
    /// The open press holds the focused node's draft (`Event::PressKept`): it went down on a
    /// [`HitKind::Hold`](crate::hit::HitKind::Hold) shape or in a layer child above the focus.
    press_holds: bool,
    /// The modifiers of the window's last modifier change (`WindowEvent::Modifiers`).
    mods: Mods,
    /// Alt went down alone and nothing has happened since: its release is a lone Alt.
    alt_armed: bool,
    /// The last Space press went to the node a modal lets input through to ([`crate::ModalReach`]): its release goes
    /// there too.
    space_to_reach: Option<NodeId>,
    /// The node that takes some of the window's keys ahead of the focused node while it holds a
    /// mode (`BuildCx::capture_keys`), and those keys: a key it does not handle goes on to focus.
    key_capture: Option<(NodeId, smallvec::SmallVec<[Key; 2]>)>,
    pub(crate) touches: Touches,
    /// The cursor last sent to the platform.
    pub(crate) cursor: Cursor,
    /// Something that decides the cursor changed since it was last picked.
    pub(crate) cursor_check: bool,
    /// The painted pointer the window shows.
    pub(crate) painted: Option<PointerShown>,
}

/// A window's platform IME session as the runtime drives it.
#[derive(Default)]
struct ImeSession {
    /// The node the session is for and the cursor rect last sent.
    at: Option<(NodeId, Rect)>,
    /// The surrounding-text revision last sent for the session's node.
    surrounding: Option<u64>,
    /// The platform composes: a non-empty preedit arrived, and no commit or empty preedit since.
    composing: bool,
    /// A composition ended here (a reset, or the session left its node while composing): its late
    /// preedits and commits are dropped until the platform reports a new session.
    fenced: bool,
    /// The session's node asked for its composition to end ([`crate::BuildCx::interrupt_ime`]).
    interrupt: bool,
}

impl InputState {
    /// The open press's pointer, last position and modifiers.
    pub(crate) fn press_state(&self) -> Option<(u64, Pos2, Mods)> {
        self.press.map(|p| (p.pointer, p.pos, p.mods))
    }

    pub(crate) fn hover_pos(&self) -> Option<Pos2> {
        self.hover_pos
    }

    /// The node the open press went down on, and the cursor its shape names.
    pub(crate) fn press_cursor_source(&self) -> Option<(NodeId, Option<ShapeCursor>)> {
        self.press.and(self.press_hit).map(|(hit, cursor)| (hit.node, cursor))
    }

    pub(crate) fn hover_cursor_source(&self) -> Option<(NodeId, Option<ShapeCursor>)> {
        self.hover_hit.map(|(hit, cursor)| (hit.node, cursor))
    }

    /// Where the mouse or pen pointer is over the window.
    pub(crate) fn pointer_pos(&self) -> Option<Pos2> {
        self.hover_pos
    }

    /// A press runs.
    pub(crate) fn pressing(&self) -> bool {
        self.press.is_some()
    }
}

impl Runtime {
    fn input(&self, window: WindowId) -> Option<&InputState> {
        self.windows.get(&window).map(|w| &w.input)
    }

    fn input_mut(&mut self, window: WindowId) -> Option<&mut InputState> {
        self.windows.get_mut(&window).map(|w| &mut w.input)
    }

    /// Writes `window`'s focused-node cell ([`crate::window::Env::focused`]).
    fn note_focused(&mut self, window: WindowId, node: Option<NodeId>) {
        if let Some(cell) = self.windows.get(&window).map(|w| w.env.focused) {
            cell.set(&mut self.store, node);
        }
    }

    /// Writes the window's modifier and pointer cells from `ev`, so a build or paint that read them
    /// runs again when they change.
    fn note_input(&mut self, window: WindowId, ev: &WindowEvent) {
        let Some(env) = self.windows.get(&window).map(|w| w.env) else { return };
        let mods = match ev {
            WindowEvent::Pointer(p) => Some(p.mods),
            WindowEvent::Wheel { mods, .. } | WindowEvent::Pinch { mods, .. } | WindowEvent::Key { mods, .. } | WindowEvent::Modifiers(mods) => {
                Some(*mods)
            }
            _ => None,
        };
        if let Some(mods) = mods {
            env.modifiers.set_if_changed(&mut self.store, mods);
        }
        let held = match ev {
            WindowEvent::Key { key, down: true, repeat: false, .. } if !self.store.peek(env.held_keys).contains(key) => {
                let mut held = self.store.peek(env.held_keys).clone();
                held.push(*key);
                Some(held)
            }
            WindowEvent::Key { key, down: false, .. } if self.store.peek(env.held_keys).contains(key) => {
                Some(self.store.peek(env.held_keys).iter().copied().filter(|k| k != key).collect())
            }
            WindowEvent::Focus(false) if !self.store.peek(env.held_keys).is_empty() => Some(Vec::new()),
            _ => None,
        };
        if let Some(held) = held {
            env.held_keys.set(&mut self.store, held);
        }
        if matches!(ev, WindowEvent::Wheel { .. } | WindowEvent::Pinch { .. }) {
            env.wheels.update(&mut self.store, |n| *n = n.wrapping_add(1));
        }
        let WindowEvent::Pointer(p) = ev else { return };
        let mut facts = *self.store.peek(env.pointer);
        let touch = p.kind == PointerKind::Touch;
        match p.phase {
            PointerPhase::Move => facts.pos = Some(p.pos),
            PointerPhase::Down => {
                facts.pos = Some(p.pos);
                facts.set_held(p.button, true);
            }
            PointerPhase::Up | PointerPhase::Cancel => {
                facts.set_held(p.button, false);
                facts.pos = if touch { None } else { Some(p.pos) };
            }
            PointerPhase::Leave => facts.pos = None,
        }
        env.pointer.set_if_changed(&mut self.store, facts);
    }

    /// Routes `window`'s pointer, wheel, key, text, IME, paste and accessibility events to its
    /// nodes.
    pub(crate) fn route_input(&mut self, window: WindowId, ev: &WindowEvent) {
        self.note_input(window, ev);
        if let Some(modal) = self.blocked_by(window) {
            if let WindowEvent::Pointer(p) = ev {
                if p.phase == PointerPhase::Down {
                    self.window_command(modal, crate::platform::WindowCommand::Focus);
                }
                if let Some(input) = self.input_mut(window) {
                    input.hover_pos = None;
                    input.hover_pointer = None;
                    // A window that takes no input shows the arrow, not what it showed under the pointer.
                    if input.cursor != crate::platform::Cursor::Default {
                        input.cursor = crate::platform::Cursor::Default;
                        self.cursor_requests.push((window, crate::platform::Cursor::Default));
                    }
                }
                self.refresh_hover(window);
            }
            return;
        }
        let lone_alt = self.track_alt(window, ev);
        match ev {
            WindowEvent::Key { down: true, .. } | WindowEvent::Text(_) => self.show_focus(window, true),
            WindowEvent::Pointer(p) if p.phase == PointerPhase::Down => self.show_focus(window, false),
            WindowEvent::Access(req) if req.action == accesskit::Action::Click => self.show_focus(window, false),
            _ => {}
        }
        match ev {
            WindowEvent::Pointer(p) => match self.drag_redirect(window, p) {
                Some((origin, pos)) => self.route_pointer(origin, PointerEvent { pos, ..*p }),
                None => self.route_pointer(window, *p),
            },
            WindowEvent::Wheel { pos, delta, unit, phase, source, mods } => {
                let input = WheelInput {
                    pos: *pos,
                    delta: *delta,
                    unit: *unit,
                    phase: *phase,
                    source: *source,
                    zoom: 1.0,
                    rotation: 0.0,
                    touch: false,
                    mods: *mods,
                    hit: None,
                };
                self.route_wheel(window, input);
            }
            WindowEvent::Pinch { pos, zoom, rotation, mods } => self.route_touchpad_pinch(window, *pos, *zoom, *rotation, *mods),
            WindowEvent::Files(drag) => self.file_drag(window, drag),
            WindowEvent::Key { key, mods, down, repeat } => self.route_key(window, KeyInput { key: *key, mods: *mods, down: *down, repeat: *repeat }),
            WindowEvent::Text(text) => {
                let withheld = self.input_mut(window).is_some_and(|i| std::mem::take(&mut i.withhold_text));
                if !withheld {
                    self.send_to_focused(window, Event::Text(text.clone()));
                }
            }
            WindowEvent::ImePreedit { text, cursor } => {
                if self.ime_live(window, Some(!text.is_empty())) {
                    self.send_to_focused(window, Event::ImePreedit { text: text.clone(), cursor: cursor.clone() })
                }
            }
            WindowEvent::ImeCommit(text) => {
                if self.ime_live(window, Some(false)) {
                    self.send_to_focused(window, Event::ImeCommit(text.clone()))
                }
            }
            WindowEvent::ImeEnabled => {
                if let Some(w) = self.windows.get_mut(&window) {
                    w.input.ime.fenced = false;
                }
            }
            WindowEvent::Paste(text) => self.send_to_focused(window, Event::Paste(text.clone())),
            WindowEvent::Access(req) => self.route_access(window, req),
            WindowEvent::Modifiers(_) if lone_alt => self.route_lone_alt(window),
            _ => {}
        }
        self.flush_queue(window);
        self.settle_drag();
        let windows: Vec<WindowId> = self.windows.keys().copied().collect();
        for w in windows {
            self.refresh_cursor(w);
        }
    }

    /// Follows Alt alone: a modifier change to Alt and nothing else arms it, any other modifier
    /// change, key, pointer press, wheel turn or pinch disarms it. Returns whether `ev` releases an
    /// armed Alt: a lone Alt.
    fn track_alt(&mut self, window: WindowId, ev: &WindowEvent) -> bool {
        let Some(input) = self.input_mut(window) else { return false };
        match ev {
            WindowEvent::Modifiers(mods) => {
                let alone = Mods { alt: true, ..Mods::NONE };
                let released = input.alt_armed && *mods == Mods::NONE;
                input.alt_armed = *mods == alone && input.mods == Mods::NONE;
                input.mods = *mods;
                released
            }
            WindowEvent::Key { down: true, .. } | WindowEvent::Wheel { .. } | WindowEvent::Pinch { .. } | WindowEvent::Focus(false) => {
                input.alt_armed = false;
                false
            }
            WindowEvent::Pointer(p) if p.phase == PointerPhase::Down => {
                input.alt_armed = false;
                false
            }
            _ => false,
        }
    }

    /// A lone Alt goes where a key goes: the focused node (or the key root) and up its path, then the
    /// window root's key stage.
    fn route_lone_alt(&mut self, window: WindowId) {
        let Some(target) = self.focused(window).or_else(|| self.key_root(window)) else {
            return;
        };
        if self.dispatch(target, &Event::LoneAlt, true).is_some() {
            return;
        }
        if let Some(root) = self.root(window)
            && !chain(&self.tree, target).contains(&root)
        {
            let mut stage = vec![root];
            stage.extend(self.tree.nodes.get(&root).map_or(&[][..], |n| n.children.as_slice()));
            for node in stage {
                if self.deliver(node, target, &Event::LoneAlt) == Handled::Yes {
                    return;
                }
            }
        }
    }

    /// A sample of the running drag's pointer that another window delivered (a platform without
    /// pointer capture), in the drag's own window coordinates.
    fn drag_redirect(&self, window: WindowId, p: &PointerEvent) -> Option<(WindowId, Pos2)> {
        let s = self.drag.as_ref().filter(|s| s.origin != window && s.pointer == Some(p.id))?;
        let desktop = self.to_desktop(window, p.pos)?;
        Some((s.origin, self.desktop_to_window(s.origin, desktop)?))
    }

    pub(crate) fn input_press_kind(&self, window: WindowId) -> Option<PointerKind> {
        self.input(window)?.press.map(|p| p.kind)
    }

    /// The cursor the shape `hit` names at window position `pos`.
    fn shape_cursor(&self, hit: Hit, pos: Pos2) -> Option<ShapeCursor> {
        let node = self.tree.nodes.get(&hit.node)?;
        let local = pos - window_origin(&self.tree, hit.node);
        node.hits
            .iter()
            .rev()
            .find(|h| h.kind == crate::hit::HitKind::Input && h.part == hit.part && h.sense == hit.sense && h.shape.contains(local))
            .and_then(|h| Some(ShapeCursor { cursor: h.cursor?, painted: h.painted }))
    }

    /// Sets the step's input time. The driver calls it before it dispatches the step's events.
    pub fn begin_input(&mut self, time: f64) {
        self.time = time;
    }

    /// Runs the time-driven recogniser (long press) and delivers queued focus events in every
    /// window. The driver calls it after the step's events.
    pub fn tick_input(&mut self) {
        let windows: Vec<WindowId> = self.windows.keys().copied().collect();
        for window in windows {
            if self.long_press_due(window)
                && let Some(press) = self.input_mut(window).and_then(|i| i.press.as_mut())
            {
                press.long_pressed = true;
                let press = *press;
                let target = self.input(window).and_then(|i| i.machine.target());
                if let (Some(owner), Some(hit)) = (press.owner, target) {
                    let ev = Event::Pointer(PointerInput {
                        id: press.pointer,
                        kind: press.kind,
                        button: press.button,
                        mods: press.mods,
                        gesture: Gesture::LongPress,
                        pos: press.pos,
                        hit: Some(hit),
                        sample: None,
                    });
                    self.deliver(owner, hit.node, &ev);
                }
            }
            self.flush_queue(window);
        }
    }

    /// A touch or pen press is held and could still become a long press: the platform must keep
    /// stepping so the recogniser sees the time pass.
    pub fn wants_input_tick(&self) -> bool {
        self.windows.keys().any(|w| self.press_may_long_press(*w))
    }

    fn press_may_long_press(&self, window: WindowId) -> bool {
        self.input(window).is_some_and(|i| i.press.is_some_and(|p| p.kind != PointerKind::Mouse && !p.long_pressed && !i.machine.dragging()))
    }

    fn long_press_due(&self, window: WindowId) -> bool {
        self.press_may_long_press(window) && self.input(window).and_then(|i| i.machine.start()).is_some_and(|s| self.time - s.time >= LONG_PRESS_TIME)
    }

    /// Where focus goes in `window` when the layer child `node` goes with focus inside it
    /// ([`crate::BuildCx::set_focus_return`]), while that node lives.
    pub fn focus_return(&self, window: WindowId, node: NodeId) -> Option<NodeId> {
        self.windows.get(&window)?.focus_returns.get(&node).copied().filter(|to| self.tree.nodes.contains_key(to))
    }

    /// The node with key focus in `window`.
    pub fn focused(&self, window: WindowId) -> Option<NodeId> {
        self.input(window)?.focused
    }

    /// Whether a pointer press gave the focused node of `window` its focus; false after a key,
    /// a request or an assistive technology moved focus there.
    pub fn focus_pressed(&self, window: WindowId) -> bool {
        self.input(window).is_some_and(|i| i.focused.is_some() && i.focus_pressed)
    }

    /// Drops the next text event of `window` before the next key press: the text the platform sends
    /// with a key press a handler took for something else.
    pub(crate) fn withhold_text(&mut self, window: WindowId) {
        if let Some(input) = self.input_mut(window) {
            input.withhold_text = true;
        }
    }

    /// Moves `window`'s key focus for `cause`. The old and the new node get [`Event::Focus`] with
    /// the cause once the current handler returns; paint readers of their interaction state
    /// repaint. A node outside the window is ignored.
    pub fn set_focus(&mut self, window: WindowId, node: Option<NodeId>, cause: FocusCause) {
        let node = node.filter(|id| self.tree.window_of(*id) == Some(window));
        let Some(input) = self.input_mut(window) else { return };
        if node == input.focused {
            return;
        }
        let old_chain = std::mem::take(&mut input.focus_chain);
        let old_focus = input.focused;
        if let Some(old) = old_focus {
            input.queue.push_back((old, Event::Focus { gained: false, cause }));
        }
        input.focused = node;
        input.focus_moved = true;
        input.focus_pressed = false;
        self.note_focused(window, node);
        let new_chain = node.map(|n| chain(&self.tree, n)).unwrap_or_default();
        let Some(input) = self.input_mut(window) else { return };
        if let Some(new) = node {
            input.queue.push_back((new, Event::Focus { gained: true, cause }));
        }
        let mut touched = old_chain;
        touched.extend_from_slice(&new_chain);
        input.focus_chain = new_chain;
        self.refresh_interact(&touched);
        self.dismiss_on_focus_left(window, old_focus, node);
    }

    /// Queues [`DismissReason::FocusLeft`] for each layer child of `window` that asks for it and
    /// held the focus (it or a node it owns) before a move to `new`, which is outside it and not
    /// its anchor.
    fn dismiss_on_focus_left(&mut self, window: WindowId, old: Option<NodeId>, new: Option<NodeId>) {
        let Some(old) = old else { return };
        let Some(w) = self.windows.get(&window) else { return };
        let watching: Vec<(NodeId, Option<NodeId>)> =
            w.layered.iter().filter(|(_, s)| s.focus_left && !s.inert).map(|(id, s)| (*id, s.anchor_node())).collect();
        if watching.is_empty() {
            return;
        }
        let old_chain = self.tree.owner_chain(old);
        let new_chain = new.map(|n| self.tree.owner_chain(n)).unwrap_or_default();
        let left: Vec<NodeId> = watching
            .into_iter()
            .filter(|(id, anchor)| old_chain.contains(id) && !new_chain.contains(id) && anchor.is_none_or(|a| !new_chain.contains(&a)))
            .map(|(id, _)| id)
            .collect();
        let Some(input) = self.input_mut(window) else { return };
        for id in left {
            input.queue.push_back((id, Event::Dismiss(DismissReason::FocusLeft)));
        }
    }

    /// Moves `window`'s focus to the next (or previous) Tab stop in tree order, wrapping around.
    pub fn move_focus(&mut self, window: WindowId, forward: bool) {
        let order = self.focus_order(window);
        if order.is_empty() {
            return;
        }
        let len = order.len();
        let at = self.focused(window).and_then(|f| order.iter().position(|&n| n == f));
        let next = match at {
            Some(i) if forward => (i + 1) % len,
            Some(i) => (i + len - 1) % len,
            None if forward => 0,
            None => len - 1,
        };
        self.set_focus(window, Some(order[next]), FocusCause::Tab { forward });
    }

    /// `window`'s Tab stops in tree order, layers bottom first: every node whose element is
    /// focusable. A list is one stop: its rows are not focusable, and it moves its lead with the
    /// arrow keys it receives.
    pub fn focus_order(&self, window: WindowId) -> Vec<NodeId> {
        let mut out = Vec::new();
        let Some(w) = self.windows.get(&window) else {
            return out;
        };
        let mut stack = vec![w.root];
        while let Some(id) = stack.pop() {
            let Some(n) = self.tree.nodes.get(&id) else {
                continue;
            };
            // A leaving layer child and what it holds take no focus.
            if w.layered.contains_key(&id) && w.inert(&self.tree, id) {
                continue;
            }
            if n.element.as_ref().is_some_and(|e| e.focusable()) {
                out.push(id);
            }
            stack.extend(n.children.iter().rev().copied());
        }
        out
    }

    /// The interaction state of `node`, as of now, without recording a read.
    pub fn interaction(&self, node: NodeId) -> Interact {
        let Some(input) = self.tree.window_of(node).and_then(|w| self.input(w)) else {
            return Interact::default();
        };
        Interact {
            hovered: input.hovered.contains(&node),
            pressed: input.pressed.contains(&node),
            focused: input.focused == Some(node),
            focus_visible: input.focused == Some(node) && input.focus_shown,
            focus_within: input.focus_chain.contains(&node),
        }
    }

    /// The cell paint reads `node`'s interaction state from, created on first use and owned by the
    /// node.
    pub(crate) fn interaction_cell(&mut self, node: NodeId) -> Option<State<Interact>> {
        if let Some(cell) = self.interact.get(&node) {
            return Some(*cell);
        }
        let value = self.interaction(node);
        let n = self.tree.nodes.get_mut(&node)?;
        let cell = self.store.create(value);
        n.owned.push(cell.slot_ref());
        self.interact.insert(node, cell);
        Some(cell)
    }

    /// The cell a build reads one interaction fact of `node` from, created on first use and owned
    /// by the node.
    pub(crate) fn interaction_fact_cell(&mut self, node: NodeId, fact: InteractFact) -> Option<State<bool>> {
        if let Some(cell) = self.interact_facts.get(&node).and_then(|cells| cells[fact.index()]) {
            return Some(cell);
        }
        let value = fact.of(self.interaction(node));
        let n = self.tree.nodes.get_mut(&node)?;
        let cell = self.store.create(value);
        n.owned.push(cell.slot_ref());
        self.interact_facts.entry(node).or_default()[fact.index()] = Some(cell);
        Some(cell)
    }

    /// The cell of the part of `node` the pointer hovers: `Some(part)` while the hover hit is on one of
    /// the node's shapes (0 for its own shape), `None` otherwise.
    pub(crate) fn hovered_part_cell(&mut self, node: NodeId) -> Option<State<Option<u64>>> {
        if let Some(cell) = self.hovered_parts.get(&node) {
            return Some(*cell);
        }
        let value = self.hovered_part(node);
        let n = self.tree.nodes.get_mut(&node)?;
        let cell = self.store.create(value);
        n.owned.push(cell.slot_ref());
        self.hovered_parts.insert(node, cell);
        Some(cell)
    }

    /// The cell of whether the pointer hovers `part` of `node`, created on first use and owned by
    /// the node.
    pub(crate) fn part_hover_cell(&mut self, node: NodeId, part: u64) -> Option<State<bool>> {
        if let Some(&(_, cell)) = self.part_hovers.get(&node).and_then(|cells| cells.iter().find(|(held, _)| *held == part)) {
            return Some(cell);
        }
        let value = self.hovered_part(node) == Some(part);
        let n = self.tree.nodes.get_mut(&node)?;
        let cell = self.store.create(value);
        n.owned.push(cell.slot_ref());
        self.part_hovers.entry(node).or_default().push((part, cell));
        Some(cell)
    }

    /// The part of `node` the hover hit is on (0 for its own shape), as of now, without recording a read.
    pub fn hovered_part(&self, node: NodeId) -> Option<u64> {
        self.windows.keys().find_map(|&w| self.input(w)?.hover_hit.filter(|(h, _)| h.node == node).map(|(h, _)| h.part))
    }

    /// The pointer's place (window coordinates) while it hovers `node` or a node under it.
    pub fn hover_point(&self, node: NodeId) -> Option<Pos2> {
        self.windows.keys().find_map(|&w| self.input(w).filter(|i| i.hovered.contains(&node)).and_then(|i| i.hover_pos))
    }

    /// The cell of `node`'s hover point ([`Runtime::hover_point`]), created on first use and owned by
    /// the node.
    pub(crate) fn hover_point_cell(&mut self, node: NodeId) -> Option<State<Option<Pos2>>> {
        if let Some(cell) = self.hover_points.get(&node) {
            return Some(*cell);
        }
        let value = self.hover_point(node);
        let n = self.tree.nodes.get_mut(&node)?;
        let cell = self.store.create(value);
        n.owned.push(cell.slot_ref());
        self.hover_points.insert(node, cell);
        Some(cell)
    }

    /// Moves the hovered part from the node of `before` to the node of `now`: each node's cell flips
    /// only when its own hovered part changes.
    fn refresh_hovered_parts(&mut self, before: Option<Hit>, now: Option<Hit>) {
        if let Some(b) = before
            && now.is_none_or(|n| n.node != b.node)
            && let Some(cell) = self.hovered_parts.get(&b.node).copied()
        {
            cell.set_if_changed(&mut self.store, None);
        }
        if let Some(n) = now
            && let Some(cell) = self.hovered_parts.get(&n.node).copied()
        {
            cell.set_if_changed(&mut self.store, Some(n.part));
        }
        for node in [before.map(|b| b.node), now.map(|n| n.node).filter(|n| before.is_none_or(|b| b.node != *n))].into_iter().flatten() {
            let Some(cells) = self.part_hovers.get(&node) else { continue };
            let hovered = now.filter(|n| n.node == node).map(|n| n.part);
            for &(part, cell) in cells.clone().iter() {
                cell.set_if_changed(&mut self.store, hovered == Some(part));
            }
        }
    }

    fn show_focus(&mut self, window: WindowId, shown: bool) {
        let Some(input) = self.input_mut(window) else { return };
        if input.focus_shown != shown {
            input.focus_shown = shown;
            let chain = input.focus_chain.clone();
            self.refresh_interact(&chain);
        }
    }

    fn refresh_interact(&mut self, nodes: &[NodeId]) {
        for &id in nodes {
            if let Some(cell) = self.interact.get(&id).copied() {
                let value = self.interaction(id);
                cell.set_if_changed(&mut self.store, value);
            }
            if let Some(cells) = self.interact_facts.get(&id).copied() {
                let value = self.interaction(id);
                for fact in
                    [InteractFact::Hovered, InteractFact::Pressed, InteractFact::Focused, InteractFact::FocusVisible, InteractFact::FocusWithin]
                {
                    if let Some(cell) = cells[fact.index()] {
                        cell.set_if_changed(&mut self.store, fact.of(value));
                    }
                }
            }
        }
    }

    /// The topmost shape under `pos` (window coordinates) in `window` whose sense passes `accept`.
    pub fn hit(&self, window: WindowId, pos: Pos2, accept: &dyn Fn(Sense) -> bool) -> Option<Hit> {
        hit_test(&self.tree, self.windows.get(&window)?, pos, accept)
    }

    /// Recomputes `window`'s hovered path from the pointer's position over the current shapes. The
    /// frame runs it after paint, so content that moved under a still pointer updates its hover.
    pub(crate) fn refresh_hover(&mut self, window: WindowId) {
        let Some(input) = self.input(window) else { return };
        let hit = match input.hover_pos {
            Some(pos) if !input.machine.dragging() => {
                self.hit(window, pos, &|s| s.intersects(Sense::HOVER | Sense::PRESS)).map(|h| (h, self.shape_cursor(h, pos)))
            }
            _ => None,
        };
        let now = hit.map(|(h, _)| chain(&self.tree, h.node)).unwrap_or_default();
        let Some(input) = self.input_mut(window) else { return };
        // Nodes the pointer stays over whose own part under it changed, for `Event::HoverPart`.
        let mut moved: Vec<(NodeId, Option<u64>)> = Vec::new();
        if input.hover_hit != hit {
            let before = std::mem::replace(&mut input.hover_hit, hit).map(|(h, _)| h);
            input.cursor_check = true;
            let now_hit = hit.map(|(h, _)| h);
            let part_of = |h: Option<Hit>, node: NodeId| h.filter(|h| h.node == node).map(|h| h.part);
            for node in before.iter().chain(&now_hit).map(|h| h.node) {
                let stays = input.hovered.contains(&node) && now.contains(&node);
                if stays && part_of(before, node) != part_of(now_hit, node) && !moved.iter().any(|(n, _)| *n == node) {
                    moved.push((node, part_of(now_hit, node)));
                }
            }
            self.refresh_hovered_parts(before, now_hit);
        }
        for (node, part) in moved {
            if self.tree.nodes.get(&node).is_some_and(|n| n.hover_events) {
                self.deliver(node, node, &Event::HoverPart(part));
            }
        }
        if !self.hover_points.is_empty() {
            // Only the nodes whose builds read their hover point have a cell; each follows the
            // pointer while it is over the node or a node under it, and is empty otherwise.
            let pos = self.input(window).and_then(|i| i.hover_pos);
            let cells: Vec<(NodeId, State<Option<Pos2>>)> = self.hover_points.iter().map(|(id, cell)| (*id, *cell)).collect();
            for (id, cell) in cells {
                if self.tree.nodes.get(&id).is_some_and(|n| n.window == window) {
                    cell.set_if_changed(&mut self.store, pos.filter(|_| now.contains(&id)));
                }
            }
        }
        let Some(input) = self.input_mut(window) else { return };
        if now == input.hovered {
            return;
        }
        let old = std::mem::replace(&mut input.hovered, now.clone());
        let wants = |id: &NodeId| self.tree.nodes.get(id).is_some_and(|n| n.hover_events);
        let left: Vec<NodeId> = old.iter().filter(|id| !now.contains(id) && wants(id)).copied().collect();
        let entered: Vec<NodeId> = now.iter().filter(|id| !old.contains(id) && wants(id)).copied().collect();
        let touched: Vec<NodeId> = old.into_iter().chain(now).collect();
        self.refresh_interact(&touched);
        for id in left {
            self.deliver(id, id, &Event::Hover(false));
        }
        for id in entered {
            self.deliver(id, id, &Event::Hover(true));
        }
    }

    fn set_pressed(&mut self, window: WindowId, nodes: Vec<NodeId>) {
        let Some(input) = self.input_mut(window) else { return };
        let mut touched = std::mem::replace(&mut input.pressed, nodes);
        touched.extend_from_slice(&input.pressed);
        self.refresh_interact(&touched);
    }

    fn slop(&self, kind: PointerKind) -> f32 {
        let slop = self.store.peek(self.tokens_cell()).size.hit.slop;
        if kind == PointerKind::Touch { slop.max(TOUCH_SLOP) } else { slop }
    }

    fn owns(&self, window: WindowId, pointer: u64) -> bool {
        self.input(window).is_some_and(|i| i.press.is_some_and(|p| p.pointer == pointer))
    }

    fn route_pointer(&mut self, window: WindowId, p: PointerEvent) {
        if p.kind == PointerKind::Touch && self.route_touch(window, p) {
            return;
        }
        match p.phase {
            PointerPhase::Move => {
                let Some(input) = self.input_mut(window) else { return };
                if p.kind != PointerKind::Touch {
                    input.hover_pos = Some(p.pos);
                    input.hover_pointer = Some(p.id);
                }
                let delta = input.press.as_mut().filter(|q| q.pointer == p.id).map(|press| {
                    let delta = p.pos - press.pos;
                    press.pos = p.pos;
                    delta
                });
                if let Some(delta) = delta {
                    self.step_press(window, &[PressEvent::moved(p.pos, p.sample)], p, delta, None);
                }
                if self.drag_follows(window, p.id) || self.file_drag_in(window) {
                    self.drag_moved(p.pos, p.mods);
                }
                self.refresh_hover(window);
            }
            PointerPhase::Down => {
                let Some(input) = self.input_mut(window) else { return };
                if let Some(press) = input.press {
                    // A second finger is a gesture the runtime does not recognise; other buttons
                    // during a press are not presses.
                    if press.kind == PointerKind::Touch && p.kind == PointerKind::Touch && press.pointer != p.id {
                        self.cancel_press(window, PressCancel::Gesture);
                    }
                    return;
                }
                if p.kind != PointerKind::Touch {
                    input.hover_pos = Some(p.pos);
                    input.hover_pointer = Some(p.id);
                }
                self.dismiss_away(Some(window));
                if self.dismiss_outside(window, p.pos) {
                    self.refresh_hover(window);
                    return;
                }
                let time = self.time;
                let slop = self.slop(p.kind);
                let focused = self.focused(window);
                let holds = p.button == PointerButton::Primary && focused.is_some() && self.press_holds(window, p.pos, focused);
                let Some(input) = self.input_mut(window) else { return };
                input.press_holds = holds;
                let clicks = count_clicks(input.last_press, p.pos, time, p.mods, slop);
                input.last_press = Some((p.pos, time, clicks, p.mods));
                input.press =
                    Some(Press { pointer: p.id, kind: p.kind, button: p.button, mods: p.mods, owner: None, pos: p.pos, long_pressed: false });
                let ev = PressEvent::pressed_by(p.button, p.pos, time, clicks);
                self.step_press(window, &[ev], p, Vec2::ZERO, None);
                if p.button == PointerButton::Primary && self.input(window).is_some_and(|i| i.press_hit.is_none()) {
                    self.unclaimed_press(window, focused);
                }
                self.refresh_hover(window);
            }
            PointerPhase::Up => {
                let Some(press) = self.input(window).and_then(|i| i.press).filter(|q| q.pointer == p.id) else {
                    return;
                };
                if self.drag_follows(window, p.id) {
                    self.settle_drag();
                    self.drag_moved(p.pos, p.mods);
                    self.finish_drag();
                }
                let ev = PressEvent::released_by(p.button, p.pos);
                self.step_press(window, &[ev], p, p.pos - press.pos, None);
                self.refresh_hover(window);
            }
            PointerPhase::Cancel => {
                if self.owns(window, p.id) {
                    self.cancel_press(window, PressCancel::Host);
                }
            }
            PointerPhase::Leave => {
                if let Some(input) = self.input_mut(window)
                    && input.hover_pointer.is_none_or(|h| h == p.id)
                {
                    input.hover_pos = None;
                    input.hover_pointer = None;
                }
                if self.owns(window, p.id) {
                    self.step_press(window, &[PressEvent::Gone], p, Vec2::ZERO, None);
                }
                self.refresh_hover(window);
            }
        }
    }

    /// Sends [`Event::Dismiss`] to every layer child of `window` that asks for it on a press
    /// outside it, topmost first, and returns whether one of them consumes the press. Inside is
    /// the child's rect or the rect of anything it owns on a layer, the shapes of everything it
    /// owns, and its anchor node's rect.
    fn dismiss_outside(&mut self, window: WindowId, pos: Pos2) -> bool {
        let Some(w) = self.windows.get(&window) else { return false };
        let mut layered: Vec<NodeId> = Vec::new();
        for layer in Layer::ALL.into_iter().rev().filter(|l| *l != Layer::Content) {
            if let Some(n) = self.tree.nodes.get(&w.layer(layer)) {
                layered.extend(n.children.iter().rev().copied());
            }
        }
        let watching: Vec<(NodeId, OutsidePress, Option<NodeId>)> = layered
            .iter()
            .filter_map(|id| {
                w.layered.get(id).filter(|s| s.outside_press != OutsidePress::Ignore && !s.inert).map(|s| (*id, s.outside_press, s.anchor_node()))
            })
            .collect();
        if watching.is_empty() {
            return false;
        }
        let hit_chain = self.hit(window, pos, &|_| true).map(|h| self.tree.owner_chain(h.node)).unwrap_or_default();
        let within = |id: NodeId| self.window_rect(id).is_some_and(|r| contains(r, pos));
        let mut dismissed = Vec::new();
        for (id, outside, anchor) in watching {
            let inside = hit_chain.contains(&id)
                || anchor.is_some_and(|a| hit_chain.contains(&a) || within(a))
                || layered.iter().any(|&l| within(l) && self.tree.owner_chain(l).contains(&id));
            if !inside {
                dismissed.push((id, outside));
            }
        }
        let mut consumed = false;
        for (id, outside) in dismissed {
            self.deliver(id, id, &Event::Dismiss(DismissReason::OutsidePress));
            consumed |= outside == OutsidePress::DismissAndConsume;
        }
        consumed
    }

    /// Sends [`DismissReason::Away`] to every layer child that asks for it ([`crate::LayerSpec::away`])
    /// in every window but `pressed` (the window a press landed in; `None` when the app lost key
    /// focus: every window).
    pub(crate) fn dismiss_away(&mut self, pressed: Option<WindowId>) {
        let away: Vec<NodeId> = self
            .windows
            .iter()
            .filter(|(id, _)| Some(**id) != pressed)
            .flat_map(|(_, w)| w.layered.iter().filter(|(_, spec)| spec.away && !spec.inert).map(|(id, _)| *id))
            .collect();
        for id in away {
            self.deliver(id, id, &Event::Dismiss(DismissReason::Away));
        }
    }

    /// A command ran in `window` from the node `from` (`None`: a native menu pick or the palette,
    /// from outside every layer): every layer child of the window that asks for it
    /// ([`crate::LayerSpec::commands`]) and does not own `from` gets [`DismissReason::Command`].
    /// The command board calls it when a key, a menu pick or the palette fires a command.
    pub fn dismiss_on_command(&mut self, window: WindowId, from: Option<NodeId>) {
        let Some(w) = self.windows.get(&window) else { return };
        let chain = from.map(|n| self.tree.owner_chain(n)).unwrap_or_default();
        let closing: Vec<NodeId> =
            w.layered.iter().filter(|(id, spec)| spec.commands && !spec.inert && !chain.contains(id)).map(|(id, _)| *id).collect();
        for id in closing {
            self.deliver(id, id, &Event::Dismiss(DismissReason::Command));
        }
    }

    /// The node a modal in `window` lets input through to ([`crate::ModalReach`]), when the pointer is over it there.
    fn over_modal_reach(&self, window: WindowId) -> Option<NodeId> {
        let reach = self.modal_reach(window)?;
        let pos = self.input(window)?.hover_pos?;
        let hit = hit_test_through(&self.tree, self.windows.get(&window)?, pos, &|_| true, Some(reach.node))?;
        self.tree.owner_chain(hit.node).contains(&reach.node).then_some(reach.node)
    }

    /// What the topmost child of `window`'s modal layer lets through to the content below it.
    pub(crate) fn modal_reach(&self, window: WindowId) -> Option<crate::layer::ModalReach> {
        let w = self.windows.get(&window)?;
        let modal = self.tree.nodes.get(&w.layer(Layer::Modal))?;
        let top = *modal.children.iter().rev().find(|&&c| !w.inert(&self.tree, c))?;
        w.layered.get(&top)?.reach
    }

    /// Ends `window`'s open press with `reason`; its owner gets `Gesture::Cancel`.
    pub fn cancel_press(&mut self, window: WindowId, reason: PressCancel) {
        let Some(press) = self.input(window).and_then(|i| i.press) else {
            return;
        };
        if self.drag_follows(window, press.pointer) {
            self.cancel_drag(reason);
        }
        let p = PointerEvent::new(press.pointer, press.kind, PointerPhase::Cancel, press.button, press.pos, press.mods);
        self.step_press(window, &[], p, Vec2::ZERO, Some(reason));
    }

    fn step_press(&mut self, window: WindowId, events: &[PressEvent], p: PointerEvent, delta: Vec2, cancel: Option<PressCancel>) {
        let held = !matches!(p.phase, PointerPhase::Up | PointerPhase::Cancel);
        let input = PassInput::new(events, self.slop(p.kind), p.pos, held).delta(delta).cancel(cancel);
        let Some(mut machine) = self.input_mut(window).map(|i| std::mem::take(&mut i.machine)) else { return };
        let reach = self.modal_reach(window);
        let Some(win) = self.windows.get(&window) else { return };
        let space = self.store.peek(win.env.held_keys).contains(&Key::Space);
        let tree = &self.tree;
        let step = machine.step(input, |at, button| {
            let accept = |s: Sense| if button == PointerButton::Secondary { s.contains(Sense::CONTEXT_CLICK) } else { s.takes_press() };
            let through = reach.filter(|r| r.admits_press(button, p.mods, space)).map(|r| r.node);
            hit_test_through(tree, win, at, &accept, through).map(|h| PressPick::new(h, h.sense))
        });
        if let Some(i) = self.input_mut(window) {
            i.machine = machine;
        }
        if let (Some(hit), Some(phase)) = (step.target, step.phase) {
            self.press_phase(window, hit, phase, p);
        }
        if let Some(input) = self.input_mut(window)
            && input.machine.target().is_none()
            && input.press.is_some()
        {
            input.press = None;
            input.press_hit = None;
            input.cursor_check = true;
            self.set_pressed(window, Vec::new());
        }
    }

    fn press_phase(&mut self, window: WindowId, hit: Hit, phase: PressPhase, p: PointerEvent) {
        let long_pressed = self.input(window).and_then(|i| i.press).is_some_and(|q| q.long_pressed);
        let gesture = match phase {
            PressPhase::Down { start, .. } => Gesture::Down { clicks: start.clicks },
            PressPhase::DragStart { start, .. } => Gesture::DragStart { origin: start.origin, long_pressed },
            PressPhase::Drag { delta, .. } => Gesture::Drag { delta },
            PressPhase::Release { start, click, .. } => Gesture::Up { click: click && !long_pressed, clicks: start.clicks },
            PressPhase::Cancel { reason, .. } => Gesture::Cancel(reason),
            _ => return,
        };
        let ev = Event::Pointer(PointerInput {
            id: p.id,
            kind: p.kind,
            button: p.button,
            mods: p.mods,
            gesture,
            pos: p.pos,
            hit: Some(hit),
            sample: p.sample,
        });
        if matches!(gesture, Gesture::Down { .. }) {
            let cursor = self.shape_cursor(hit, p.pos);
            if let Some(input) = self.input_mut(window) {
                input.press_hit = Some((hit, cursor));
                input.cursor_check = true;
            }
            let before = self.focused(window);
            let primary = p.button == PointerButton::Primary;
            let holds = self.input_mut(window).is_some_and(|i| std::mem::take(&mut i.press_holds));
            let held = holds && before.is_some_and(|f| self.tree.nodes.get(&f).and_then(|n| n.element.as_ref()).is_some_and(|e| e.holds_drafts()));
            if !hit.sense.contains(Sense::KEEP_FOCUS) && !held {
                let stop = self.nearest_focusable(hit.node);
                self.set_focus(window, stop, FocusCause::Pointer);
                if stop.is_some()
                    && let Some(input) = self.input_mut(window)
                {
                    input.focus_pressed = true;
                }
            }
            if primary {
                self.note_press_kept(window, before, Some(hit.node), holds);
            }
            let owner = self.dispatch(hit.node, &ev, true);
            if let Some(press) = self.input_mut(window).and_then(|i| i.press.as_mut()) {
                press.owner = owner;
            }
            let mut path = chain(&self.tree, hit.node);
            if let Some(owner) = owner
                && let Some(i) = path.iter().position(|&n| n == owner)
            {
                path.truncate(i + 1);
            }
            self.set_pressed(window, path);
        } else if let Some(owner) = self.input(window).and_then(|i| i.press).and_then(|q| q.owner) {
            self.deliver(owner, hit.node, &ev);
        }
    }

    /// A primary press no shape took gives the focus up (a focused text commits through its focus
    /// loss), unless it holds the focused node's draft or the focus lies under a node that keeps it on
    /// such presses ([`crate::BuildCx::keep_focus_on_empty_presses`]: a trapping scope, a canvas).
    fn unclaimed_press(&mut self, window: WindowId, focused: Option<NodeId>) {
        let holds = self.input_mut(window).is_some_and(|i| std::mem::take(&mut i.press_holds));
        let Some(node) = focused else { return };
        let held = holds && self.tree.nodes.get(&node).and_then(|n| n.element.as_ref()).is_some_and(|e| e.holds_drafts());
        let kept = chain(&self.tree, node).iter().any(|id| self.tree.nodes.get(id).is_some_and(|n| n.keeps_focus));
        if held || kept {
            self.note_press_kept(window, focused, None, holds);
        } else {
            self.set_focus(window, None, FocusCause::Pointer);
        }
    }

    /// Whether a primary press at `pos` holds the draft of `focused`: it lands on a
    /// [`HitKind::Hold`] shape, or in a layer child above the one `focused` shows in (a popup opened
    /// over a field).
    fn press_holds(&self, window: WindowId, pos: Pos2, focused: Option<NodeId>) -> bool {
        let Some(w) = self.windows.get(&window) else { return false };
        if hit_records(&self.tree, w, pos, &|h| h.kind == HitKind::Hold).is_some() {
            return true;
        }
        let Some((under, _)) = hit_records(&self.tree, w, pos, &|h| h.kind == HitKind::Input) else { return false };
        match (self.layer_place(window, under.node), focused.map(|f| self.layer_place(window, f))) {
            (Some(at), Some(below)) => at.0 >= Layer::Popup.index() && below.is_none_or(|below| below < at),
            _ => false,
        }
    }

    /// The layer `node` shows in and the place of its layer child among that layer's children, so
    /// that a later place is above an earlier one; `None` on the content layer.
    fn layer_place(&self, window: WindowId, node: NodeId) -> Option<(u8, usize)> {
        let w = self.windows.get(&window)?;
        let path = chain(&self.tree, node);
        Layer::ALL.into_iter().filter(|l| *l != Layer::Content).find_map(|layer| {
            let at = path.iter().position(|&n| n == w.layer(layer))?;
            let child = path.get(at.checked_sub(1)?)?;
            let index = self.tree.nodes.get(&w.layer(layer))?.children.iter().position(|c| c == child)?;
            Some((layer.index(), index))
        })
    }

    /// Sends the node that held the focus before a primary press, `before`, [`Event::PressKept`]
    /// when it still holds it and the press was not its own: `on` (the node the press went down on,
    /// `None` when no shape took it) takes focus for some other stop, or for none.
    fn note_press_kept(&mut self, window: WindowId, before: Option<NodeId>, on: Option<NodeId>, holds: bool) {
        let Some(focused) = before.filter(|f| self.focused(window) == Some(*f)) else { return };
        if on.is_some_and(|on| self.nearest_focusable(on) == Some(focused)) {
            return;
        }
        self.deliver(focused, focused, &Event::PressKept { holds });
    }

    fn route_key(&mut self, window: WindowId, k: KeyInput) {
        self.drag_mods(k.mods);
        if k.down
            && let Some(input) = self.input_mut(window)
        {
            input.withhold_text = false;
        }
        if k.down && k.key == Key::Escape && self.input(window).is_some_and(|i| i.press.is_some()) {
            self.cancel_press(window, PressCancel::Escape);
            return;
        }
        if let Some(node) = self.input(window).and_then(|i| i.key_capture.as_ref()).filter(|(_, keys)| keys.contains(&k.key)).map(|(node, _)| *node)
            && self.deliver(node, node, &Event::Key(k)) == Handled::Yes
        {
            return;
        }
        // Space over the canvas a modal keeps open goes to that canvas, not to the dialog's focused control: held, it
        // turns the canvas's primary press into its pan. Its release follows it there.
        let space = k.key == Key::Space && !(k.mods.cmd || k.mods.ctrl || k.mods.alt);
        let to_reach = match (space, k.down) {
            (false, _) => None,
            (true, true) => {
                let node = self.over_modal_reach(window);
                if let Some(input) = self.input_mut(window) {
                    input.space_to_reach = node;
                }
                node
            }
            (true, false) => self.input_mut(window).and_then(|i| i.space_to_reach.take()),
        };
        let Some(target) = to_reach.or_else(|| self.focused(window)).or_else(|| self.key_root(window)) else {
            return;
        };
        let mut handled = self.dispatch(target, &Event::Key(k), true).is_some();
        // A key nothing on a layer child's path took reaches the app root and its content's top
        // nodes last, as a key in the content does by bubbling: the window's key stage (Tab order,
        // F6) lives there.
        if !handled
            && let Some(root) = self.root(window)
            && !chain(&self.tree, target).contains(&root)
        {
            let mut stage = vec![root];
            stage.extend(self.tree.nodes.get(&root).map_or(&[][..], |n| n.children.as_slice()));
            for node in stage {
                if self.deliver(node, target, &Event::Key(k)) == Handled::Yes {
                    handled = true;
                    break;
                }
            }
        }
        let plain_tab = k.key == Key::Tab && !(k.mods.cmd || k.mods.ctrl || k.mods.alt);
        if !handled && k.down && plain_tab {
            self.move_focus(window, !k.mods.shift);
        }
        if !handled && k.down && k.key == Key::Escape {
            self.dismiss_on_escape(window);
        }
    }

    /// Where a key goes with nothing focused in `window`: the root, or for an inline root
    /// (`Runtime::set_inline_root`) the content its build made, where the window's key stage lives.
    fn key_root(&self, window: WindowId) -> Option<NodeId> {
        let root = self.root(window)?;
        let node = self.tree.nodes.get(&root)?;
        let inline = node.element.as_ref().is_some_and(|e| e.as_any().is::<crate::element::ElementBox<crate::element::InlineRoot>>());
        match node.children.first() {
            Some(&content) if inline => Some(content),
            _ => Some(root),
        }
    }

    /// The layer children of `window`, topmost first.
    fn layered_top_first(&self, window: WindowId) -> Vec<NodeId> {
        let Some(w) = self.windows.get(&window) else { return Vec::new() };
        let mut out = Vec::new();
        for layer in Layer::ALL.into_iter().rev().filter(|l| *l != Layer::Content) {
            if let Some(n) = self.tree.nodes.get(&w.layer(layer)) {
                out.extend(n.children.iter().rev().copied());
            }
        }
        out
    }

    /// Sends [`DismissReason::Escape`] to the topmost layer child of `window` that asks for it.
    fn dismiss_on_escape(&mut self, window: WindowId) {
        let Some(w) = self.windows.get(&window) else { return };
        let top = self.layered_top_first(window).into_iter().find(|id| w.layered.get(id).is_some_and(|s| s.escape && !s.inert));
        if let Some(id) = top {
            self.deliver(id, id, &Event::Dismiss(DismissReason::Escape));
        }
    }

    /// The window whose app-modal layer child blocks `window`'s input: another window with a
    /// child on its modal layer whose spec is [`crate::LayerSpec::app_modal`]. A blocked window
    /// drops pointer, wheel, key, text, IME, file and accessibility input; a press there focuses
    /// the blocking window.
    pub fn blocked_by(&self, window: WindowId) -> Option<WindowId> {
        self.windows.iter().filter(|(id, _)| **id != window).find_map(|(id, w)| {
            let modal = self.tree.nodes.get(&w.layer(Layer::Modal))?;
            modal.children.iter().any(|c| w.layered.get(c).is_some_and(|s| s.app_modal && !s.inert)).then_some(*id)
        })
    }

    /// Whether an input method event in `window` reaches the focused node: not while a fence
    /// drops the late events of an ended composition. `composing` is what the event says of the
    /// composition after it.
    fn ime_live(&mut self, window: WindowId, composing: Option<bool>) -> bool {
        let Some(w) = self.windows.get_mut(&window) else { return false };
        if w.input.ime.fenced {
            return false;
        }
        if let Some(composing) = composing {
            w.input.ime.composing = composing;
        }
        true
    }

    /// The composition of `node`'s IME session ends: the platform drops its marked text and
    /// restarts the session, and late events of the composition are dropped
    /// ([`ImeRequest::Reset`]). Sent at the window's next IME sync; nothing happens when the
    /// session is not `node`'s.
    pub(crate) fn interrupt_ime(&mut self, node: NodeId) {
        let Some(window) = self.tree.window_of(node) else { return };
        let Some(w) = self.windows.get_mut(&window) else { return };
        if w.input.ime.at.is_some_and(|(n, _)| n == node) {
            w.input.ime.interrupt = true;
        }
    }

    fn send_to_focused(&mut self, window: WindowId, ev: Event) {
        if let Some(focused) = self.focused(window) {
            self.dispatch(focused, &ev, false);
        }
    }

    /// Routes an assistive technology's request from `window`'s adapter to the node it names in
    /// that window; a request naming a node of another window is dropped.
    fn route_access(&mut self, window: WindowId, req: &accesskit::ActionRequest) {
        let raw = req.target_node.0;
        let target = if self.tree.window_of(NodeId(raw)) == Some(window) {
            Some((NodeId(raw), None))
        } else {
            self.windows.get(&window).and_then(|w| w.access.subs.get(&raw).map(|&(owner, key)| (owner, Some(key))))
        };
        let Some((target, sub)) = target else {
            return;
        };
        // An owner that stands for a folded control sends the request to the control first.
        let control = sub.is_none().then(|| self.windows.get(&window).and_then(|w| w.access.stands.get(&target).copied())).flatten();
        if let Some(control) = control {
            let ev = Event::Access(AccessInput { action: req.action, data: req.data.clone(), target: control, sub: None });
            if self.dispatch(control, &ev, false).is_some() {
                return;
            }
        }
        let ev = Event::Access(AccessInput { action: req.action, data: req.data.clone(), target, sub });
        let handled = self.dispatch(target, &ev, true);
        if handled.is_none()
            && req.action == accesskit::Action::Focus
            && let Some(stop) = self.nearest_focusable(target)
            && let Some(window) = self.tree.window_of(stop)
        {
            self.set_focus(window, Some(stop), FocusCause::Program);
        }
    }

    /// `node` or its nearest ancestor that is a Tab stop.
    pub fn nearest_focusable(&self, node: NodeId) -> Option<NodeId> {
        chain(&self.tree, node).into_iter().find(|id| self.is_focusable(*id))
    }

    /// Whether `id` is a Tab stop (its element is focusable).
    pub fn is_focusable(&self, id: NodeId) -> bool {
        self.tree.nodes.get(&id).and_then(|n| n.element.as_ref()).is_some_and(|e| e.focusable())
    }

    /// Delivers `ev` to `target`, then, when `bubble`, to its ancestors until one handles it.
    /// Returns the node that handled it.
    pub(crate) fn dispatch(&mut self, target: NodeId, ev: &Event, bubble: bool) -> Option<NodeId> {
        let mut cur = Some(target);
        while let Some(id) = cur {
            cur = self.tree.nodes.get(&id)?.parent;
            if self.deliver(id, target, ev) == Handled::Yes {
                return Some(id);
            }
            if !bubble {
                return None;
            }
        }
        None
    }

    pub(crate) fn deliver(&mut self, node: NodeId, target: NodeId, ev: &Event) -> Handled {
        let Some(mut el) = self.tree.nodes.get_mut(&node).and_then(|n| n.element.take()) else {
            return Handled::No;
        };
        let handled = el.event(&mut EventCx { rt: &mut *self, node, target }, ev);
        if let Some(n) = self.tree.nodes.get_mut(&node) {
            n.element = Some(el);
        }
        handled
    }

    fn flush_queue(&mut self, window: WindowId) {
        while let Some((node, ev)) = self.input_mut(window).and_then(|i| i.queue.pop_front()) {
            self.deliver(node, node, &ev);
        }
    }

    /// Clears what input state referred to a node that is gone. The press it owned ends without an
    /// event; focus falls back after the build pass ([`Runtime::repair_focus`]).
    pub(crate) fn forget_node(&mut self, window: WindowId, id: NodeId) {
        self.interact.remove(&id);
        self.interact_facts.remove(&id);
        self.hovered_parts.remove(&id);
        self.part_hovers.remove(&id);
        if let Some(input) = self.input_mut(window).filter(|i| i.key_capture.as_ref().is_some_and(|(node, _)| *node == id)) {
            input.key_capture = None;
        }
        self.hover_points.remove(&id);
        self.forget_drag_node(id);
        self.forget_touch_node(window, id);
        if let Some(w) = self.windows.get_mut(&window) {
            w.window_drops.retain(|&(n, _)| n != id);
            w.close_guards.retain(|&n| n != id);
        }
        let Some(input) = self.input_mut(window) else { return };
        if input.hover_hit.is_some_and(|(h, _)| h.node == id) {
            input.hover_hit = None;
            input.cursor_check = true;
        }
        if input.press_hit.is_some_and(|(h, _)| h.node == id) {
            input.press_hit = None;
            input.cursor_check = true;
        }
        if input.press.is_some_and(|p| p.owner == Some(id)) {
            input.machine = PressMachine::new();
            input.press = None;
        }
        input.pressed.retain(|&n| n != id);
        input.hovered.retain(|&n| n != id);
    }

    /// After the build pass: focus enters the content that asked for it; when `window`'s focused
    /// node is gone, focus moves to the node it named or the nearest remaining Tab stop on its old
    /// path, or nowhere; then the nodes that gave up focus in the pass lose it, if they still hold
    /// it ([`crate::BuildCx::surrender_focus`]).
    pub(crate) fn repair_focus(&mut self, window: WindowId) {
        self.enter_focus(window);
        self.fall_back_focus(window);
        let surrenders = self.windows.get_mut(&window).map(|w| std::mem::take(&mut w.surrenders)).unwrap_or_default();
        if self.focused(window).is_some_and(|f| surrenders.contains(&f)) {
            self.set_focus(window, None, FocusCause::Program);
        }
    }

    fn fall_back_focus(&mut self, window: WindowId) {
        // Focus inside a leaving layer child (`LayerSpec::inert`) moves on as if the child were gone.
        let leaving = |id: &NodeId| self.windows.get(&window).is_some_and(|w| w.inert(&self.tree, *id));
        let gone = self.focused(window).filter(|f| !self.tree.nodes.contains_key(f) || leaving(f));
        if gone.is_none() {
            self.forget_focus_returns(window);
            return;
        }
        let chain_now = self.input(window).map(|i| i.focus_chain.clone()).unwrap_or_default();
        // A node that named where focus returns (a popup: its opener) and went with focus inside.
        let returned = self
            .windows
            .get(&window)
            .and_then(|w| chain_now.iter().find_map(|gone| w.focus_returns.get(gone).copied()).filter(|to| self.tree.nodes.contains_key(to)));
        let fallback = returned.or_else(|| chain_now.iter().copied().find(|id| self.is_focusable(*id) && !leaving(id)));
        if let Some(input) = self.input_mut(window) {
            input.focused = None;
            input.focus_moved = true;
        }
        self.note_focused(window, None);
        self.set_focus(window, fallback, FocusCause::Removed);
        self.forget_focus_returns(window);
    }

    /// Moves focus into the content of the nodes that asked in this build pass: to the first
    /// text input among their Tab stops when they asked for one and it exists, else their first
    /// Tab stop, in tree order.
    fn enter_focus(&mut self, window: WindowId) {
        let Some(entries) = self.windows.get_mut(&window).map(|w| std::mem::take(&mut w.focus_entries)) else { return };
        for (node, text_first) in entries {
            let mut stops = Vec::new();
            let mut stack = vec![node];
            while let Some(id) = stack.pop() {
                let Some(n) = self.tree.nodes.get(&id) else { continue };
                if let Some(el) = n.element.as_ref().filter(|e| e.focusable()) {
                    stops.push((id, el.text_input().is_some()));
                }
                stack.extend(n.children.iter().rev().copied());
            }
            let pick = stops.iter().find(|(_, text)| text_first && *text).or(stops.first()).map(|(id, _)| *id);
            if pick.is_some() {
                self.set_focus(window, pick, FocusCause::Program);
            }
        }
    }

    /// Drops the focus returns of nodes that are gone.
    pub(crate) fn forget_focus_returns(&mut self, window: WindowId) {
        let Some(w) = self.windows.get_mut(&window) else { return };
        if !w.focus_returns.is_empty() {
            let tree = &self.tree;
            w.focus_returns.retain(|from, _| tree.nodes.contains_key(from));
        }
    }

    /// Starts, moves or stops `window`'s platform IME session to follow its focused node's text
    /// input and cursor rect. Only a window with key focus holds a session, so the session moves
    /// with the user between windows. A session that leaves its node while the platform composes
    /// fences the composition's late events (the node committed the composition itself when it
    /// lost focus). The session's node's surrounding text goes out at the start and at each new
    /// revision, and an interrupt the node asked for goes out as a reset. The frame runs it after
    /// layout.
    pub(crate) fn sync_ime(&mut self, window: WindowId) {
        let active = self.windows.get(&window).is_some_and(|w| *self.store.peek(w.env.window_focused));
        let want = self.focused(window).filter(|_| active).and_then(|f| {
            let n = self.tree.nodes.get(&f)?;
            let el = n.element.as_ref()?;
            let purpose: ImePurpose = el.text_input()?;
            let local = el.ime_cursor().unwrap_or(Rect::from_min_size(Pos2::ZERO, n.frame.size));
            Some((f, purpose, local.translate(window_origin(&self.tree, f))))
        });
        let since =
            self.windows.get(&window).and_then(|w| w.input.ime.at.filter(|(n, _)| want.is_some_and(|(f, ..)| f == *n)).and(w.input.ime.surrounding));
        let fresh = want.and_then(|(node, ..)| self.tree.nodes.get(&node).and_then(|n| n.element.as_ref()?.ime_surrounding(since)));
        let Some(w) = self.windows.get_mut(&window) else { return };
        let ime = &mut w.input.ime;
        let ended = |ime: &mut ImeSession| {
            if std::mem::take(&mut ime.composing) {
                ime.fenced = true;
            }
            ime.surrounding = None;
            ime.interrupt = false;
        };
        match (ime.at, want) {
            (None, None) => {}
            (Some(_), None) => {
                w.ime_requests.push(ImeRequest::Stop);
                ended(ime);
                ime.at = None;
            }
            (old, Some((node, purpose, rect))) => {
                let start = ImeRequest::Start { purpose, cursor: rect };
                match old {
                    Some((n, _)) if n != node => {
                        w.ime_requests.push(ImeRequest::Stop);
                        ended(ime);
                        w.ime_requests.push(start);
                    }
                    None => w.ime_requests.push(start),
                    Some((_, last)) if last != rect => w.ime_requests.push(ImeRequest::Cursor(rect)),
                    Some(_) => {}
                }
                ime.at = Some((node, rect));
                if std::mem::take(&mut ime.interrupt) {
                    w.ime_requests.push(ImeRequest::Reset);
                    if std::mem::take(&mut ime.composing) {
                        ime.fenced = true;
                    }
                }
                if let Some(s) = fresh.filter(|s| ime.surrounding != Some(s.revision)) {
                    ime.surrounding = Some(s.revision);
                    w.ime_requests.push(ImeRequest::Surrounding { text: s.text, cursor: s.cursor, anchor: s.anchor });
                }
            }
        }
    }

    /// Clipboard writes and read requests since the last call; the driver hands them to the
    /// platform.
    pub(crate) fn take_clipboard_requests(&mut self) -> Vec<(WindowId, ClipboardRequest)> {
        std::mem::take(&mut self.clipboard)
    }
}

impl crate::paint::PaintCx<'_> {
    /// This node's hover, press and focus state. The read is recorded, so the node repaints when
    /// the state changes, and only then.
    pub fn interaction(&mut self) -> Interact {
        let node = self.node();
        self.interaction_of(node)
    }

    /// One fact of this node's interaction state. The read is of that fact alone, so the node
    /// repaints when it flips and not when another fact changes (a focus ring that reads
    /// `FocusVisible` does not repaint on hover).
    pub fn interaction_fact(&mut self, fact: InteractFact) -> bool {
        let node = self.node();
        self.read_interaction(node, fact)
    }

    /// The part of this node the pointer hovers: its ui-rt part id, 0 for the node's own shape, `None`
    /// while the pointer is over none of the node's shapes. The read flips only when this node's
    /// hovered part changes, so a pointer moving between other nodes' shapes repaints nothing here.
    pub fn hovered_part(&mut self) -> Option<u64> {
        let node = self.node();
        match self.runtime_mut().hovered_part_cell(node) {
            Some(cell) => cell.get(self),
            None => None,
        }
    }

    /// Another node's interaction state (a row painting its focus ring while its list has focus).
    pub fn interaction_of(&mut self, node: NodeId) -> Interact {
        match self.runtime_mut().interaction_cell(node) {
            Some(cell) => cell.get(self),
            None => Interact::default(),
        }
    }

    /// One fact of `node`'s interaction state, recording a read of that fact alone: this node
    /// repaints when the fact flips, not when another fact of `node` changes.
    pub fn read_interaction(&mut self, node: NodeId, fact: InteractFact) -> bool {
        match self.runtime_mut().interaction_fact_cell(node, fact) {
            Some(cell) => cell.get(self),
            None => false,
        }
    }
}

impl crate::build::BuildCx<'_> {
    /// The pointer's place (window coordinates) while it hovers `node` or a node under it, without
    /// recording a read.
    pub fn hover_point(&self, node: NodeId) -> Option<Pos2> {
        self.runtime().hover_point(node)
    }

    /// [`hover_point`](Self::hover_point), recording a build read: this node builds again when the
    /// pointer moves over `node` (or a node under it), enters or leaves it.
    pub fn read_hover_point(&mut self, node: NodeId) -> Option<Pos2> {
        match self.runtime_mut().hover_point_cell(node) {
            Some(cell) => cell.get(self),
            None => None,
        }
    }

    /// A command ran from `from` in this node's window (a menu pick, the palette): the layer
    /// children that close on a command and do not own `from` are dismissed
    /// ([`Runtime::dismiss_on_command`]).
    pub fn dismiss_on_command(&mut self, from: Option<NodeId>) {
        let window = self.window();
        self.runtime_mut().dismiss_on_command(window, from);
    }

    /// Asks every window to close, through each window's close path ([`Runtime::quit`]): Quit
    /// picked from a menu or the palette.
    pub fn quit(&mut self) {
        self.runtime_mut().quit();
    }

    /// `node`'s hover, press and focus state as of now, without recording a read.
    pub fn interaction(&self, node: NodeId) -> Interact {
        self.runtime().interaction(node)
    }

    /// The cell `node`'s interaction state lives in; reading it through this context records a
    /// build read, so this node builds again when any fact of it changes.
    pub fn interaction_cell(&mut self, node: NodeId) -> Option<State<Interact>> {
        self.runtime_mut().interaction_cell(node)
    }

    /// One fact of `node`'s interaction state, recording a build read of that fact alone: this node
    /// builds again when the fact flips.
    pub fn read_interaction(&mut self, node: NodeId, fact: InteractFact) -> bool {
        match self.runtime_mut().interaction_fact_cell(node, fact) {
            Some(cell) => cell.get(self),
            None => false,
        }
    }

    /// The part of `node` the pointer hovers, as of now, without recording a read.
    pub fn hovered_part(&self, node: NodeId) -> Option<u64> {
        self.runtime().hovered_part(node)
    }

    /// The part of `node` the pointer hovers (as `PaintCx::hovered_part` reads it for its own node),
    /// recording a build read that flips only when `node`'s hovered part changes.
    pub fn read_hovered_part(&mut self, node: NodeId) -> Option<u64> {
        match self.runtime_mut().hovered_part_cell(node) {
            Some(cell) => cell.get(self),
            None => None,
        }
    }

    /// Whether the pointer hovers the part `part` of `node` (0 for its own shape), recording a build
    /// read that flips only when that part's hover does: a pointer moving between `node`'s other
    /// parts builds nothing here.
    pub fn read_part_hovered(&mut self, node: NodeId, part: u64) -> bool {
        match self.runtime_mut().part_hover_cell(node, part) {
            Some(cell) => cell.get(self),
            None => false,
        }
    }

    /// This node takes `keys` in its window ahead of the focused node while it holds a mode (a
    /// middle-button auto-scroll latch, which Escape ends whatever has focus); a key it does not
    /// handle goes on to focus. No keys releases the capture if this node holds it; a node that
    /// goes releases it too.
    pub fn capture_keys(&mut self, keys: &[Key]) {
        let (node, window) = (self.node(), self.window());
        let Some(input) = self.runtime_mut().input_mut(window) else { return };
        if !keys.is_empty() {
            input.key_capture = Some((node, keys.iter().copied().collect()));
        } else if input.key_capture.as_ref().is_some_and(|(held, _)| *held == node) {
            input.key_capture = None;
        }
    }

    /// Moves key focus to this node (a field that takes focus when it opens).
    pub fn request_focus(&mut self) {
        let (node, window) = (self.node(), self.window());
        self.runtime_mut().set_focus(window, Some(node), FocusCause::Program);
    }

    /// Moves key focus to `node` in this node's window (a popup that focuses its first stop when it
    /// appears).
    pub fn focus_node(&mut self, node: NodeId) {
        let window = self.window();
        self.runtime_mut().set_focus(window, Some(node), FocusCause::Program);
    }

    /// Moves focus into this node's content once the build pass has made it: to its first Tab stop
    /// in tree order, or its first text input when `text_first` and there is one (a dialog that
    /// focuses its first field when it opens).
    pub fn focus_into(&mut self, text_first: bool) {
        let node = self.node();
        self.focus_into_node(node, text_first);
    }

    /// [`Self::focus_into`] for `node`: once the build pass has made it, focus moves to `node` when it is a Tab stop,
    /// else to its first Tab stop in tree order (a collection row that holds stops but is none).
    pub fn focus_into_node(&mut self, node: NodeId, text_first: bool) {
        let window = self.window();
        if let Some(w) = self.runtime_mut().windows.get_mut(&window) {
            w.focus_entries.push((node, text_first));
        }
    }

    /// Whether `node` is a Tab stop (its element is focusable).
    pub fn is_focusable(&self, node: NodeId) -> bool {
        self.runtime().is_focusable(node)
    }

    /// Gives up key focus once this frame's build pass ends, if this node still holds it then (a
    /// field a scrub left, or one whose Escape leaves it). A scope that closes in the same pass
    /// with the node inside returns focus to where it names first, and that move stands.
    pub fn surrender_focus(&mut self) {
        let (node, window) = (self.node(), self.window());
        if let Some(w) = self.runtime_mut().windows.get_mut(&window) {
            w.surrenders.push(node);
        }
    }

    /// Names where focus goes when this node goes with focus inside it (a popup returns it to its
    /// opener); `None` forgets it. Without one, focus falls back to the nearest stop on its old
    /// path.
    pub fn set_focus_return(&mut self, to: Option<NodeId>) {
        let (node, window) = (self.node(), self.window());
        let Some(w) = self.runtime_mut().windows.get_mut(&window) else { return };
        match to {
            Some(to) => {
                w.focus_returns.insert(node, to);
            }
            None => {
                w.focus_returns.remove(&node);
            }
        }
    }
}
