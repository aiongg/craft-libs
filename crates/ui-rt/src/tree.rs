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
    /// Taken out while the node's own build, layout or paint runs.
    pub(crate) element: Option<Box<dyn AnyElement>>,
    pub(crate) props_key: u64,
    pub(crate) dirty: Dirty,
    /// The constraints of the last layout; `None` before the first.
    pub(crate) constraints: Option<Constraints>,
    /// A relayout inside this node stops here.
    pub(crate) boundary: bool,
    pub(crate) frame: FrameNode,
    /// Window-space ink of both display lists at the last frame that changed this node, for
    /// damage.
    pub(crate) abs: Option<Rect>,
    /// Images and surfaces its display lists show, with their local rects.
    pub(crate) media: SmallVec<[(Media, Rect); 1]>,
    /// Cells this node created; freed with it.
    pub(crate) owned: Vec<(u32, u32)>,
    pub(crate) outbox: Option<Box<dyn Any>>,
    /// Recorded by the last paint, in local coordinates, in paint order.
    pub(crate) hits: Vec<HitRecord>,
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
            element,
            props_key,
            dirty: Dirty::default(),
            constraints: None,
            boundary: parent.is_none(),
            frame: FrameNode::empty(),
            abs: None,
            media: SmallVec::new(),
            owned: Vec::new(),
            outbox: None,
            hits: Vec::new(),
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

    pub(crate) fn mark_build(&mut self, id: NodeId) {
        if let Some(n) = self.nodes.get_mut(&id)
            && !n.is_fixed()
            && !n.dirty.has(Dirty::BUILD)
        {
            n.dirty.insert(Dirty::BUILD);
            self.build_q.push(id);
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
