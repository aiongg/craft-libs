//! The frame tree: per node an offset, a clip, an opacity and two kept display lists. It is a view
//! over the retained nodes of one window, so producing it costs nothing; renderers cache their
//! converted output by (node, version) and evict on `removed`.

use std::sync::Arc;

use emath::{Rect, Vec2};
use smallvec::SmallVec;

use crate::display::{Clip, DisplayList, ImageStore, SurfaceKey};
use crate::geom::is_empty;
use crate::id::{NodeId, WindowId};
use crate::tree::Tree;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Part {
    /// Drawn under the node's children.
    Content,
    /// Drawn over the node's children.
    Overlay,
}

#[derive(Clone, Debug)]
pub struct FrameNode {
    /// Offset from the parent's origin, in logical points.
    pub offset: Vec2,
    pub size: Vec2,
    /// Clips the node's content, children and overlay, in its own coordinates. Renderers apply
    /// the rounded corners; damage and hit testing use the rect.
    pub clip: Option<Clip>,
    pub opacity: f32,
    pub content: Arc<DisplayList>,
    pub overlay: Arc<DisplayList>,
    /// [`DisplayList::ink`] of `content` and of `overlay`, local coordinates.
    pub content_ink: Option<Rect>,
    pub overlay_ink: Option<Rect>,
    /// Moves whenever either display list is re-recorded; unique across nodes.
    pub version: u64,
}

impl FrameNode {
    pub(crate) fn empty() -> FrameNode {
        FrameNode {
            offset: Vec2::ZERO,
            size: Vec2::ZERO,
            clip: None,
            opacity: 1.0,
            content: Arc::new(DisplayList::default()),
            overlay: Arc::new(DisplayList::default()),
            content_ink: None,
            overlay_ink: None,
            version: 0,
        }
    }

    fn ink(&self) -> Option<Rect> {
        match (self.content_ink, self.overlay_ink) {
            (Some(a), Some(b)) => Some(a.union(b)),
            (a, b) => a.or(b),
        }
    }

    fn clip_rect(&self) -> Option<Rect> {
        self.clip.map(|c| c.rect)
    }
}

/// What changed in one window since its previous rendered frame.
#[derive(Clone, Debug, Default)]
pub struct FrameChanges {
    pub repainted: Vec<NodeId>,
    pub moved: Vec<NodeId>,
    /// Nodes gone since the last frame; their cached output can go.
    pub removed: Vec<NodeId>,
    /// App-drawn surfaces whose content changed ([`crate::Runtime::invalidate_surface`]).
    pub surfaces: Vec<SurfaceKey>,
    /// Every pixel needs drawing: first frame, new surface, scale or size change.
    pub full: bool,
}

impl FrameChanges {
    pub(crate) fn clear(&mut self) {
        self.repainted.clear();
        self.moved.clear();
        self.removed.clear();
        self.surfaces.clear();
        self.full = false;
    }

    pub(crate) fn any(&self) -> bool {
        self.full || !(self.repainted.is_empty() && self.moved.is_empty() && self.removed.is_empty() && self.surfaces.is_empty())
    }
}

/// One display list to draw, with everything resolved to window space.
#[derive(Clone, Copy, Debug)]
pub struct Draw<'a> {
    pub node: NodeId,
    pub part: Part,
    pub list: &'a Arc<DisplayList>,
    pub version: u64,
    /// The node's origin in window coordinates (logical points).
    pub origin: Vec2,
    /// The intersection of every clip rect above and at this node, in window coordinates.
    pub clip: Rect,
    /// The rounded clips above and at this node, nearest last, in window coordinates. Empty when
    /// no clip on the path has rounded corners.
    pub rounded: &'a [Clip],
    /// What the list can paint ([`DisplayList::ink`]) in window coordinates, cut by `clip`;
    /// never empty. No pixel outside it changes when the list is drawn.
    pub ink: Rect,
    /// The product of the opacities above and at this node.
    pub opacity: f32,
}

/// One window's frame.
pub struct FrameTree<'a> {
    pub(crate) tree: &'a Tree,
    pub(crate) images: &'a ImageStore,
    pub(crate) changes: &'a FrameChanges,
    pub(crate) window: WindowId,
    pub(crate) root: NodeId,
    pub(crate) viewport: Vec2,
    pub(crate) scale: f32,
}

impl<'a> FrameTree<'a> {
    pub fn window(&self) -> WindowId {
        self.window
    }

    /// The window's root node; its children are the window's layers, bottom first.
    pub fn root(&self) -> NodeId {
        self.root
    }

    pub fn node(&self, id: NodeId) -> Option<&'a FrameNode> {
        self.tree.nodes.get(&id).map(|n| &n.frame)
    }

    /// Children in paint order.
    pub fn children(&self, id: NodeId) -> &'a [NodeId] {
        self.tree.nodes.get(&id).map_or(&[], |n| n.children.as_slice())
    }

    pub fn images(&self) -> &'a ImageStore {
        self.images
    }

    pub fn changes(&self) -> &'a FrameChanges {
        self.changes
    }

    pub fn viewport(&self) -> Vec2 {
        self.viewport
    }

    pub fn scale(&self) -> f32 {
        self.scale
    }

    /// Calls `f` for every display list that paints inside its clip, in paint order: a node's
    /// content, then its children, then its overlay. Subtrees whose clip is empty are skipped.
    pub fn walk(&self, mut f: impl FnMut(Draw<'_>)) {
        let mut rounded: Vec<Clip> = Vec::new();
        let mut stack: Vec<Step> = vec![Step::Enter(self.root, Vec2::ZERO, Rect::from_min_size(emath::Pos2::ZERO, self.viewport), 1.0, 0)];
        while let Some(step) = stack.pop() {
            match step {
                Step::Enter(id, parent_origin, parent_clip, parent_opacity, depth) => {
                    let Some(node) = self.tree.nodes.get(&id) else {
                        continue;
                    };
                    rounded.truncate(depth);
                    let fr = &node.frame;
                    let origin = parent_origin + fr.offset;
                    let clip = match fr.clip {
                        Some(c) => {
                            if !c.radii.is_zero() {
                                rounded.push(Clip { rect: c.rect.translate(origin), radii: c.radii });
                            }
                            parent_clip.intersect(c.rect.translate(origin))
                        }
                        None => parent_clip,
                    };
                    if is_empty(clip) {
                        continue;
                    }
                    let opacity = parent_opacity * fr.opacity;
                    let resolve = |ink: Option<Rect>| {
                        let ink = ink?.translate(origin).intersect(clip);
                        (!is_empty(ink)).then_some(ink)
                    };
                    if let Some(ink) = resolve(fr.content_ink) {
                        f(Draw {
                            node: id,
                            part: Part::Content,
                            list: &fr.content,
                            version: fr.version,
                            origin,
                            clip,
                            rounded: &rounded,
                            ink,
                            opacity,
                        });
                    }
                    if let Some(ink) = resolve(fr.overlay_ink) {
                        stack.push(Step::Overlay(id, origin, clip, ink, opacity, rounded.len()));
                    }
                    for &child in node.children.iter().rev() {
                        stack.push(Step::Enter(child, origin, clip, opacity, rounded.len()));
                    }
                }
                Step::Overlay(id, origin, clip, ink, opacity, depth) => {
                    let Some(node) = self.tree.nodes.get(&id) else {
                        continue;
                    };
                    rounded.truncate(depth);
                    f(Draw {
                        node: id,
                        part: Part::Overlay,
                        list: &node.frame.overlay,
                        version: node.frame.version,
                        origin,
                        clip,
                        rounded: &rounded,
                        ink,
                        opacity,
                    });
                }
            }
        }
    }
}

enum Step {
    Enter(NodeId, Vec2, Rect, f32, usize),
    /// A node's overlay, drawn after its children: node, origin, clip, ink, opacity, rounded depth.
    Overlay(NodeId, Vec2, Rect, Rect, f32, usize),
}

/// The window-space origin and clip of `id`'s parent.
pub(crate) fn parent_space(tree: &Tree, id: NodeId, viewport: Vec2) -> (Vec2, Rect) {
    let mut chain: SmallVec<[NodeId; 16]> = SmallVec::new();
    let mut cur = tree.nodes.get(&id).and_then(|n| n.parent);
    while let Some(c) = cur {
        chain.push(c);
        cur = tree.nodes.get(&c).and_then(|n| n.parent);
    }
    let mut origin = Vec2::ZERO;
    let mut clip = Rect::from_min_size(emath::Pos2::ZERO, viewport);
    for &c in chain.iter().rev() {
        let Some(n) = tree.nodes.get(&c) else { break };
        origin += n.frame.offset;
        if let Some(r) = n.frame.clip_rect() {
            clip = clip.intersect(r.translate(origin));
        }
    }
    (origin, clip)
}

/// Damage for `id` (and, with `deep`, everything under it): the window-space ink it had at its
/// last change and the ink it has now. Each visited node keeps the new ink for the next time.
/// `deep` is for a node whose placement, clip or opacity changed, which moves or restyles every
/// pixel under it.
pub(crate) fn node_damage(tree: &mut Tree, id: NodeId, viewport: Vec2, deep: bool, damage: &mut Vec<Rect>) {
    let (origin, clip) = parent_space(tree, id, viewport);
    let mut stack = vec![(id, origin, clip)];
    while let Some((id, parent_origin, parent_clip)) = stack.pop() {
        let Some(node) = tree.nodes.get_mut(&id) else {
            continue;
        };
        let origin = parent_origin + node.frame.offset;
        let clip = match node.frame.clip_rect() {
            Some(r) => parent_clip.intersect(r.translate(origin)),
            None => parent_clip,
        };
        let new = node.frame.ink().map(|r| r.translate(origin).intersect(clip)).filter(|r| !is_empty(*r));
        damage.extend(node.abs);
        damage.extend(new);
        node.abs = new;
        if deep {
            stack.extend(node.children.iter().map(|&c| (c, origin, clip)));
        }
    }
}

/// Damage rects a frame keeps at most; past it the pair whose union wastes least joins.
const MAX_DAMAGE: usize = 8;

/// Two rects join when their union covers at most this share more than the two did: drawing a
/// little extra is cheaper than another clip and pass.
const JOIN_WASTE: f32 = 0.25;

/// Past this many rects the pairwise search is too slow for a frame: neighbours in reading order
/// join in pairs first.
const MAX_PAIRWISE: usize = 64;

fn area(r: Rect) -> f32 {
    r.width() * r.height()
}

fn overlaps(a: Rect, b: Rect) -> bool {
    !is_empty(a.intersect(b))
}

/// Window area a union of `a` and `b` adds beyond what they cover.
fn waste(a: Rect, b: Rect) -> f32 {
    let overlap = a.intersect(b);
    let both = area(a) + area(b) - if is_empty(overlap) { 0.0 } else { area(overlap) };
    area(a.union(b)) - both
}

/// `rects` with every overlapping group joined; the result's rects never overlap.
fn join_overlapping(rects: impl IntoIterator<Item = Rect>) -> Vec<Rect> {
    let mut out: Vec<Rect> = Vec::new();
    for mut r in rects.into_iter().filter(|r| !is_empty(*r)) {
        while let Some(i) = out.iter().position(|o| overlaps(*o, r)) {
            r = r.union(out.swap_remove(i));
        }
        out.push(r);
    }
    out
}

/// Merges damage rects: overlapping rects join, rects whose union wastes little join, and past
/// [`MAX_DAMAGE`] the pair with the smallest waste joins until the count fits. Scattered small
/// changes stay small rects instead of growing to their bounding box.
pub(crate) fn merge_damage(rects: &mut Vec<Rect>) {
    let mut out = join_overlapping(rects.drain(..));
    while out.len() > MAX_PAIRWISE {
        out.sort_by(|a, b| (a.min.y, a.min.x).partial_cmp(&(b.min.y, b.min.x)).unwrap_or(std::cmp::Ordering::Equal));
        let paired: Vec<Rect> = out.chunks(2).map(|c| c.iter().copied().reduce(|a, b| a.union(b)).unwrap_or(c[0])).collect();
        out = join_overlapping(paired);
    }
    while out.len() > 1 {
        let mut best = (0, 1, f32::INFINITY);
        for i in 0..out.len() {
            for j in i + 1..out.len() {
                let w = waste(out[i], out[j]);
                if w < best.2 {
                    best = (i, j, w);
                }
            }
        }
        let (i, j, w) = best;
        let cheap = w <= JOIN_WASTE * (area(out[i]) + area(out[j]));
        if !cheap && out.len() <= MAX_DAMAGE {
            break;
        }
        let b = out.swap_remove(j);
        let a = out.swap_remove(i);
        out.push(a.union(b));
        out = join_overlapping(out);
    }
    *rects = out;
}

#[cfg(test)]
mod tests {
    use super::*;
    use emath::pos2;

    fn r(x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
        Rect::from_min_max(pos2(x0, y0), pos2(x1, y1))
    }

    #[test]
    fn far_apart_damage_stays_apart_and_scattered_damage_stays_small() {
        let mut d = vec![r(0.0, 0.0, 10.0, 10.0), r(990.0, 990.0, 1000.0, 1000.0)];
        merge_damage(&mut d);
        assert_eq!(d.len(), 2, "two corners do not become the whole window");

        // 40 small rects on a grid over a 1000 x 1000 window: the count is bounded and the area
        // stays a small share of the window.
        let mut d: Vec<Rect> = (0..40)
            .map(|i| {
                let (x, y) = ((i % 8) as f32 * 120.0, (i / 8) as f32 * 190.0);
                r(x, y, x + 12.0, y + 12.0)
            })
            .collect();
        merge_damage(&mut d);
        assert!(d.len() <= MAX_DAMAGE);
        let covered: f32 = d.iter().map(|r| area(*r)).sum();
        assert!(covered < 0.35 * 1_000_000.0, "covered {covered}");
        for (i, a) in d.iter().enumerate() {
            for b in &d[i + 1..] {
                assert!(is_empty(a.intersect(*b)), "merged rects never overlap");
            }
        }

        // Neighbours join: two touching row rects become one.
        let mut d = vec![r(0.0, 0.0, 100.0, 20.0), r(0.0, 20.0, 100.0, 40.0)];
        merge_damage(&mut d);
        assert_eq!(d, vec![r(0.0, 0.0, 100.0, 40.0)]);
    }
}
