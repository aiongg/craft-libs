use std::any::Any;

use emath::Rect;
use smallvec::SmallVec;

use crate::access::AccessData;
use crate::display::Media;
use crate::element::AnyElement;
use crate::frame::FrameNode;
use crate::geom::Constraints;
use crate::hit::HitRecord;
use crate::id::{IdMap, NodeId, WindowId};
use crate::state::Phase;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Dirty(u8);

impl Dirty {
    pub(crate) const BUILD: Dirty = Dirty(1);
    pub(crate) const LAYOUT: Dirty = Dirty(2);
    pub(crate) const PAINT: Dirty = Dirty(4);
    /// On an inline node: a node it owns, directly or further down, carries `BUILD`, so the build
    /// of its owner must reach it.
    pub(crate) const PATH: Dirty = Dirty(8);

    pub(crate) fn has(self, d: Dirty) -> bool {
        self.0 & d.0 != 0
    }

    pub(crate) fn insert(&mut self, d: Dirty) {
        self.0 |= d.0;
    }

    pub(crate) fn remove(&mut self, d: Dirty) {
        self.0 &= !d.0;
    }
}

/// What a node is to the runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Role {
    /// An element's node.
    Element,
    /// A window's root: its children are the layers, bottom first.
    WindowRoot,
    /// One layer of a window; its children are the app root (content layer) or the layer
    /// children nodes put there ([`crate::BuildCx::layer`]), oldest first.
    Layer(crate::layer::Layer),
}

pub(crate) struct Node {
    /// The node it paints and lays out under.
    pub(crate) parent: Option<NodeId>,
    /// The node whose build created it and gets its output: the parent, or for a layer child the
    /// node that put it on the layer. `None` for fixed nodes and the app root.
    pub(crate) owner: Option<NodeId>,
    pub(crate) window: WindowId,
    pub(crate) role: Role,
    /// Build order: the owner's depth plus one, so an owner builds before what it owns.
    pub(crate) depth: u32,
    pub(crate) children: Vec<NodeId>,
    /// Children it put on layers, in build order.
    pub(crate) layered: Vec<NodeId>,
    /// Its children are created by its owner's build ([`crate::BuildCx::inline`]), never by its own
    /// `Element::build`. An inline node with no owner is an inline root: the host passes its content
    /// per frame ([`crate::Runtime::frame_with`]).
    pub(crate) inline: bool,
    /// It gets [`crate::Event::Hover`] when the pointer enters or leaves it
    /// ([`crate::BuildCx::hover_events`]).
    pub(crate) hover_events: bool,
    /// Its inline build is running: a node it owns is being built inside it ([`crate::BuildCx::is_building`]).
    pub(crate) building: bool,
    /// Taken out while the node's own build, layout or paint runs.
    pub(crate) element: Option<Box<dyn AnyElement>>,
    pub(crate) props_key: u64,
    pub(crate) dirty: Dirty,
    /// The constraints of the last layout; `None` before the first.
    pub(crate) constraints: Option<Constraints>,
    /// A relayout inside this node stops here.
    pub(crate) boundary: bool,
    /// The texts its last layout shaped, in call order, with the scale: a layout that asks for the
    /// same text again gets the shaped one back.
    pub(crate) texts: SmallVec<[crate::layout::ShapedText; 2]>,
    pub(crate) frame: FrameNode,
    /// Window-space ink of both display lists at the last frame that changed this node, for
    /// damage.
    pub(crate) abs: Option<Rect>,
    /// Images and surfaces its display lists show, with their local rects.
    pub(crate) media: SmallVec<[(Media, Rect); 1]>,
    /// Cells this node created; freed with it.
    pub(crate) owned: Vec<(u32, u32)>,
    /// Cells created by key ([`crate::BuildCx::slot`]), owned like `owned`.
    pub(crate) slots: Vec<(u64, (u32, u32))>,
    /// Values this node provides to itself and what it owns ([`crate::BuildCx::provide`]).
    pub(crate) provided: Vec<(std::any::TypeId, std::rc::Rc<dyn Any>)>,
    pub(crate) outbox: Option<Box<dyn Any>>,
    /// The items of its paint slots by key, in local coordinates ([`crate::BuildCx::fill_slot`]): kept while its
    /// display lists reserve the slot, put in place of each [`crate::Item::Slot`] when it paints.
    pub(crate) slot_lists: Vec<(u64, std::sync::Arc<crate::display::DisplayList>)>,
    /// Recorded by the last paint, in local coordinates, in paint order.
    pub(crate) hits: Vec<HitRecord>,
    /// The cursor its last paint named for itself and its descendants.
    pub(crate) cursor: Option<crate::platform::Cursor>,
    pub(crate) access: AccessData,
}

impl Node {
    pub(crate) fn new(
        parent: Option<NodeId>,
        owner: Option<NodeId>,
        window: WindowId,
        depth: u32,
        element: Option<Box<dyn AnyElement>>,
        props_key: u64,
    ) -> Node {
        Node {
            parent,
            owner,
            window,
            role: Role::Element,
            depth,
            children: Vec::new(),
            layered: Vec::new(),
            inline: false,
            hover_events: false,
            building: false,
            element,
            props_key,
            dirty: Dirty::default(),
            constraints: None,
            texts: SmallVec::new(),
            boundary: parent.is_none(),
            frame: FrameNode::empty(),
            abs: None,
            media: SmallVec::new(),
            owned: Vec::new(),
            slots: Vec::new(),
            provided: Vec::new(),
            outbox: None,
            slot_lists: Vec::new(),
            hits: Vec::new(),
            cursor: None,
            access: AccessData::default(),
        }
    }

    /// The window's root and layer nodes: the runtime sizes and fills them; they never build.
    pub(crate) fn is_fixed(&self) -> bool {
        self.role != Role::Element
    }
}

/// The persistent node tree of every window, with its dirty queues.
#[derive(Default)]
pub(crate) struct Tree {
    pub(crate) nodes: IdMap<Node>,
    pub(crate) build_q: Vec<NodeId>,
    /// Relayout boundaries with a dirty node at or below them.
    pub(crate) layout_q: Vec<NodeId>,
    pub(crate) paint_q: Vec<NodeId>,
}

impl Tree {
    /// The nearest value of type `T` provided by `node` or the nodes above it (the owner of a
    /// layer child, else the parent).
    pub(crate) fn provided<T: Clone + 'static>(&self, node: NodeId) -> Option<T> {
        let want = std::any::TypeId::of::<T>();
        let mut cur = Some(node);
        while let Some(id) = cur {
            let n = self.nodes.get(&id)?;
            if let Some((_, v)) = n.provided.iter().find(|(t, _)| *t == want) {
                return v.downcast_ref::<T>().cloned();
            }
            cur = n.owner.or(n.parent);
        }
        None
    }

    /// The nearest value of type `T` provided by a node above `node` (its owner, else its parent, and on up),
    /// passing over what `node` provides itself.
    pub(crate) fn provided_above<T: Clone + 'static>(&self, node: NodeId) -> Option<T> {
        let n = self.nodes.get(&node)?;
        self.provided::<T>(n.owner.or(n.parent)?)
    }

    pub(crate) fn has_work(&self) -> bool {
        !(self.build_q.is_empty() && self.layout_q.is_empty() && self.paint_q.is_empty())
    }

    pub(crate) fn mark(&mut self, id: NodeId, phase: Phase) {
        match phase {
            Phase::Build => self.mark_build(id),
            Phase::Layout => self.mark_layout(id),
            Phase::Paint => self.mark_paint(id),
        }
    }

    /// Queues `id`'s build. An inline node cannot build alone: it takes `BUILD`, each inline owner
    /// above it takes `PATH`, and the build is queued on the first owner that can run, the inline
    /// root (or a plain element, which rebuilds).
    pub(crate) fn mark_build(&mut self, id: NodeId) {
        let Some(n) = self.nodes.get_mut(&id) else { return };
        if n.is_fixed() {
            return;
        }
        if !n.inline || n.owner.is_none() {
            if !n.dirty.has(Dirty::BUILD) {
                let queued = n.dirty.has(Dirty::PATH);
                n.dirty.insert(Dirty::BUILD);
                if !queued {
                    self.build_q.push(id);
                }
            }
            return;
        }
        n.dirty.insert(Dirty::BUILD);
        let mut owner = n.owner;
        while let Some(o) = owner {
            let Some(n) = self.nodes.get_mut(&o) else { return };
            if !n.inline {
                self.mark_build(o);
                return;
            }
            let queued = n.dirty.has(Dirty::BUILD) || n.dirty.has(Dirty::PATH);
            n.dirty.insert(Dirty::PATH);
            if n.owner.is_none() {
                if !queued {
                    self.build_q.push(o);
                }
                return;
            }
            owner = n.owner;
        }
    }

    /// Marks `id` and its ancestors up to the nearest relayout boundary, and queues the boundary.
    /// A node already marked has its chain marked, so the climb stops there.
    pub(crate) fn mark_layout(&mut self, mut id: NodeId) {
        loop {
            let Some(n) = self.nodes.get_mut(&id) else {
                return;
            };
            if n.is_fixed() || n.dirty.has(Dirty::LAYOUT) {
                return;
            }
            n.dirty.insert(Dirty::LAYOUT);
            match n.parent {
                Some(parent) if !n.boundary => id = parent,
                _ => {
                    self.layout_q.push(id);
                    return;
                }
            }
        }
    }

    pub(crate) fn mark_paint(&mut self, id: NodeId) {
        if let Some(n) = self.nodes.get_mut(&id)
            && !n.is_fixed()
            && !n.dirty.has(Dirty::PAINT)
        {
            n.dirty.insert(Dirty::PAINT);
            self.paint_q.push(id);
        }
    }

    /// `ids` deduplicated and sorted owners first.
    pub(crate) fn by_depth(&self, mut ids: Vec<NodeId>) -> Vec<NodeId> {
        ids.sort_by_key(|id| (self.nodes.get(id).map_or(u32::MAX, |n| n.depth), *id));
        ids.dedup();
        ids
    }

    pub(crate) fn depth(&self, id: NodeId) -> u32 {
        self.nodes.get(&id).map_or(0, |n| n.depth)
    }

    pub(crate) fn window_of(&self, id: NodeId) -> Option<WindowId> {
        self.nodes.get(&id).map(|n| n.window)
    }

    /// The node that gets `id`'s output: its owner.
    pub(crate) fn owner(&self, id: NodeId) -> Option<NodeId> {
        self.nodes.get(&id).and_then(|n| n.owner)
    }

    /// `id` and the nodes that own it, nearest first: through layer children to the node that put
    /// them there. A press inside a submenu is inside its menu by this chain.
    pub(crate) fn owner_chain(&self, id: NodeId) -> SmallVec<[NodeId; 16]> {
        let mut out = SmallVec::new();
        let mut cur = Some(id);
        while let Some(c) = cur {
            let Some(n) = self.nodes.get(&c) else { break };
            out.push(c);
            cur = n.owner.or(n.parent);
        }
        out
    }
}
