//! The retained half of contract content: `ComponentNode` and `ScopeNode`, the ui-rt elements that
//! keep what the last run produced (docs/contract-rt.md sections 2 and 5).

use std::any::Any;

use craft_ui_rt::{
    AccessCx, Constraints, DisplayList, Element, Event, EventCx, Gesture, Handled, HitShape, LayoutCx, Mods, NodeId, PaintCx, PointerButton, Pos2,
    Sense, Vec2,
};
use egui::Id;

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
    /// A press phase after its `Down` (which the retained half answers alone: it takes the press,
    /// and the press look is paint-only).
    Pointer {
        /// The phase.
        gesture: Gesture,
        /// Where the pointer is, in the node's coordinates.
        pos: Pos2,
        /// The button that pressed.
        button: PointerButton,
        /// The modifiers held.
        mods: Mods,
        /// The part of the node the press went down on.
        part: u64,
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

/// What the last run of a node produced; the `Local` of both node elements.
pub(crate) struct Kept {
    /// The craft `Id` services key by.
    pub(crate) id: Id,
    /// The component key of the last run; `None` before the first and for unkeyed types.
    pub(crate) key: Option<u128>,
    pub(crate) place: Place,
    pub(crate) size: Vec2,
    pub(crate) sense: Sense,
    /// Empty until the first run; one entry when the paint read no pointer state.
    pub(crate) variants: smallvec::SmallVec<[Variant; 1]>,
    pub(crate) access: Option<accesskit::Node>,
    pub(crate) queue: Vec<NodeInput>,
    pub(crate) placed: Vec<Placed>,
    /// A scope's returned value.
    pub(crate) value: Option<Box<dyn Any>>,
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
            access: None,
            queue: Vec::new(),
            placed: Vec::new(),
            value: None,
        }
    }

    /// Places the children where the last build laid them out; a child keeps its size.
    fn place_children(&self, cx: &mut LayoutCx<'_>) {
        for p in &self.placed {
            cx.layout_child(p.node, Constraints::tight(p.size));
            cx.place_child(p.node, p.offset);
        }
    }
}

/// A shown component (leaf or container). Its hooks run in its owner's build; this element replays
/// their outputs in ui-rt's phases.
pub(crate) struct ComponentNode {
    pub(crate) key: u64,
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

    /// Picks the variant for the node's pointer state (reading it only when the variants differ)
    /// and re-records the kept hit shape.
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
    }

    /// Queues what the next run consumes and marks the node, which marks the path to its window
    /// root. A press's `Down` only takes the press.
    fn event(&self, kept: &mut Kept, cx: &mut EventCx<'_>, ev: &Event) -> Handled {
        if !kept.sense.takes_press() {
            return Handled::No;
        }
        match ev {
            Event::Pointer(p) => {
                if !matches!(p.gesture, Gesture::Down { .. }) {
                    let part = p.hit.map_or(0, |h| h.part);
                    kept.queue.push(NodeInput::Pointer { gesture: p.gesture, pos: cx.to_local(p.pos), button: p.button, mods: p.mods, part });
                    cx.request_build();
                }
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
