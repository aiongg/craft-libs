//! Input routing: hit testing, the press lifecycle with gesture recognition, hover, key focus and
//! key bubbling, IME sessions and accessibility actions.
//!
//! One press at a time runs through craft-behavior's [`PressMachine`], stepped once per pointer
//! event. Its press winner is the topmost shape that takes the press; the node that handles the
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
use crate::hit::{Hit, chain, contains, hit_test, window_origin};
use crate::id::{NodeId, WindowId};
use crate::layer::{DismissReason, Layer, OutsidePress};
use crate::platform::{ImePurpose, ImeRequest, PointerEvent, PointerPhase, WindowEvent};
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
    /// A mouse or pen pointer inside the surface.
    hover_pos: Option<Pos2>,
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
}

impl Runtime {
    fn input(&self, window: WindowId) -> Option<&InputState> {
        self.windows.get(&window).map(|w| &w.input)
    }

    fn input_mut(&mut self, window: WindowId) -> Option<&mut InputState> {
        self.windows.get_mut(&window).map(|w| &mut w.input)
    }

    /// Routes `window`'s pointer, wheel, key, text, IME, paste and accessibility events to its
    /// nodes.
    pub(crate) fn route_input(&mut self, window: WindowId, ev: &WindowEvent) {
        match ev {
            WindowEvent::Key { down: true, .. } | WindowEvent::Text(_) => self.show_focus(window, true),
            WindowEvent::Pointer(p) if p.phase == PointerPhase::Down => self.show_focus(window, false),
            WindowEvent::Access(req) if req.action == accesskit::Action::Click => self.show_focus(window, false),
            _ => {}
        }
        match ev {
            WindowEvent::Pointer(p) => self.route_pointer(window, *p),
            WindowEvent::Wheel { pos, delta, mods } => {
                if let Some(hit) = self.hit(window, *pos, &|s| s.intersects(Sense::WHEELS)) {
                    let ev = Event::Wheel(WheelInput { pos: *pos, delta: *delta, mods: *mods, hit: Some(hit) });
                    self.dispatch(hit.node, &ev, true);
                }
            }
            WindowEvent::Key { key, mods, down, repeat } => self.route_key(window, KeyInput { key: *key, mods: *mods, down: *down, repeat: *repeat }),
            WindowEvent::Text(text) => self.send_to_focused(window, Event::Text(text.clone())),
            WindowEvent::ImePreedit { text, cursor } => self.send_to_focused(window, Event::ImePreedit { text: text.clone(), cursor: cursor.clone() }),
            WindowEvent::ImeCommit(text) => self.send_to_focused(window, Event::ImeCommit(text.clone())),
            WindowEvent::Paste(text) => self.send_to_focused(window, Event::Paste(text.clone())),
            WindowEvent::Access(req) => self.route_access(req),
            _ => {}
        }
        self.flush_queue(window);
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
        if let Some(old) = input.focused {
            input.queue.push_back((old, Event::Focus(false)));
        }
        input.focused = node;
        input.focus_moved = true;
        let new_chain = node.map(|n| chain(&self.tree, n)).unwrap_or_default();
        let Some(input) = self.input_mut(window) else { return };
        if let Some(new) = node {
            input.queue.push_back((new, Event::Focus(true)));
        }
        input.focus_chain = new_chain.clone();
        let touched: Vec<NodeId> = old_chain.into_iter().chain(new_chain).collect();
        self.refresh_interact(&touched);
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
        let leaf = match input.hover_pos {
            Some(pos) if !input.machine.dragging() => self.hit(window, pos, &|s| s.intersects(Sense::HOVER | Sense::PRESS)).map(|h| h.node),
            _ => None,
        };
        let now = leaf.map(|n| chain(&self.tree, n)).unwrap_or_default();
        let Some(input) = self.input_mut(window) else { return };
        if now == input.hovered {
            return;
        }
        let old = std::mem::replace(&mut input.hovered, now.clone());
        let touched: Vec<NodeId> = old.into_iter().chain(now).collect();
        self.refresh_interact(&touched);
    }

    fn set_pressed(&mut self, window: WindowId, nodes: Vec<NodeId>) {
        let Some(input) = self.input_mut(window) else { return };
        let old = std::mem::replace(&mut input.pressed, nodes.clone());
        let touched: Vec<NodeId> = old.into_iter().chain(nodes).collect();
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
        match p.phase {
            PointerPhase::Move => {
                let Some(input) = self.input_mut(window) else { return };
                if p.kind != PointerKind::Touch {
                    input.hover_pos = Some(p.pos);
                }
                let delta = input.press.as_mut().filter(|q| q.pointer == p.id).map(|press| {
                    let delta = p.pos - press.pos;
                    press.pos = p.pos;
                    delta
                });
                if let Some(delta) = delta {
                    self.step_press(window, &[PressEvent::moved(p.pos, None)], p, delta, None);
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
                input.hover_pos = (p.kind != PointerKind::Touch).then_some(p.pos);
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
                if let Some(input) = self.input_mut(window) {
                    input.hover_pos = None;
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
        let p =
            PointerEvent { id: press.pointer, kind: press.kind, phase: PointerPhase::Cancel, button: press.button, pos: press.pos, mods: press.mods };
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
        let ev = Event::Pointer(PointerInput { id: p.id, kind: p.kind, button: p.button, mods: p.mods, gesture, pos: p.pos, hit: Some(hit) });
        if matches!(gesture, Gesture::Down { .. }) {
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
        if k.down && k.key == Key::Escape && self.input(window).is_some_and(|i| i.press.is_some()) {
            self.cancel_press(window, PressCancel::Escape);
            return;
        }
        let Some(target) = self.focused(window).or_else(|| self.root(window)) else {
            return;
        };
        let handled = self.dispatch(target, &Event::Key(k), true).is_some();
        let plain_tab = k.key == Key::Tab && !(k.mods.cmd || k.mods.ctrl || k.mods.alt);
        if !handled && k.down && plain_tab {
            self.move_focus(window, !k.mods.shift);
        }
    }

    fn send_to_focused(&mut self, window: WindowId, ev: Event) {
        if let Some(focused) = self.focused(window) {
            self.dispatch(focused, &ev, false);
        }
    }

    fn route_access(&mut self, req: &accesskit::ActionRequest) {
        let raw = req.target_node.0;
        let target = if self.tree.nodes.contains_key(&NodeId(raw)) {
            Some((NodeId(raw), None))
        } else {
            self.windows.values().find_map(|w| w.access.subs.get(&raw).map(|&(owner, key)| (owner, Some(key))))
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

    fn deliver(&mut self, node: NodeId, target: NodeId, ev: &Event) -> Handled {
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
        let Some(input) = self.input_mut(window) else { return };
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
        let Some(focused) = self.focused(window) else {
            return;
        };
        if self.tree.nodes.contains_key(&focused) {
            return;
        }
        let fallback = self.input(window).and_then(|i| i.focus_chain.iter().copied().find(|id| self.is_focusable(*id)));
        if let Some(input) = self.input_mut(window) {
            input.focused = None;
            input.focus_moved = true;
        }
        self.set_focus(window, fallback);
    }

    /// Starts, moves or stops `window`'s platform IME session to follow its focused node's text
    /// input and cursor rect. The frame runs it after layout.
    pub(crate) fn sync_ime(&mut self, window: WindowId) {
        let want = self.focused(window).and_then(|f| {
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

    /// Another node's interaction state (a row painting its focus ring while its list has focus).
    pub fn interaction_of(&mut self, node: NodeId) -> Interact {
        match self.runtime_mut().interaction_cell(node) {
            Some(cell) => cell.get(self),
            None => Interact::default(),
        }
    }
}

impl crate::build::BuildCx<'_> {
    /// Moves key focus to this node (a field that takes focus when it opens).
    pub fn request_focus(&mut self) {
        let (node, window) = (self.node(), self.window());
        self.runtime_mut().set_focus(window, Some(node));
    }
}
