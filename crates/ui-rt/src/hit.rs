//! Hit shapes. A node records them while it paints, in its own coordinates, so a node that only
//! moves keeps them. Hit testing walks the retained tree from the top of the paint order down.
//!
//! Only recorded shapes take input: a layer child blocks presses to what lies under it only where
//! it records a shape that takes the press. A drag ghost records none, so it never blocks.

use craft_behavior::Sense;
use emath::{Pos2, Rect, Vec2};

use crate::frame::Part;
use crate::id::NodeId;
use crate::layer::Layer;
use crate::platform::Cursor;
use crate::tree::Tree;
use crate::window::Window;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HitShape {
    Rect(Rect),
    RoundedRect(Rect, f32),
    Circle(Pos2, f32),
}

impl HitShape {
    pub fn contains(&self, p: Pos2) -> bool {
        match *self {
            HitShape::Rect(r) => contains(r, p),
            HitShape::RoundedRect(r, radius) => {
                if !contains(r, p) {
                    return false;
                }
                let radius = radius.min(r.width() / 2.0).min(r.height() / 2.0);
                let cx = p.x.clamp(r.min.x + radius, r.max.x - radius);
                let cy = p.y.clamp(r.min.y + radius, r.max.y - radius);
                (p - Pos2::new(cx, cy)).length_sq() <= radius * radius
            }
            HitShape::Circle(c, radius) => (p - c).length_sq() <= radius * radius,
        }
    }
}

impl HitShape {
    /// The smallest rect around the shape.
    pub fn bounds(&self) -> Rect {
        match *self {
            HitShape::Rect(r) | HitShape::RoundedRect(r, _) => r,
            HitShape::Circle(c, radius) => Rect::from_center_size(c, Vec2::splat(radius * 2.0)),
        }
    }
}

impl From<Rect> for HitShape {
    fn from(r: Rect) -> HitShape {
        HitShape::Rect(r)
    }
}

/// What a recorded shape is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HitKind {
    /// Pointer, wheel and hover input, by its sense.
    Input,
    /// Where drags may drop ([`crate::PaintCx::drop_target`]).
    Drop,
    /// Where drags may drop, OS file drags also under a modal layer
    /// ([`crate::PaintCx::drop_target_under_modal`]).
    DropUnderModal,
    /// What the operating system does with a press there ([`crate::PaintCx::window_zone`]).
    Zone,
}

/// One recorded shape: what it senses and which part of its node it is (the node's own tag
/// space; 0 when the node has one part), with the cursor the pointer shows over it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct HitRecord {
    pub(crate) shape: HitShape,
    pub(crate) sense: Sense,
    pub(crate) part: u64,
    pub(crate) level: Part,
    pub(crate) kind: HitKind,
    pub(crate) cursor: Option<Cursor>,
}

/// The topmost shape under a point that passed the filter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Hit {
    pub node: NodeId,
    pub part: u64,
    pub sense: Sense,
}

/// Half-open containment, so a point on a shared edge belongs to one of two touching rects.
pub(crate) fn contains(r: Rect, p: Pos2) -> bool {
    r.min.x <= p.x && p.x < r.max.x && r.min.y <= p.y && p.y < r.max.y
}

/// The topmost shape under `pos` (window coordinates) in `window` whose sense passes `accept`.
/// Layers are tried from the top; a modal layer with children hides the layers below it. Within a
/// layer paint order decides: a node's overlay shapes, then its children from the last, then its
/// content shapes. A node's clip bounds its own shapes and its whole subtree.
pub(crate) fn hit_test(tree: &Tree, window: &Window, pos: Pos2, accept: &dyn Fn(Sense) -> bool) -> Option<Hit> {
    hit_records(tree, window, pos, &|h| h.kind == HitKind::Input && accept(h.sense)).map(|(hit, _)| hit)
}

/// The topmost recorded shape under `pos` that passes `accept`, with the record. Same order as
/// [`hit_test`].
pub(crate) fn hit_records(tree: &Tree, window: &Window, pos: Pos2, accept: &dyn Fn(&HitRecord) -> bool) -> Option<(Hit, HitRecord)> {
    hit_records_through(tree, window, pos, accept, None)
}

/// [`hit_test`], except that a shown modal layer lets the hit through to `through` and its subtree
/// (a [`crate::ModalReach`]): below the modal, the topmost shape counts only when it is `through`'s.
pub(crate) fn hit_test_through(tree: &Tree, window: &Window, pos: Pos2, accept: &dyn Fn(Sense) -> bool, through: Option<NodeId>) -> Option<Hit> {
    hit_records_through(tree, window, pos, &|h| h.kind == HitKind::Input && accept(h.sense), through).map(|(hit, _)| hit)
}

/// The topmost drop shape under `pos` that takes the drag: for an OS file drag (`files`) a shown
/// modal layer lets through only the shapes below it that take files under a modal
/// ([`crate::PaintCx::drop_target_under_modal`]); for any other drag it takes the hit.
pub(crate) fn drop_hit(tree: &Tree, window: &Window, pos: Pos2, files: bool) -> Option<Hit> {
    if !files {
        return hit_records(tree, window, pos, &|h| matches!(h.kind, HitKind::Drop | HitKind::DropUnderModal)).map(|(hit, _)| hit);
    }
    let viewport = Rect::from_min_size(Pos2::ZERO, window.viewport);
    let mut below_modal = false;
    for layer in Layer::ALL.into_iter().rev() {
        let id = window.layer(layer);
        let accept = |h: &HitRecord| h.kind == HitKind::DropUnderModal || (!below_modal && h.kind == HitKind::Drop);
        if let Some((hit, _)) = visit(tree, id, Vec2::ZERO, viewport, pos, &accept) {
            return Some(hit);
        }
        below_modal |= layer == Layer::Modal && modal_shown(tree, window);
    }
    None
}

/// Whether `window`'s modal layer holds a child: input below it is blocked.
pub(crate) fn modal_shown(tree: &Tree, window: &Window) -> bool {
    tree.nodes.get(&window.layer(Layer::Modal)).is_some_and(|n| !n.children.is_empty())
}

fn hit_records_through(
    tree: &Tree,
    window: &Window,
    pos: Pos2,
    accept: &dyn Fn(&HitRecord) -> bool,
    through: Option<NodeId>,
) -> Option<(Hit, HitRecord)> {
    let viewport = Rect::from_min_size(Pos2::ZERO, window.viewport);
    let mut blocked = false;
    for layer in Layer::ALL.into_iter().rev() {
        let id = window.layer(layer);
        if let Some(hit) = visit(tree, id, Vec2::ZERO, viewport, pos, accept) {
            return match through {
                Some(node) if blocked => tree.owner_chain(hit.0.node).contains(&node).then_some(hit),
                _ => Some(hit),
            };
        }
        if layer == Layer::Modal && tree.nodes.get(&id).is_some_and(|n| !n.children.is_empty()) {
            if through.is_none() {
                return None;
            }
            blocked = true;
        }
    }
    None
}

fn visit(
    tree: &Tree,
    id: NodeId,
    parent_origin: Vec2,
    parent_clip: Rect,
    pos: Pos2,
    accept: &dyn Fn(&HitRecord) -> bool,
) -> Option<(Hit, HitRecord)> {
    let node = tree.nodes.get(&id)?;
    let origin = parent_origin + node.frame.offset;
    let clip = match node.frame.clip {
        Some(c) => parent_clip.intersect(c.rect.translate(origin)),
        None => parent_clip,
    };
    if !contains(clip, pos) {
        return None;
    }
    let local = pos - origin;
    let own = |level: Part| {
        node.hits
            .iter()
            .rev()
            .filter(|h| h.level == level && accept(h) && h.shape.contains(local))
            .map(|h| (Hit { node: id, part: h.part, sense: h.sense }, *h))
            .next()
    };
    if let Some(hit) = own(Part::Overlay) {
        return Some(hit);
    }
    for &child in node.children.iter().rev() {
        if let Some(hit) = visit(tree, child, origin, clip, pos, accept) {
            return Some(hit);
        }
    }
    own(Part::Content)
}

/// A place in a window the operating system acts on ([`crate::PaintCx::window_zone`]), with the
/// parts of other nodes that lie over it.
#[derive(Clone, Debug, PartialEq)]
pub struct WindowZone {
    /// What the node tagged it with.
    pub tag: u64,
    /// In window coordinates.
    pub rect: Rect,
    /// The parts of other nodes painted above the zone's node that take input, clipped to the
    /// zone: a press there goes to them, not to the zone.
    pub covers: Vec<Rect>,
}

/// Every zone of `window`, with the input shapes above each: paint order is the tree's pre-order
/// with the layers in order, so a shape recorded after the zone's (by another node) is above it.
/// An ancestor is visited before its descendants and so never covers them.
pub(crate) fn window_zones(tree: &Tree, window: &Window) -> Vec<WindowZone> {
    enum Entry {
        Zone(u64, Rect, NodeId),
        Input(Rect, NodeId),
    }
    fn walk(tree: &Tree, id: NodeId, parent_origin: Vec2, out: &mut Vec<Entry>) {
        let Some(node) = tree.nodes.get(&id) else { return };
        let origin = parent_origin + node.frame.offset;
        for h in &node.hits {
            let rect = h.shape.bounds().translate(origin);
            match h.kind {
                HitKind::Zone => out.push(Entry::Zone(h.part, rect, id)),
                HitKind::Input if !h.sense.is_empty() => out.push(Entry::Input(rect, id)),
                HitKind::Input | HitKind::Drop | HitKind::DropUnderModal => {}
            }
        }
        for &child in &node.children {
            walk(tree, child, origin, out);
        }
    }
    let mut entries = Vec::new();
    for layer in Layer::ALL {
        walk(tree, window.layer(layer), Vec2::ZERO, &mut entries);
    }
    entries
        .iter()
        .enumerate()
        .filter_map(|(i, e)| match e {
            Entry::Zone(tag, rect, node) => {
                let covers = entries[i + 1..]
                    .iter()
                    .filter_map(|o| match o {
                        Entry::Input(cover, owner) if owner != node => Some(cover.intersect(*rect)),
                        _ => None,
                    })
                    .filter(|cover| cover.is_positive())
                    .collect();
                Some(WindowZone { tag: *tag, rect: *rect, covers })
            }
            Entry::Input(..) => None,
        })
        .collect()
}

/// The window origin of `id`: the sum of its and its ancestors' offsets.
pub(crate) fn window_origin(tree: &Tree, id: NodeId) -> Vec2 {
    let mut origin = Vec2::ZERO;
    let mut cur = Some(id);
    while let Some(c) = cur {
        let Some(n) = tree.nodes.get(&c) else { break };
        origin += n.frame.offset;
        cur = n.parent;
    }
    origin
}

/// `id` and its ancestors, nearest first.
pub(crate) fn chain(tree: &Tree, id: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    let mut cur = Some(id);
    while let Some(c) = cur {
        let Some(n) = tree.nodes.get(&c) else { break };
        out.push(c);
        cur = n.parent;
    }
    out
}
