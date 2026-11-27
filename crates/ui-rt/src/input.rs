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

use crate::event::{AccessInput, Event, EventCx, Gesture, Handled, KeyInput, PointerInput, WheelInput};
use crate::gesture::Touches;
use crate::hit::{Hit, chain, contains, hit_test, window_origin};
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
    hover_hit: Option<(Hit, Option<Cursor>)>,
    /// The shape the open press went down on, with the cursor it names.
    press_hit: Option<(Hit, Option<Cursor>)>,
    hovered: Vec<NodeId>,
    pressed: Vec<NodeId>,
    pub(crate) focused: Option<NodeId>,
    /// The focused node and its ancestors when focus moved there: where focus falls back to when
    /// the node goes.
    focus_chain: Vec<NodeId>,
    /// Focus events, delivered after the handler that caused them returns.
    queue: VecDeque<(NodeId, Event)>,
    ime: Option<(NodeId, Rect)>,
    /// Focus moved since the last frame: the access focus and the IME session follow.
    pub(crate) focus_moved: bool,
    /// Modality, as craft keeps it: a key press or typed text shows the focus ring, a pointer
    /// press or an accessibility click hides it.
    focus_shown: bool,
    pub(crate) touches: Touches,
    /// The cursor last sent to the platform.
    pub(crate) cursor: Cursor,
    /// Something that decides the cursor changed since it was last picked.
    pub(crate) cursor_check: bool,
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
    pub(crate) fn press_cursor_source(&self) -> Option<(NodeId, Option<Cursor>)> {
        self.press.and(self.press_hit).map(|(hit, cursor)| (hit.node, cursor))
    }

    pub(crate) fn hover_cursor_source(&self) -> Option<(NodeId, Option<Cursor>)> {
        self.hover_hit.map(|(hit, cursor)| (hit.node, cursor))
    }
}

impl Runtime {
    fn input(&self, window: WindowId) -> Option<&InputState> {
        self.windows.get(&window).map(|w| &w.input)
    }

    fn input_mut(&mut self, window: WindowId) -> Option<&mut InputState> {
        self.windows.get_mut(&window).map(|w| &mut w.input)
    }

    /// Writes the window's modifier and pointer cells from `ev`, so a build or paint that read them
    /// runs again when they change.
    fn note_input(&mut self, window: WindowId, ev: &WindowEvent) {
        let Some(env) = self.windows.get(&window).map(|w| w.env) else { return };
        let mods = match ev {
            WindowEvent::Pointer(p) => Some(p.mods),
            WindowEvent::Wheel { mods, .. } | WindowEvent::Pinch { mods, .. } | WindowEvent::Key { mods, .. } => Some(*mods),
            _ => None,
        };
        if let Some(mods) = mods {
            env.modifiers.set_if_changed(&mut self.store, mods);
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
            WindowEvent::Text(text) => self.send_to_focused(window, Event::Text(text.clone())),
            WindowEvent::ImePreedit { text, cursor } => {
                self.send_to_focused(window, Event::ImePreedit { text: text.clone(), cursor: cursor.clone() })
            }
            WindowEvent::ImeCommit(text) => self.send_to_focused(window, Event::ImeCommit(text.clone())),
            WindowEvent::Paste(text) => self.send_to_focused(window, Event::Paste(text.clone())),
            WindowEvent::Access(req) => self.route_access(window, req),
            _ => {}
        }
        self.flush_queue(window);
        self.settle_drag();
        let windows: Vec<WindowId> = self.windows.keys().copied().collect();
        for w in windows {
            self.refresh_cursor(w);
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
    fn shape_cursor(&self, hit: Hit, pos: Pos2) -> Option<Cursor> {
        let node = self.tree.nodes.get(&hit.node)?;
        let local = pos - window_origin(&self.tree, hit.node);
        node.hits
            .iter()
            .rev()
            .find(|h| h.kind == crate::hit::HitKind::Input && h.part == hit.part && h.sense == hit.sense && h.shape.contains(local))
            .and_then(|h| h.cursor)
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

    /// The node with key focus in `window`.
    pub fn focused(&self, window: WindowId) -> Option<NodeId> {
        self.input(window)?.focused
    }

    /// Moves `window`'s key focus. The old and the new node get [`Event::Focus`] once the current
    /// handler returns; paint readers of their interaction state repaint. A node outside the
    /// window is ignored.
    pub fn set_focus(&mut self, window: WindowId, node: Option<NodeId>) {
        let node = node.filter(|id| self.tree.window_of(*id) == Some(window));
        let Some(input) = self.input_mut(window) else { return };
        if node == input.focused {
            return;
        }
        let old_chain = std::mem::take(&mut input.focus_chain);
        let old_focus = input.focused;
        if let Some(old) = old_focus {
            input.queue.push_back((old, Event::Focus(false)));
        }
        input.focused = node;
        input.focus_moved = true;
        let new_chain = node.map(|n| chain(&self.tree, n)).unwrap_or_default();
        let Some(input) = self.input_mut(window) else { return };
        if let Some(new) = node {
            input.queue.push_back((new, Event::Focus(true)));
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
        let watching: Vec<(NodeId, Option<NodeId>)> = w.layered.iter().filter(|(_, s)| s.focus_left).map(|(id, s)| (*id, s.anchor_node())).collect();
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
        self.set_focus(window, Some(order[next]));
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
        let value = self.windows.keys().find_map(|&w| self.input(w)?.hover_hit.filter(|(h, _)| h.node == node).map(|(h, _)| h.part));
        let n = self.tree.nodes.get_mut(&node)?;
        let cell = self.store.create(value);
        n.owned.push(cell.slot_ref());
        self.hovered_parts.insert(node, cell);
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
        if input.hover_hit != hit {
            let before = std::mem::replace(&mut input.hover_hit, hit);
            input.cursor_check = true;
            self.refresh_hovered_parts(before.map(|(h, _)| h), hit.map(|(h, _)| h));
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
                if self.dismiss_outside(window, p.pos) {
                    self.refresh_hover(window);
                    return;
                }
                let time = self.time;
                let slop = self.slop(p.kind);
                let Some(input) = self.input_mut(window) else { return };
                let clicks = count_clicks(input.last_press, p.pos, time, p.mods, slop);
                input.last_press = Some((p.pos, time, clicks, p.mods));
                input.press =
                    Some(Press { pointer: p.id, kind: p.kind, button: p.button, mods: p.mods, owner: None, pos: p.pos, long_pressed: false });
                let ev = PressEvent::pressed_by(p.button, p.pos, time, clicks);
                self.step_press(window, &[ev], p, Vec2::ZERO, None);
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
            .filter_map(|id| w.layered.get(id).filter(|s| s.outside_press != OutsidePress::Ignore).map(|s| (*id, s.outside_press, s.anchor_node())))
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
        let Some(win) = self.windows.get(&window) else { return };
        let tree = &self.tree;
        let step = machine.step(input, |at, button| {
            let accept = |s: Sense| if button == PointerButton::Secondary { s.contains(Sense::CONTEXT_CLICK) } else { s.takes_press() };
            hit_test(tree, win, at, &accept).map(|h| PressPick::new(h, h.sense))
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
            if !hit.sense.contains(Sense::KEEP_FOCUS) {
                let stop = self.nearest_focusable(hit.node);
                self.set_focus(window, stop);
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

    fn route_key(&mut self, window: WindowId, k: KeyInput) {
        self.drag_mods(k.mods);
        if k.down && k.key == Key::Escape && self.input(window).is_some_and(|i| i.press.is_some()) {
            self.cancel_press(window, PressCancel::Escape);
            return;
        }
        let Some(target) = self.focused(window).or_else(|| self.key_root(window)) else {
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
        let top = self.layered_top_first(window).into_iter().find(|id| w.layered.get(id).is_some_and(|s| s.escape));
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
            modal.children.iter().any(|c| w.layered.get(c).is_some_and(|s| s.app_modal)).then_some(*id)
        })
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
        let ev = Event::Access(AccessInput { action: req.action, data: req.data.clone(), target, sub });
        let handled = self.dispatch(target, &ev, true);
        if handled.is_none()
            && req.action == accesskit::Action::Focus
            && let Some(stop) = self.nearest_focusable(target)
            && let Some(window) = self.tree.window_of(stop)
        {
            self.set_focus(window, Some(stop));
        }
    }

    /// `node` or its nearest ancestor that is a Tab stop.
    pub fn nearest_focusable(&self, node: NodeId) -> Option<NodeId> {
        chain(&self.tree, node).into_iter().find(|id| self.is_focusable(*id))
    }

    fn is_focusable(&self, id: NodeId) -> bool {
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
        self.forget_drag_node(id);
        self.forget_touch_node(window, id);
        if let Some(w) = self.windows.get_mut(&window) {
            w.window_drops.retain(|&n| n != id);
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

    /// When `window`'s focused node is gone, focus moves to the nearest remaining Tab stop on its
    /// old path, or nowhere.
    pub(crate) fn repair_focus(&mut self, window: WindowId) {
        self.enter_focus(window);
        let gone = self.focused(window).filter(|f| !self.tree.nodes.contains_key(f));
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
        let fallback = returned.or_else(|| chain_now.iter().copied().find(|id| self.is_focusable(*id)));
        if let Some(input) = self.input_mut(window) {
            input.focused = None;
            input.focus_moved = true;
        }
        self.set_focus(window, fallback);
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
                self.set_focus(window, pick);
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
    /// with the user between windows. The frame runs it after layout.
    pub(crate) fn sync_ime(&mut self, window: WindowId) {
        let active = self.windows.get(&window).is_some_and(|w| *self.store.peek(w.env.window_focused));
        let want = self.focused(window).filter(|_| active).and_then(|f| {
            let n = self.tree.nodes.get(&f)?;
            let el = n.element.as_ref()?;
            let purpose: ImePurpose = el.text_input()?;
            let local = el.ime_cursor().unwrap_or(Rect::from_min_size(Pos2::ZERO, n.frame.size));
            Some((f, purpose, local.translate(window_origin(&self.tree, f))))
        });
        let Some(w) = self.windows.get_mut(&window) else { return };
        match (w.input.ime, want) {
            (None, None) => {}
            (Some(_), None) => {
                w.ime_requests.push(ImeRequest::Stop);
                w.input.ime = None;
            }
            (old, Some((node, purpose, rect))) => {
                let start = ImeRequest::Start { purpose, cursor: rect };
                match old {
                    Some((n, _)) if n != node => {
                        w.ime_requests.push(ImeRequest::Stop);
                        w.ime_requests.push(start);
                    }
                    None => w.ime_requests.push(start),
                    Some((_, last)) if last != rect => w.ime_requests.push(ImeRequest::Cursor(rect)),
                    Some(_) => {}
                }
                w.input.ime = Some((node, rect));
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

    /// Moves key focus to this node (a field that takes focus when it opens).
    pub fn request_focus(&mut self) {
        let (node, window) = (self.node(), self.window());
        self.runtime_mut().set_focus(window, Some(node));
    }

    /// Moves key focus to `node` in this node's window (a popup that focuses its first stop when it
    /// appears).
    pub fn focus_node(&mut self, node: NodeId) {
        let window = self.window();
        self.runtime_mut().set_focus(window, Some(node));
    }

    /// Moves focus into this node's content once the build pass has made it: to its first Tab stop
    /// in tree order, or its first text input when `text_first` and there is one (a dialog that
    /// focuses its first field when it opens).
    pub fn focus_into(&mut self, text_first: bool) {
        let (node, window) = (self.node(), self.window());
        if let Some(w) = self.runtime_mut().windows.get_mut(&window) {
            w.focus_entries.push((node, text_first));
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
