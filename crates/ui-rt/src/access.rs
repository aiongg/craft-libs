//! Incremental AccessKit. Every node is an access node with id `NodeId as u64`; an element
//! without its own node is a generic container. A node's transform is its offset and its bounds
//! are its own rect, so scrolling moves one transform and no row's node changes.
//!
//! An access node is rebuilt only when its node built, moved or resized, or asked for it
//! ([`crate::EventCx::request_access`]). A frame's `TreeUpdate` holds the rebuilt nodes that
//! differ from what was sent, nodes that came into view, parents whose child list changed, and
//! the focus. Nodes outside every clip above them are absent, so rows out of view are not in the
//! tree.

use std::collections::HashMap;

use accesskit::{Affine, Node, Role, TreeId, TreeInfo, TreeUpdate};
use emath::{Pos2, Rect, Vec2};
use smallvec::SmallVec;

use crate::id::{IdMap, IdSet, NodeId, WindowId, hash_key};
use crate::runtime::Runtime;
use crate::state::{Access, Phase, Reader, Reads, Store};
use crate::tree::Role as NodeRole;

/// What a node last produced and sent.
#[derive(Default)]
pub(crate) struct AccessData {
    /// The node without its children.
    node: Option<Node>,
    subs: Vec<SubNode>,
    /// The node was in the last sent tree.
    included: bool,
    children: Vec<accesskit::NodeId>,
}

#[derive(Clone, Debug, PartialEq)]
struct SubNode {
    key: u64,
    parent: Option<u64>,
    node: Node,
}

pub(crate) struct AccessState {
    /// Off until the platform reports an assistive technology ([`crate::WindowEvent::AccessActivated`]).
    pub(crate) enabled: bool,
    /// The next update sends the whole tree.
    pub(crate) full: bool,
    pub(crate) dirty: IdSet,
    /// Sub-node id -> (owner node, key), for routing actions.
    pub(crate) subs: HashMap<u64, (NodeId, u64)>,
    /// Focus stop -> the item it reports as focused while it has focus.
    active: IdMap<NodeId>,
    last_focus: Option<accesskit::NodeId>,
    pub(crate) announcer: Announcer,
}

/// How an announcement is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Politeness {
    /// After the current speech.
    Polite,
    /// At once.
    Assertive,
}

/// Ends the text of a status that repeats the previous frame's: some platform adapters raise a live-region event
/// only when the live node's text changes. Alternates, so three equal frames in a row still differ pairwise.
const REPEAT_MARK: char = '\u{200b}';

/// One window's announcements: the texts made since the last update become the text of one live status node, empty
/// again in the update after.
#[derive(Default)]
pub(crate) struct Announcer {
    pending: Vec<(String, Politeness)>,
    previous: String,
    previous_marked: bool,
    /// The text and urgency the status node last sent carried.
    sent: Option<(String, bool)>,
}

impl Announcer {
    pub(crate) fn announce(&mut self, text: String, politeness: Politeness) {
        if !self.pending.iter().any(|(t, p)| *t == text && *p == politeness) {
            self.pending.push((text, politeness));
        }
    }

    /// Something to say, or a text to clear.
    fn pending(&self) -> bool {
        !self.pending.is_empty() || self.sent.as_ref().is_some_and(|(text, _)| !text.is_empty())
    }

    /// The status node for this update, when it differs from the one sent (always on a full update).
    fn flush(&mut self, full: bool) -> Option<Node> {
        let texts = std::mem::take(&mut self.pending);
        let assertive = texts.iter().any(|(_, p)| *p == Politeness::Assertive);
        let plain = texts.iter().map(|(t, _)| t.as_str()).collect::<Vec<_>>().join(". ");
        let marked = !plain.is_empty() && plain == self.previous && !self.previous_marked;
        let mut text = plain.clone();
        if marked {
            text.push(REPEAT_MARK);
        }
        self.previous = plain;
        self.previous_marked = marked;
        let key = (text, assertive);
        if !full && self.sent.as_ref() == Some(&key) {
            return None;
        }
        let mut node = Node::new(Role::Status);
        node.set_live(if assertive { accesskit::Live::Assertive } else { accesskit::Live::Polite });
        node.set_live_atomic();
        if !key.0.is_empty() {
            node.set_label(key.0.as_str());
        }
        self.sent = Some(key);
        Some(node)
    }
}

fn status_id(window: WindowId) -> accesskit::NodeId {
    accesskit::NodeId(hash_key(&(window.root().0, "status")))
}

impl Default for AccessState {
    fn default() -> Self {
        AccessState {
            enabled: false,
            full: true,
            dirty: IdSet::default(),
            subs: HashMap::new(),
            active: IdMap::default(),
            last_focus: None,
            announcer: Announcer::default(),
        }
    }
}

impl AccessState {
    /// Work for the next frame: something to rebuild, or a full tree owed.
    pub(crate) fn pending(&self) -> bool {
        self.enabled && (self.full || !self.dirty.is_empty() || self.announcer.pending())
    }
}

/// The context an element writes its access node in. Reads are recorded as build reads, so a
/// cell the access node shows rebuilds the node when it changes.
pub struct AccessCx<'a> {
    store: &'a mut Store,
    node: NodeId,
    size: Vec2,
    subs: Vec<SubNode>,
    active: bool,
}

impl Reads for AccessCx<'_> {
    fn access(&mut self) -> Access<'_> {
        Access { store: self.store, reader: Some(Reader::Node(self.node, Phase::Build)) }
    }
}

impl AccessCx<'_> {
    pub fn node(&self) -> NodeId {
        self.node
    }

    pub fn size(&self) -> Vec2 {
        self.size
    }

    /// An access node of its own for a part of this node (a switch inside a cell). `parent` is
    /// another sub-node's key, or `None` for this node; sub-nodes come before the node's
    /// children, in the order added. Bounds are in this node's coordinates.
    pub fn sub(&mut self, key: u64, parent: Option<u64>, node: Node) -> accesskit::NodeId {
        self.subs.push(SubNode { key, parent, node });
        sub_id(self.node, key)
    }

    pub fn sub_id(&self, key: u64) -> accesskit::NodeId {
        sub_id(self.node, key)
    }

    /// This node is the active item of its nearest focusable ancestor (a list's lead row):
    /// while that ancestor has focus, the accessibility focus is here.
    pub fn set_active(&mut self) {
        self.active = true;
    }
}

fn sub_id(node: NodeId, key: u64) -> accesskit::NodeId {
    accesskit::NodeId(hash_key(&(node.0, key, "sub")))
}

fn to_ak(r: Rect) -> accesskit::Rect {
    accesskit::Rect::new(f64::from(r.min.x), f64::from(r.min.y), f64::from(r.max.x), f64::from(r.max.y))
}

fn from_ak(r: accesskit::Rect) -> Rect {
    Rect::from_min_max(Pos2::new(r.x0 as f32, r.y0 as f32), Pos2::new(r.x1 as f32, r.y1 as f32))
}

/// One node of [`Runtime::access_snapshot`].
#[derive(Clone, Debug)]
pub struct AccessEntry<'a> {
    pub id: accesskit::NodeId,
    pub node: &'a Node,
    /// Window coordinates, logical points.
    pub bounds: Rect,
    pub owner: NodeId,
    pub sub: Option<u64>,
}

impl Runtime {
    /// Turns `window`'s access tree on or off. Turning it on sends the whole tree with the next
    /// frame.
    pub fn set_access_enabled(&mut self, window: WindowId, on: bool) {
        let Some(w) = self.windows.get_mut(&window) else { return };
        let turned_on = on && !w.access.enabled;
        w.access.enabled = on;
        w.access.full = true;
        w.access.last_focus = None;
        w.access.announcer = Announcer::default();
        if turned_on {
            // Nodes that ran while access was off kept no access node: every node builds once more.
            let ids: Vec<NodeId> = self.tree.nodes.iter().filter(|(_, n)| n.window == window).map(|(id, _)| *id).collect();
            for id in ids {
                self.tree.mark_build(id);
            }
        }
    }

    /// The text of `window`'s status node as last sent (empty when nothing is announced); `None`
    /// before the first update.
    pub fn access_status(&self, window: WindowId) -> Option<String> {
        self.windows.get(&window)?.access.announcer.sent.as_ref().map(|(text, _)| text.trim_end_matches(REPEAT_MARK).to_owned())
    }

    /// Whether `window`'s access tree is on: nodes build their access node only then.
    pub fn access_enabled(&self, window: WindowId) -> bool {
        self.windows.get(&window).is_some_and(|w| w.access.enabled)
    }

    /// The node assistive technologies see as focused in `window`: the focused node's active item
    /// when it is in the tree, else the focused node when it is, else the window.
    pub fn access_focus(&self, window: WindowId) -> accesskit::NodeId {
        let root = window.root().access_id();
        let included = |id: &NodeId| self.tree.nodes.get(id).is_some_and(|n| n.access.included);
        let Some(w) = self.windows.get(&window) else { return root };
        let Some(focused) = w.input.focused else {
            return root;
        };
        if let Some(item) = w.access.active.get(&focused).filter(|i| included(i)) {
            return item.access_id();
        }
        if included(&focused) { focused.access_id() } else { root }
    }

    /// Builds `window`'s `TreeUpdate` for this frame from what changed. Returns the number of
    /// nodes in it.
    pub(crate) fn update_access(&mut self, window: WindowId) -> u32 {
        let Some(w) = self.windows.get_mut(&window) else { return 0 };
        let dirty = std::mem::take(&mut w.access.dirty);
        if !w.access.enabled || !self.tree.nodes.contains_key(&w.app_root) {
            return 0;
        }
        let full = w.access.full;
        let (root, viewport) = (w.root, w.viewport);
        let mut out = Vec::new();
        if full || !dirty.is_empty() || !w.changes.moved.is_empty() {
            let moved: IdSet = w.changes.moved.iter().copied().collect();
            let mut walk = Walk { window, full, dirty: &dirty, moved: &moved, status: status_id(window), out: &mut out };
            let mut kids = AccessIds::new();
            self.walk_access(root, Vec2::ZERO, Rect::from_min_size(Pos2::ZERO, viewport), true, &mut walk, &mut kids);
        }
        let focus = self.access_focus(window);
        let Some(w) = self.windows.get_mut(&window) else { return 0 };
        if let Some(status) = w.access.announcer.flush(full) {
            out.push((status_id(window), status));
        }
        if out.is_empty() && !full && w.access.last_focus == Some(focus) {
            return 0;
        }
        w.access.full = false;
        w.access.last_focus = Some(focus);
        let count = u32::try_from(out.len()).unwrap_or(u32::MAX);
        let update = TreeUpdate {
            nodes: out,
            tree: full.then(|| {
                let mut info = TreeInfo::new(root.access_id());
                info.toolkit_name = Some("craft".into());
                info
            }),
            tree_id: TreeId::ROOT,
            focus,
        };
        w.access_update = Some(match w.access_update.take() {
            None => update,
            Some(mut earlier) => {
                earlier.nodes.retain(|(id, _)| !update.nodes.iter().any(|(n, _)| n == id));
                earlier.nodes.extend(update.nodes);
                earlier.tree = earlier.tree.or(update.tree);
                earlier.focus = update.focus;
                earlier
            }
        });
        count
    }

    /// Visits `id`'s subtree and appends the access ids it contributes to its parent's children
    /// to `kids`: its own when it is in the tree, or, for a layer node (which has no access node),
    /// those of its children.
    fn walk_access(&mut self, id: NodeId, parent_origin: Vec2, parent_clip: Rect, parent_included: bool, walk: &mut Walk<'_>, kids: &mut AccessIds) {
        let Some(n) = self.tree.nodes.get(&id) else {
            return;
        };
        let origin = parent_origin + n.frame.offset;
        if let NodeRole::Layer(_) = n.role {
            let children: SmallVec<[NodeId; 16]> = n.children.iter().copied().collect();
            for child in children {
                self.walk_access(child, origin, parent_clip, parent_included, walk, kids);
            }
            return;
        }
        let rect = Rect::from_min_size(origin.to_pos2(), n.frame.size);
        let top = n.role == NodeRole::WindowRoot;
        let visible = top || crate::geom::is_empty(rect) || !crate::geom::is_empty(rect.intersect(parent_clip));
        let included = parent_included && visible;
        let was = n.access.included;
        if !included && !was {
            return;
        }
        let clip = match n.frame.clip {
            Some(c) => parent_clip.intersect(c.rect.translate(origin)),
            None => parent_clip,
        };
        let mut changed = false;
        if included && (walk.full || !was || walk.dirty.contains(&id) || walk.moved.contains(&id) || n.access.node.is_none()) {
            changed = self.rebuild_access(walk.window, id);
        }

        let children: SmallVec<[NodeId; 16]> = self.tree.nodes.get(&id).map(|n| n.children.iter().copied().collect()).unwrap_or_default();
        let mut own_kids = AccessIds::new();
        for child in children {
            self.walk_access(child, origin, clip, included, walk, &mut own_kids);
        }

        let Some(n) = self.tree.nodes.get_mut(&id) else {
            return;
        };
        let data = &mut n.access;
        data.included = included;
        if !included {
            return;
        }
        let mut children: AccessIds = data.subs.iter().filter(|s| s.parent.is_none()).map(|s| sub_id(id, s.key)).collect();
        children.extend(own_kids);
        if top {
            children.push(walk.status);
        }
        if walk.full || !was || changed || children[..] != data.children[..] {
            let mut node = data.node.clone().unwrap_or_else(|| Node::new(Role::GenericContainer));
            node.set_children(children.to_vec());
            walk.out.push((id.access_id(), node));
            for s in &data.subs {
                let mut sub = s.node.clone();
                let under: Vec<accesskit::NodeId> = data.subs.iter().filter(|c| c.parent == Some(s.key)).map(|c| sub_id(id, c.key)).collect();
                if !under.is_empty() {
                    sub.set_children(under);
                }
                walk.out.push((sub_id(id, s.key), sub));
            }
        }
        if children[..] != data.children[..] {
            data.children = children.to_vec();
        }
        kids.push(id.access_id());
    }

    /// Asks the element for its access node; returns whether it differs from the last one.
    fn rebuild_access(&mut self, window: WindowId, id: NodeId) -> bool {
        let Runtime { tree, store, windows, .. } = self;
        let Some(access) = windows.get_mut(&window).map(|w| &mut w.access) else { return false };
        let Some(n) = tree.nodes.get(&id) else {
            return false;
        };
        let mut cx = AccessCx { store, node: id, size: n.frame.size, subs: Vec::new(), active: false };
        let own = n.element.as_ref().and_then(|e| e.access(&mut cx));
        let (subs, active) = (cx.subs, cx.active);
        let role = if n.role == NodeRole::WindowRoot { Role::Window } else { Role::GenericContainer };
        let mut node = own.unwrap_or_else(|| Node::new(role));
        node.set_bounds(to_ak(Rect::from_min_size(Pos2::ZERO, n.frame.size)));
        if n.frame.offset != Vec2::ZERO {
            node.set_transform(Affine::translate((f64::from(n.frame.offset.x), f64::from(n.frame.offset.y))));
        }
        for s in &subs {
            access.subs.insert(sub_id(id, s.key).0, (id, s.key));
        }
        let stop = if active { nearest_stop(tree, id) } else { None };
        match stop {
            Some(stop) => {
                access.active.insert(stop, id);
            }
            None => access.active.retain(|_, item| *item != id),
        }
        let Some(n) = tree.nodes.get_mut(&id) else {
            return false;
        };
        let changed = n.access.node.as_ref() != Some(&node) || n.access.subs != subs;
        n.access.node = Some(node);
        n.access.subs = subs;
        changed
    }

    /// The nodes of `window`'s last sent tree in tree order (a node, its sub-nodes, then its
    /// children), with window bounds. The harness reads the access tree this way.
    pub fn access_snapshot(&self, window: WindowId) -> Vec<AccessEntry<'_>> {
        let mut out = Vec::new();
        if let Some(w) = self.windows.get(&window) {
            self.snapshot(w.root, Vec2::ZERO, &mut out);
        }
        out
    }

    fn snapshot<'a>(&'a self, id: NodeId, parent_origin: Vec2, out: &mut Vec<AccessEntry<'a>>) {
        let Some(n) = self.tree.nodes.get(&id) else {
            return;
        };
        let origin = parent_origin + n.frame.offset;
        if let NodeRole::Layer(_) = n.role {
            for &child in &n.children {
                self.snapshot(child, origin, out);
            }
            return;
        }
        if !n.access.included {
            return;
        }
        let Some(node) = n.access.node.as_ref() else {
            return;
        };
        out.push(AccessEntry { id: id.access_id(), node, bounds: Rect::from_min_size(origin.to_pos2(), n.frame.size), owner: id, sub: None });
        fn subs<'a>(id: NodeId, all: &'a [SubNode], parent: Option<u64>, origin: Vec2, out: &mut Vec<AccessEntry<'a>>) {
            for s in all.iter().filter(|s| s.parent == parent) {
                let b = s.node.bounds().map_or(Rect::NOTHING, from_ak);
                out.push(AccessEntry { id: sub_id(id, s.key), node: &s.node, bounds: b.translate(origin), owner: id, sub: Some(s.key) });
                subs(id, all, Some(s.key), origin, out);
            }
        }
        subs(id, &n.access.subs, None, origin, out);
        for &child in &n.children {
            self.snapshot(child, origin, out);
        }
    }
}

/// The access ids of one node's children, inline up to a typical row's worth.
type AccessIds = SmallVec<[accesskit::NodeId; 16]>;

struct Walk<'a> {
    window: WindowId,
    full: bool,
    dirty: &'a IdSet,
    moved: &'a IdSet,
    status: accesskit::NodeId,
    out: &'a mut Vec<(accesskit::NodeId, Node)>,
}

fn nearest_stop(tree: &crate::tree::Tree, id: NodeId) -> Option<NodeId> {
    let mut cur = tree.nodes.get(&id)?.parent;
    while let Some(c) = cur {
        let n = tree.nodes.get(&c)?;
        if n.element.as_ref().is_some_and(|e| e.focusable()) {
            return Some(c);
        }
        cur = n.parent;
    }
    None
}
