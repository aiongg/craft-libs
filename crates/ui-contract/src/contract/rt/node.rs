//! The retained half of contract content: `ComponentNode` and `ScopeNode`, the ui-rt elements that
//! keep what the last run produced (docs/contract-rt.md sections 2 and 5).

use std::any::Any;

use craft_behavior::{PartRef, PressStart};
use craft_ui_rt::{
    AccessCx, Constraints, Cursor, DisplayList, DragData, DragOutcome, DropPhase, Element, Event, EventCx, Gesture, Handled, HitShape, LayoutCx,
    Mods, NodeId, PaintCx, PointerButton, PointerInput, Pos2, Sense, State, Vec2,
};

use egui::Id;

use super::drag::{DragSource, DropRule};
use super::memory::Memory;
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
    /// Whether the last real run of the body under this node met fill children; `None` before it.
    pub(crate) body_fills: Option<bool>,
    pub(crate) memory: Memory,
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
    /// The env cells the node's window root provided (window root only).
    pub(crate) env: Option<super::env::EnvCells>,
    /// The cursor over the node's own shape, from `Component::cursor`.
    pub(crate) cursor: Option<Cursor>,
    /// Paint-time state: the fade between pointer looks.
    pub(crate) fade: super::motion::FadeCell,
    /// The opacity a run aimed at and its tween, evaluated at paint time.
    pub(crate) level: std::cell::RefCell<Option<super::motion::Level>>,
}

impl Kept {
    pub(crate) fn new() -> Kept {
        Kept {
            id: Id::NULL,
            key: None,
            place: Place::default(),
            size: Vec2::ZERO,
            sense: Sense::empty(),
            variants: smallvec::SmallVec::new(),
            parts: Vec::new(),
            access: None,
            access_subs: Vec::new(),
            queue: Vec::new(),
            placed: Vec::new(),
            value: None,
            sizes: smallvec::SmallVec::new(),
            plan: None,
            body_fills: None,
            memory: Memory::default(),
            sweep: None,
            toggle: None,
            press: None,
            popups: PopupStates::new(),
            tip: None,
            drag: None,
            drop: None,
            drag_session: false,
            dragging: false,
            env: None,
            cursor: None,
            fade: Default::default(),
            level: Default::default(),
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
}

/// A shown component (leaf or container). Its hooks run in its owner's build; this element replays
/// their outputs in ui-rt's phases.
pub(crate) struct ComponentNode {
    pub(crate) key: u64,
    /// The toggle sweep group of a sweeping toggle (its flow's id), 0 otherwise.
    pub(crate) tag: u64,
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

    /// Shows the kept looks for the node's pointer state (reading it only when the variants differ),
    /// fades between them and tweens the node's opacity at the frame's time ([`super::motion`]), and
    /// re-records the kept hit shapes: the node's own, then its parts above it.
    fn paint(&self, kept: &Kept, cx: &mut PaintCx<'_>) {
        let look = if kept.variants.len() > 1 {
            let state = cx.interaction();
            if state.pressed {
                Pointer::Press
            } else if state.hovered {
                Pointer::Hover
            } else {
                Pointer::Rest
            }
        } else {
            Pointer::Rest
        };
        super::motion::paint_level(kept, cx);
        super::motion::paint_looks(kept, cx, look);
        super::cursor::record_own_shape(kept, cx);
        for p in &kept.parts {
            match p.cursor {
                Some(cursor) => cx.hit_cursor(p.shape, p.sense, p.id, cursor),
                None => cx.hit(p.shape, p.sense, p.id),
            }
        }
        if kept.drop.is_some() {
            let bounds = cx.bounds();
            cx.drop_target(HitShape::Rect(bounds), 0);
        }
    }

    /// Queues what the next run consumes and marks the node, which marks the path to its window
    /// root. A press's `Down` is queued without marking unless the part it went down on sets its
    /// value on press. Focus requests are the runtime's: it moves focus to the nearest stop.
    fn event(&self, kept: &mut Kept, cx: &mut EventCx<'_>, ev: &Event) -> Handled {
        match ev {
            Event::Hover(on) => {
                if let Some(tip) = kept.tip.as_mut() {
                    tip.hover(*on, cx);
                }
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
            Event::Pointer(p) if kept.takes_press() => {
                self.pointer(kept, cx, p);
                Handled::Yes
            }
            Event::Access(a) if a.target == cx.node() && a.action != accesskit::Action::Focus => {
                if a.action == accesskit::Action::Click && !kept.takes_press() && a.sub.is_none() {
                    return Handled::No;
                }
                kept.queue.push(NodeInput::Access(super::access::AccessAction { action: a.action, data: a.data.clone(), sub: a.sub }));
                cx.request_build();
                Handled::Yes
            }
            _ => Handled::No,
        }
    }

    fn access(&self, kept: &Kept, cx: &mut AccessCx<'_>) -> Option<accesskit::Node> {
        for s in &kept.access_subs {
            cx.sub(s.key, s.parent, s.node.clone());
        }
        kept.access.clone()
    }
}

impl ComponentNode {
    /// The press machine's phases on this node. `Down` takes the press; a sweeping toggle's drag
    /// runs the sweep instead of queuing moves.
    fn pointer(&self, kept: &mut Kept, cx: &mut EventCx<'_>, p: &PointerInput) {
        let pos = cx.to_local(p.pos);
        let part = p.hit.filter(|h| h.node == cx.node()).map_or(0, |h| h.part);
        let start = match p.gesture {
            Gesture::Down { clicks } => {
                let start = PressStart::new(pos, p.button, cx.time(), clicks);
                kept.press = Some(start);
                start
            }
            _ => kept.press.unwrap_or_else(|| PressStart::new(pos, p.button, cx.time(), 1)),
        };
        match p.gesture {
            Gesture::Down { .. } => {
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
}
