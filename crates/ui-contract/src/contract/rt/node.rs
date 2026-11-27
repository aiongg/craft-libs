//! The retained half of contract content: `ComponentNode` and `ScopeNode`, the ui-rt elements that
//! keep what the last run produced (docs/contract-rt.md sections 2 and 5).

use std::any::{Any, TypeId};

use craft_behavior::{PartRef, PressStart, WheelNotch, WheelUnit};
use craft_ui_rt::{
    AccessCx, Constraints, Cursor, DisplayList, Element, Event, EventCx, Gesture, Handled, HitShape, LayoutCx, Mods, NodeId, PaintCx, PointerButton,
    PointerInput, Pos2, Sense, State, Vec2, WheelInput,
};
use egui::Id;

use super::sweep::Sweep;
use crate::runtime::memory::Slot;

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
    /// An assistive technology's action on the node.
    Access(accesskit::Action),
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

/// A scrolling container's offset and its largest value; the wheel moves the offset in the
/// retained half alone.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Scroll {
    pub(crate) offset: Vec2,
    pub(crate) max: Vec2,
}

/// A node's memory slots, freed with the node.
#[derive(Default)]
pub(crate) struct Memory(Vec<(TypeId, Id, Box<dyn Any>)>);

impl Memory {
    /// The slot of type `T` under `id`, created with its default on first use.
    pub(crate) fn shown<T: Slot>(&mut self, id: Id) -> &mut T {
        let at = match self.0.iter().position(|(t, i, _)| *t == TypeId::of::<T>() && *i == id) {
            Some(at) => at,
            None => {
                self.0.push((TypeId::of::<T>(), id, Box::new(T::default())));
                self.0.len() - 1
            }
        };
        self.0[at].2.downcast_mut::<T>().expect("a slot holds the type it was created with")
    }

    /// A copy of the slot of type `T` under `id`, or its default.
    pub(crate) fn recall<T: Slot>(&self, id: Id) -> T {
        self.0.iter().find(|(t, i, _)| *t == TypeId::of::<T>() && *i == id).and_then(|(_, _, v)| v.downcast_ref::<T>()).cloned().unwrap_or_default()
    }
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
    pub(crate) queue: Vec<NodeInput>,
    pub(crate) placed: Vec<Placed>,
    /// A scope's returned value.
    pub(crate) value: Option<Box<dyn Any>>,
    pub(crate) memory: Memory,
    pub(crate) scroll: Option<Scroll>,
    /// The window's toggle sweep record: the window root creates it, every run copies it from the
    /// flow it was shown in.
    pub(crate) sweep: Option<State<Sweep>>,
    /// The value a sweeping toggle showed at its last run (`Component::sweep_value`).
    pub(crate) toggle: Option<bool>,
    /// The press the node owns, from its `Down` to its release or cancel.
    pub(crate) press: Option<PressStart>,
    /// A drag of the node's press runs, as of the inputs its runs consumed.
    pub(crate) dragging: bool,
}

impl Kept {
    fn new() -> Kept {
        Kept {
            id: Id::NULL,
            key: None,
            place: Place::default(),
            size: Vec2::ZERO,
            sense: Sense::empty(),
            variants: smallvec::SmallVec::new(),
            parts: Vec::new(),
            access: None,
            queue: Vec::new(),
            placed: Vec::new(),
            value: None,
            memory: Memory::default(),
            scroll: None,
            sweep: None,
            toggle: None,
            press: None,
            dragging: false,
        }
    }

    /// Places the children where the last build laid them out, moved by the scroll offset; a child
    /// keeps its size.
    fn place_children(&self, cx: &mut LayoutCx<'_>) {
        let scroll = self.scroll.map_or(Vec2::ZERO, |s| s.offset);
        for p in &self.placed {
            cx.layout_child(p.node, Constraints::tight(p.size));
            cx.place_child(p.node, p.offset - scroll);
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
        kept.place_children(cx);
        kept.size
    }

    /// Its size is decided in the build.
    fn relayout_boundary(&self) -> bool {
        true
    }

    fn tag(&self) -> u64 {
        self.tag
    }

    /// Picks the variant for the node's pointer state (reading it only when the variants differ)
    /// and re-records the kept hit shapes: the node's own, then its parts above it.
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
        if let Some(v) = kept.variants.iter().find(|v| v.look == look).or_else(|| kept.variants.first()) {
            cx.extend(&v.content);
            if !v.overlay.is_empty() {
                cx.over(|cx| cx.extend(&v.overlay));
            }
        }
        if !kept.sense.is_empty() {
            let bounds = cx.bounds();
            cx.hit(HitShape::Rect(bounds), kept.sense, 0);
        }
        for p in &kept.parts {
            match p.cursor {
                Some(cursor) => cx.hit_cursor(p.shape, p.sense, p.id, cursor),
                None => cx.hit(p.shape, p.sense, p.id),
            }
        }
    }

    /// Queues what the next run consumes and marks the node, which marks the path to its window
    /// root. The wheel on a scrolling container moves its content here alone.
    fn event(&self, kept: &mut Kept, cx: &mut EventCx<'_>, ev: &Event) -> Handled {
        match ev {
            Event::Wheel(w) if kept.scroll.is_some() => scroll(kept, cx, w),
            Event::Pointer(p) if kept.takes_press() => {
                self.pointer(kept, cx, p);
                Handled::Yes
            }
            Event::Access(a) if a.target == cx.node() && a.action == accesskit::Action::Click => {
                kept.queue.push(NodeInput::Access(a.action));
                cx.request_build();
                Handled::Yes
            }
            _ => Handled::No,
        }
    }

    fn access(&self, kept: &Kept, _: &mut AccessCx<'_>) -> Option<accesskit::Node> {
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

/// The scroll distance of one wheel line before the wheel service (B9) converts per taker:
/// `WheelNotch::DEFAULT`.
const LINE: WheelNotch = WheelNotch::DEFAULT;

/// Moves a scrolling container's offset by a wheel step and lays its children out again: no run.
/// A step that cannot move the offset bubbles to the container's ancestors.
fn scroll(kept: &mut Kept, cx: &mut EventCx<'_>, w: &WheelInput) -> Handled {
    let (Some(s), false) = (kept.scroll.as_mut(), w.pinches()) else {
        return Handled::No;
    };
    let points = match w.unit {
        WheelUnit::Line => w.delta * LINE.points(kept.size.y),
        WheelUnit::Page => w.delta * kept.size.y,
        _ => w.delta,
    };
    let next = (s.offset - points).clamp(Vec2::ZERO, s.max);
    if next == s.offset {
        return Handled::No;
    }
    s.offset = next;
    cx.request_layout();
    Handled::Yes
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
        kept.place_children(cx);
        kept.size
    }

    fn relayout_boundary(&self) -> bool {
        true
    }
}
