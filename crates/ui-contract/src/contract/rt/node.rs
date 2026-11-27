//! The retained half of contract content: `ComponentNode` and `ScopeNode`, the ui-rt elements that
//! keep what the last run produced (docs/contract-rt.md sections 2 and 5).

use std::any::Any;

use craft_behavior::{EditorInput, Intent, PartRef, PressStart};
use craft_keymap::Chord;
use craft_ui_rt::{
    AccessCx, Constraints, Cursor, DisplayList, DragData, DragOutcome, DropPhase, Element, Event, EventCx, Gesture, Handled, HitShape, ImePurpose,
    InteractFact, LayoutCx, Mods, NodeId, PaintCx, PointerButton, PointerInput, Pos2, Rect, Sense, State, Vec2,
};

use crate::contract::Id;

use super::drag::{DragSource, DropRule};
use super::focus::FocusKept;
use super::keys::Taken;
use super::overlay::PopupStates;
use super::sweep::Sweep;
use super::tip::TipKept;

/// The pointer state a paint variant was recorded for (section 8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pointer {
    /// Neither hovered nor pressed.
    Rest,
    /// A mouse or pen pointer is over the node.
    Hover,
    /// The node holds a press.
    Press,
}

/// An input a node's retained half queued for its next run (section 4): step 8 senses from it.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum NodeInput {
    /// A press phase. A `Down` is queued without marking the node (the press look is paint-only)
    /// unless the part it went down on sets its value on press; the phases after it mark the node.
    Pointer {
        /// The phase.
        gesture: Gesture,
        /// Where the pointer is, in the node's coordinates.
        pos: Pos2,
        /// The button that pressed.
        button: PointerButton,
        /// The modifiers held.
        mods: Mods,
        /// The part of the node the press went down on: the ui-rt part id of its kept shape, 0 for
        /// the node's own shape.
        part: u64,
        /// The press, with its origin in the node's coordinates.
        start: PressStart,
    },
    /// An assistive technology's action on the node or one of its sub-nodes.
    Access(super::access::AccessAction),
    /// The drag the node started ended.
    DragEnd(DragOutcome),
    /// A payload the node's drop rule takes was dropped on it.
    Drop(DragData),
    /// An intent the key stage took for the focused node (`super::keys`): from a key press, or typed
    /// or pasted text. The run reads the queued intents through `Cx::intents`.
    Intent {
        /// The intent.
        intent: Intent,
        /// A horizontal arrow sent it: the run reads its move in the node's direction.
        horizontal: bool,
        /// Shift was held at the press.
        shift: bool,
    },
    /// A press a recorder (`OwnerClass::Recorder`) or a claim (`KeyUse::claims`) took, as a chord.
    Chord(Chord),
    /// Text the clipboard delivered to the focused text part (an editor's Paste).
    Paste(std::sync::Arc<str>),
    /// An arrow press as a nudge of the node's content (`KeyUse::nudge`): its unit direction.
    Nudge {
        /// The unit direction.
        direction: Vec2,
        /// The modifiers held.
        mods: Mods,
    },
    /// Input method input for the node's editor ([`super::ime`]): a preedit, or a commit, whose text
    /// follows as an `Intent::Text`.
    Ime(EditorInput),
    /// The node is its window's close guard and the window was asked to close.
    Close(craft_ui_rt::CloseReason),
    /// The node gained (`true`) or lost key focus: the next run's `Response::gained_focus` and
    /// `lost_focus`. Queued without marking the node.
    Focus(bool),
    /// A wheel step the node's wheel shape took, in points by its `wheel_notch`.
    Wheel(craft_behavior::Wheel),
    /// A menu key the window root routed to its menu bar node (F10, a lone Alt, an Alt mnemonic).
    Menu(crate::runtime::keys::MenuKey),
    /// A reveal a child of a container that owns its body's scroll asked for, in window coordinates (`Focus::take_reveal`).
    Reveal(craft_ui_rt::Rect),
}

/// The room a node was shown in. A node shown in other room runs again.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Place {
    pub(crate) room: Vec2,
}

/// A child's place in its owner, as the owner's build laid it out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Placed {
    pub(crate) node: NodeId,
    pub(crate) offset: Vec2,
    pub(crate) size: Vec2,
}

/// One recorded paint of a node: its lists under and over its children.
#[derive(Clone, Debug)]
pub(crate) struct Variant {
    pub(crate) look: Pointer,
    pub(crate) content: DisplayList,
    pub(crate) overlay: DisplayList,
}

/// The rooms a node's size cache keeps: a measuring run's and the real run's, and one more for a
/// fill whose share moved.
const SIZE_CACHE_ROOMS: usize = 3;

/// A part `parts` declared, kept as a hit shape in the node's coordinates. Paint records it with
/// `id` as its ui-rt part id, so the presses that go down on it carry that id.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct KeptPart {
    pub(crate) part: PartRef,
    pub(crate) id: u64,
    pub(crate) shape: HitShape,
    pub(crate) sense: Sense,
    pub(crate) cursor: Option<Cursor>,
    /// What the operating system does with a press on the part (a title bar's drag area, a caption
    /// button, a resize edge).
    pub(crate) exclusive: Option<crate::runtime::windows::WindowHit>,
}

/// The ui-rt part id of `part`: never 0, which names the node's own shape.
pub(crate) fn part_id(part: PartRef) -> u64 {
    craft_ui_rt::hash_key(&part) | 1
}

/// What the last run of a node produced; the `Local` of both node elements.
pub(crate) struct Kept {
    /// The craft `Id` services key by.
    pub(crate) id: Id,
    /// The component key after the last run's writes; `None` before the first and for unkeyed types.
    pub(crate) key: Option<u128>,
    pub(crate) place: Place,
    pub(crate) size: Vec2,
    pub(crate) sense: Sense,
    /// Empty until the first run; one entry when the paint read no pointer state.
    pub(crate) variants: smallvec::SmallVec<[Variant; 1]>,
    pub(crate) parts: Vec<KeptPart>,
    pub(crate) access: Option<accesskit::Node>,
    /// The sub-nodes of the access node; empty with it.
    pub(crate) access_subs: Vec<super::access::BuiltSub>,
    pub(crate) queue: Vec<NodeInput>,
    pub(crate) placed: Vec<Placed>,
    /// A scope's returned value.
    pub(crate) value: Option<Box<dyn Any>>,
    /// The node size cache: the size the node takes in each room it was measured or shown in since
    /// its last run. It holds while the node keeps its key and carries no mark: a measuring run
    /// asks a kept node for its size here instead of running it.
    pub(crate) sizes: smallvec::SmallVec<[(Vec2, Vec2); 2]>,
    /// A container's last measurement of its body, reused while its key, room and content are
    /// unchanged.
    pub(crate) plan: Option<super::container::Plan>,
    /// A container node's body state (open state of a collapsing body).
    pub(crate) body: super::container::BodyKept,
    /// Whether the last real run of the body under this node met fill children; `None` before it.
    pub(crate) body_fills: Option<bool>,
    /// The node's memory slots (`Remember`, the pipeline's records), freed with the node.
    pub(crate) memory: crate::runtime::memory::MemoryStage,
    /// The window's toggle sweep record: the window root creates it, every run copies it from the
    /// flow it was shown in.
    pub(crate) sweep: Option<State<Sweep>>,
    /// The value a sweeping toggle showed at its last run (`Component::sweep_value`).
    pub(crate) toggle: Option<bool>,
    /// The press the node owns, from its `Down` to its release or cancel.
    pub(crate) press: Option<PressStart>,
    /// A drag of the node's press runs, as of the inputs its runs consumed.
    pub(crate) dragging: bool,
    /// The component's popups and whether each is open.
    pub(crate) popups: PopupStates,
    /// The component's tooltip and where it stands.
    pub(crate) tip: Option<TipKept>,
    /// What a drag from the component carries, from its last run.
    pub(crate) drag: Option<DragSource>,
    /// The payloads the component takes as drops, from its last run.
    pub(crate) drop: Option<DropRule>,
    /// A ui-rt drag session the node started is running: its press moves are the session's, not
    /// input for a run.
    pub(crate) drag_session: bool,
    /// The component answers its window's close requests, from its last run.
    pub(crate) close_guard: bool,
    /// The env cells the node's window root provided (window root only).
    pub(crate) env: Option<super::env::EnvCells>,
    /// The node's entry in its window's `Id -> NodeId` map ([`super::ids`]); it leaves the map with the node.
    pub(crate) ids: Option<super::ids::IdEntry>,
    /// The ui-rt part id of the part the open press went down on, 0 for the node's own shape.
    pub(crate) press_part: u64,
    /// The node is a window root scope: it owns the window's Tab and F6 order.
    pub(crate) window_root: bool,
    /// The command board the host gave the window root (window root only).
    pub(crate) commands: Option<super::commands::CommandBoard>,
    /// The cursor over the node's own shape, from `Component::cursor`.
    pub(crate) cursor: Option<Cursor>,
    /// The distance of one wheel line over the node (`Component::wheel_notch`).
    pub(crate) notch: craft_behavior::WheelNotch,
    /// The node registered as its window's menu bar (`Focus::register_menu_bar`), `Some(true)` when it claims a
    /// lone Alt: the window root routes F10, Alt and Alt mnemonics to it as `NodeInput::Menu`.
    pub(crate) menu_bar: Option<bool>,
    /// The component's type name and bound command, from its last run (`runtime::inspect` records them).
    pub(crate) type_name: &'static str,
    pub(crate) command: Option<craft_keymap::CommandRef>,
    /// Paint-time state: the fade between pointer looks.
    pub(crate) fade: super::motion::FadeCell,
    /// The opacity a run aimed at and its tween, evaluated at paint time.
    pub(crate) level: std::cell::RefCell<Option<super::motion::Level>>,
    /// The node's stop, scope and roving group, and its focus ring.
    pub(crate) focus: FocusKept,
    /// The node's input method state: the composition and the caret rect the platform places its
    /// candidate window at.
    pub(crate) ime: super::ime::ImeKept,
    /// A members collection's rows: one sub-list per row and pointer look (`rt/items.rs`).
    pub(crate) members: Vec<super::items::MemberLooks>,
    /// The column group reports the node's last build or run made, handed to the open groups again while it is kept.
    pub(crate) column_reports: Vec<crate::layout::Report>,
}

impl Kept {
    pub(crate) fn new() -> Kept {
        Kept {
            id: Id::NULL,
            ids: None,
            press_part: 0,
            key: None,
            place: Place::default(),
            size: Vec2::ZERO,
            sense: Sense::empty(),
            variants: smallvec::SmallVec::new(),
            parts: Vec::new(),
            access: None,
            access_subs: Vec::new(),
            type_name: "",
            command: None,
            queue: Vec::new(),
            placed: Vec::new(),
            value: None,
            sizes: smallvec::SmallVec::new(),
            plan: None,
            body: super::container::BodyKept::default(),
            body_fills: None,
            memory: Default::default(),
            sweep: None,
            toggle: None,
            press: None,
            popups: PopupStates::new(),
            tip: None,
            drag: None,
            drop: None,
            drag_session: false,
            close_guard: false,
            dragging: false,
            env: None,
            window_root: false,
            commands: None,
            cursor: None,
            notch: craft_behavior::WheelNotch::DEFAULT,
            menu_bar: None,
            fade: Default::default(),
            level: Default::default(),
            focus: FocusKept::default(),
            ime: Default::default(),
            members: Vec::new(),
            column_reports: Vec::new(),
        }
    }

    /// The size cached for `room`.
    pub(crate) fn cached(&self, room: Vec2) -> Option<Vec2> {
        self.sizes.iter().find(|(r, _)| *r == room).map(|(_, size)| *size)
    }

    /// Caches `size` for `room`, keeping the newest few rooms.
    pub(crate) fn cache(&mut self, room: Vec2, size: Vec2) {
        if let Some(entry) = self.sizes.iter_mut().find(|(r, _)| *r == room) {
            entry.1 = size;
            return;
        }
        if self.sizes.len() == SIZE_CACHE_ROOMS {
            self.sizes.remove(0);
        }
        self.sizes.push((room, size));
    }

    /// Places the children where the last build laid them out, moved by `shift` (a scroll body's
    /// offset); a child keeps its size.
    pub(crate) fn place_children(&self, cx: &mut LayoutCx<'_>, shift: Vec2) {
        for p in &self.placed {
            cx.layout_child(p.node, Constraints::tight(p.size));
            cx.place_child(p.node, p.offset - shift);
        }
    }

    /// The sense of the node's own shape (`part` 0) or of the kept part `part`.
    fn sense_of(&self, part: u64) -> Sense {
        if part == 0 { self.sense } else { self.parts.iter().find(|p| p.id == part).map_or(Sense::empty(), |p| p.sense) }
    }

    fn takes_press(&self) -> bool {
        self.sense.takes_press() || self.parts.iter().any(|p| p.sense.takes_press())
    }

    /// The node or one of its parts takes a wheel axis.
    fn takes_wheel(&self) -> bool {
        let wheel = Sense::WHEEL | Sense::WHEEL_X | Sense::WHEEL_Y | Sense::WHEEL_ZOOM;
        !self.sense.intersection(wheel).is_empty() || self.parts.iter().any(|p| !p.sense.intersection(wheel).is_empty())
    }
}

/// A shown component (leaf or container). Its hooks run in its owner's build; this element replays
/// their outputs in ui-rt's phases.
pub(crate) struct ComponentNode {
    pub(crate) key: u64,
    /// The toggle sweep group of a sweeping toggle (its flow's id), 0 otherwise.
    pub(crate) tag: u64,
    /// The component is a Tab stop (`Component::keys`): ui-rt focuses it on a press and falls back
    /// to it when focus inside it goes.
    pub(crate) stop: bool,
    /// The component takes text input while focused (`Component::text_input`): ui-rt runs a
    /// platform IME session for it.
    pub(crate) text: Option<ImePurpose>,
}

impl Element for ComponentNode {
    type Output = ();
    type Local = Kept;

    fn props_key(&self) -> u64 {
        self.key
    }

    fn init(&self, _: &mut craft_ui_rt::BuildCx<'_>) -> Kept {
        Kept::new()
    }

    fn layout(&self, kept: &mut Kept, cx: &mut LayoutCx<'_>, _: Constraints) -> Vec2 {
        kept.place_children(cx, Vec2::ZERO);
        kept.size
    }

    /// Its size is decided in the build.
    fn relayout_boundary(&self) -> bool {
        true
    }

    fn tag(&self) -> u64 {
        self.tag
    }

    fn focusable(&self) -> bool {
        self.stop
    }

    fn text_input(&self) -> Option<ImePurpose> {
        self.text
    }

    /// The caret rect the last run reported (`Cx::ime_cursor`), in the node's coordinates.
    fn ime_cursor(&self, kept: &Kept) -> Option<Rect> {
        kept.ime.caret()
    }

    fn ime_surrounding(&self, kept: &Kept, since: Option<u64>) -> Option<craft_ui_rt::ImeSurrounding> {
        kept.ime.surrounding_since(since)
    }

    /// Shows the kept looks for the node's pointer state (reading it only when the variants differ),
    /// fades between them and tweens the node's opacity at the frame's time ([`super::motion`]), and
    /// re-records the kept hit shapes: the node's own, then its parts above it.
    fn paint(&self, kept: &Kept, cx: &mut PaintCx<'_>) {
        // Each fact is read only when a look for it was recorded, so a press does not repaint a node
        // that keeps no press look, and a single look reads nothing.
        let has = |look: Pointer| kept.variants.iter().any(|v| v.look == look);
        let look = if has(Pointer::Press) && cx.interaction_fact(InteractFact::Pressed) {
            Pointer::Press
        } else if has(Pointer::Hover) && cx.interaction_fact(InteractFact::Hovered) {
            Pointer::Hover
        } else {
            Pointer::Rest
        };
        super::motion::paint_level(kept, cx);
        super::motion::paint_looks(kept, cx, look);
        if !kept.members.is_empty() {
            // Member rows: each row's sub-list for the pointer's place; the read flips only when the
            // hovered row changes, and the press is read only while a row is hovered.
            let hovered = cx.hovered_part();
            let pressed = hovered.is_some() && cx.interaction_fact(InteractFact::Pressed);
            for list in super::items::member_lists(&kept.members, hovered, pressed) {
                cx.extend(list);
            }
        }
        if let Some(edit) = &kept.ime.edit {
            edit.paint(cx);
        }
        super::cursor::record_own_shape(kept, cx);
        if kept.close_guard {
            cx.close_guard();
        }
        for p in &kept.parts {
            if let Some(hit) = p.exclusive {
                cx.window_zone(p.shape, super::windows::zone_tag(hit));
            }
            match p.cursor {
                Some(cursor) => cx.hit_cursor(p.shape, p.sense, p.id, cursor),
                None => cx.hit(p.shape, p.sense, p.id),
            }
        }
        if kept.drop.is_some() {
            let bounds = cx.bounds();
            cx.drop_target(HitShape::Rect(bounds), 0);
        }
        // The ring reads only the focus-visible fact: a focus move repaints the node, a hover does not.
        if let Some(ring) = kept.focus.ring
            && cx.interaction_fact(InteractFact::FocusVisible)
        {
            let bounds = cx.bounds();
            kept.focus.ring_shape.draw(cx, ring, bounds);
        }
    }

    /// Queues what the next run consumes and marks the node, which marks the path to its window
    /// root. A press's `Down` is queued without marking unless the part it went down on sets its
    /// value on press. Keys and focus requests go through the key stage and the focus service.
    fn event(&self, kept: &mut Kept, cx: &mut EventCx<'_>, ev: &Event) -> Handled {
        match ev {
            Event::Focus(on) => {
                if self.stop {
                    kept.queue.push(NodeInput::Focus(*on));
                }
                if *on {
                    kept.focus.took_focus();
                } else {
                    super::ime::focus_left(kept, cx);
                }
                Handled::Yes
            }
            Event::Text(_) | Event::ImePreedit { .. } | Event::ImeCommit(_) | Event::Paste(_) => super::ime::event(self.text.is_some(), kept, cx, ev),
            Event::Key(k) => self.key(kept, cx, k),
            Event::Access(a) if a.target == cx.node() && a.action == accesskit::Action::Focus && a.sub.is_none() => super::focus::request(kept, cx),
            Event::Hover(on) => {
                if let Some(tip) = kept.tip.as_mut() {
                    tip.hover(*on, cx);
                }
                Handled::Yes
            }
            Event::CloseRequested(reason) => {
                kept.queue.push(NodeInput::Close(*reason));
                cx.request_build();
                Handled::Yes
            }
            Event::DragEnd(outcome) => {
                kept.drag_session = false;
                kept.queue.push(NodeInput::DragEnd(*outcome));
                cx.request_build();
                Handled::Yes
            }
            Event::Drop(d) => {
                let Some(rule) = kept.drop else { return Handled::No };
                match d.phase {
                    DropPhase::Over if rule.accepts(&d.data) => cx.accept_drop(rule.op),
                    DropPhase::Over => Handled::No,
                    DropPhase::Drop => {
                        kept.queue.push(NodeInput::Drop(d.data.clone()));
                        cx.request_build();
                        Handled::Yes
                    }
                    _ => Handled::Yes,
                }
            }
            Event::Wheel(w) if kept.takes_wheel() => {
                kept.queue.push(NodeInput::Wheel(wheel_of(w, kept.notch, kept.size)));
                cx.request_build();
                Handled::Yes
            }
            Event::Pointer(p) if kept.takes_press() => {
                self.pointer(kept, cx, p);
                Handled::Yes
            }
            Event::Access(a) if a.target == cx.node() && !matches!(a.action, accesskit::Action::Focus | accesskit::Action::ScrollIntoView) => {
                if a.action == accesskit::Action::Click && !kept.takes_press() && a.sub.is_none() {
                    return Handled::No;
                }
                let action = super::access::AccessAction { action: a.action, data: a.data.clone(), sub: a.sub };
                let input = crate::runtime::access::text::selection_input(&kept.ime.text_runs, &action);
                kept.queue.push(input.unwrap_or(NodeInput::Access(action)));
                cx.request_build();
                Handled::Yes
            }
            _ => Handled::No,
        }
    }

    fn access(&self, kept: &Kept, cx: &mut AccessCx<'_>) -> Option<accesskit::Node> {
        for s in &kept.access_subs {
            let mut sub = s.node.clone();
            kept.resolve_relations(&mut sub);
            cx.sub(s.key, s.parent, sub);
        }
        let mut node = kept.access.clone()?;
        kept.resolve_relations(&mut node);
        Some(node)
    }
}

impl ComponentNode {
    /// A key on this node's path: taken when the node is focused and its kept `KeyUse` says so,
    /// then by its roving group or its trapping scope.
    fn key(&self, kept: &mut Kept, cx: &mut EventCx<'_>, k: &craft_ui_rt::KeyInput) -> Handled {
        if cx.target() == cx.node() {
            // While the input method composes, its keys are the composition's.
            if kept.ime.composing() {
                return Handled::Yes;
            }
            match super::keys::take(&kept.focus, cx, k) {
                Taken::Queue(input) => {
                    kept.queue.push(input);
                    cx.request_build();
                    return Handled::Yes;
                }
                Taken::Held => return Handled::Yes,
                Taken::No => {}
            }
            return super::focus::trap_tab(kept, false, cx, k);
        }
        match super::focus::rove(kept, cx, k) {
            Handled::No => super::focus::trap_tab(kept, false, cx, k),
            handled => handled,
        }
    }

    /// The press machine's phases on this node. `Down` takes the press; a sweeping toggle's drag
    /// runs the sweep instead of queuing moves.
    fn pointer(&self, kept: &mut Kept, cx: &mut EventCx<'_>, p: &PointerInput) {
        let pos = cx.to_local(p.pos);
        let part = p.hit.filter(|h| h.node == cx.node()).map_or(0, |h| h.part);
        let start = match p.gesture {
            Gesture::Down { clicks } => {
                let start = PressStart::new(pos, p.button, cx.time(), clicks);
                kept.press = Some(start);
                kept.press_part = part;
                start
            }
            _ => kept.press.unwrap_or_else(|| PressStart::new(pos, p.button, cx.time(), 1)),
        };
        match p.gesture {
            Gesture::Down { .. } => {
                super::ime::pressed(kept, cx);
                if let Some(tip) = kept.tip.as_mut() {
                    tip.pressed(cx);
                }
            }
            Gesture::DragStart { .. } if kept.drag.is_some() => {
                let tokens = cx.env().tokens;
                let tokens = tokens.get(cx);
                if let Some(source) = &kept.drag {
                    kept.drag_session = cx.start_drag(source.start(&tokens));
                }
            }
            // The session moves the preview; the source does not run per move.
            Gesture::Drag { .. } if kept.drag_session => return,
            _ => {}
        }
        if super::sweep::on_pointer(self.tag, kept, cx, p) {
            return;
        }
        let marks = match p.gesture {
            Gesture::Down { .. } => kept.sense_of(part).contains(Sense::SETS_ON_PRESS),
            Gesture::Up { .. } | Gesture::Cancel(_) => {
                kept.press = None;
                true
            }
            Gesture::DragStart { .. } | Gesture::Drag { .. } | Gesture::LongPress => true,
        };
        kept.queue.push(NodeInput::Pointer { gesture: p.gesture, pos, button: p.button, mods: p.mods, part, start });
        if marks {
            cx.request_build();
        }
    }
}

/// A scope: the window root's content, a `MemoView`. It keeps its returned value and its children's
/// places; it paints nothing and has no access node of its own.
pub(crate) struct ScopeNode {
    pub(crate) key: u64,
}

impl Element for ScopeNode {
    type Output = ();
    type Local = Kept;

    fn props_key(&self) -> u64 {
        self.key
    }

    fn init(&self, _: &mut craft_ui_rt::BuildCx<'_>) -> Kept {
        Kept::new()
    }

    fn layout(&self, kept: &mut Kept, cx: &mut LayoutCx<'_>, _: Constraints) -> Vec2 {
        kept.place_children(cx, Vec2::ZERO);
        kept.size
    }

    fn relayout_boundary(&self) -> bool {
        true
    }

    /// The window root is the last claimant on every key's path: Tab and F6, then the command the
    /// key is bound to.
    fn event(&self, kept: &mut Kept, cx: &mut EventCx<'_>, ev: &Event) -> Handled {
        match ev {
            Event::Key(k) if kept.window_root => match super::focus::window_keys(kept, cx, k) {
                Handled::No => super::commands::window_command(kept, cx, k),
                handled => handled,
            },
            _ => Handled::No,
        }
    }
}

/// A wheel step as the node's run reads it (`Pointer::take_wheel`): line steps in points of the
/// node's notch, a two-finger gesture as touch pan, zoom and rotation.
fn wheel_of(w: &craft_ui_rt::WheelInput, notch: craft_behavior::WheelNotch, extent: Vec2) -> craft_behavior::Wheel {
    let (scroll, lines) = match w.unit {
        craft_behavior::WheelUnit::Line => (Vec2::new(w.delta.x * notch.points(extent.x), w.delta.y * notch.points(extent.y)), w.delta),
        _ => (w.delta, Vec2::ZERO),
    };
    let mut wheel = craft_behavior::Wheel::new(if w.touch { Vec2::ZERO } else { scroll }, w.mods, w.unit, w.phase);
    wheel.lines = lines;
    wheel.source = w.source;
    wheel.at = Some(w.pos);
    if w.touch {
        wheel.touch_pan = w.delta;
        wheel.touch_zoom = w.zoom;
        wheel.touch_rotation = w.rotation;
    } else {
        wheel.zoom = w.zoom;
        wheel.rotation = w.rotation;
    }
    wheel
}
