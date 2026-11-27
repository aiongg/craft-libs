use std::hash::Hash;
use std::sync::Arc;

use craft_theme::Tokens;

use crate::element::{AnyElement, Element, ElementBox, Memo};
use crate::id::{ChildKey, IdSet, NodeId, WindowId, hash_key};
use crate::layer::{Layer, LayerSpec};
use crate::runtime::Runtime;
use crate::state::{Access, Phase, Reader, Reads, State, Store, Writes};
use crate::tree::{Dirty, Node};
use crate::window::Env;

/// The build context of one node. Children are reconciled against the node's previous children by
/// id: an explicit key inside [`BuildCx::keyed`], else the position among unkeyed siblings.
pub struct BuildCx<'a> {
    rt: &'a mut Runtime,
    node: NodeId,
    window: WindowId,
    children: Vec<NodeId>,
    layered: Vec<NodeId>,
    pos: u32,
    scope: Option<(u64, u32)>,
    /// Set by [`BuildCx::keep_children`]: the build leaves the node's children as they were.
    keep: bool,
}

/// Why an inline node is reached in its owner's build: the facts the owner decides on, before
/// running the node's hooks or keeping its outputs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InlineState {
    /// The node was created by this call.
    pub new: bool,
    /// Its props key differs from the last build's.
    pub key_moved: bool,
    /// It carried `BUILD`: queued input or a changed read of its own.
    pub marked: bool,
    /// It carried `PATH`: a node it owns is marked, so its content must be reached.
    pub path: bool,
}

/// What an inline node kept, read without reconciling it ([`BuildCx::peek`]).
pub struct Peek<'p, L> {
    pub node: NodeId,
    pub local: &'p L,
    /// Its marks as they stand; `new` and `key_moved` are always false.
    pub state: InlineState,
}

impl InlineState {
    /// The node's own hooks must run: it is new, its key moved, or it is marked.
    pub fn must_run(&self) -> bool {
        self.new || self.key_moved || self.marked
    }
}

impl Reads for BuildCx<'_> {
    fn access(&mut self) -> Access<'_> {
        Access { store: &mut self.rt.store, reader: Some(Reader::Node(self.node, Phase::Build)) }
    }
}

impl Writes for BuildCx<'_> {
    fn store_mut(&mut self) -> &mut Store {
        &mut self.rt.store
    }
}

impl BuildCx<'_> {
    pub fn node(&self) -> NodeId {
        self.node
    }

    pub fn window(&self) -> WindowId {
        self.window
    }

    pub(crate) fn runtime_mut(&mut self) -> &mut Runtime {
        self.rt
    }

    pub fn env(&self) -> Env {
        self.rt.env_of(self.node)
    }

    pub fn tokens(&mut self) -> Arc<Tokens> {
        let tokens = self.env().tokens;
        tokens.get(self)
    }

    /// The images renderers upload. A component registers the pixels its display lists name (a
    /// thumbnail, an icon mask) by key; a key already there is left alone.
    pub fn images(&mut self) -> &mut crate::display::ImageStore {
        &mut self.rt.images
    }

    /// A cell owned by this node, freed when the node goes. Call it from `Element::init`.
    pub fn state<T: 'static>(&mut self, value: T) -> State<T> {
        let state = self.rt.store.create(value);
        if let Some(n) = self.rt.tree.nodes.get_mut(&self.node) {
            n.owned.push(state.slot_ref());
        }
        state
    }

    /// Children created inside `f` are named by `key`, so they keep their nodes when siblings are
    /// inserted, removed or reordered.
    pub fn keyed<R>(&mut self, key: impl Hash, f: impl FnOnce(&mut Self) -> R) -> R {
        let saved = self.scope.replace((hash_key(&key), 0));
        let out = f(self);
        self.scope = saved;
        out
    }

    /// A child that reruns `f` only when the hash of `props` moves (or something `f` read
    /// changed).
    pub fn memo<P: Hash + 'static>(&mut self, id: impl Hash, props: P, f: impl Fn(&P, &mut BuildCx<'_>) + 'static) {
        self.keyed(id, |cx| cx.child(Memo::new(props, f)));
    }

    /// The child shown last in this build, for anchoring a layer child to it.
    pub fn last_child(&self) -> Option<NodeId> {
        self.children.last().copied()
    }

    /// Reconciles one child and returns what it emitted since this node last built.
    pub fn child<E: Element>(&mut self, el: E) -> E::Output {
        let id = self.next_child();
        reconcile(self.rt, id, self.node, self.node, self.window, el)
    }

    fn next_child(&mut self) -> NodeId {
        let key = match &mut self.scope {
            Some((k, i)) => {
                *i += 1;
                ChildKey::Keyed(*k, *i - 1)
            }
            None => {
                self.pos += 1;
                ChildKey::Pos(self.pos - 1)
            }
        };
        let id = self.node.child(key);
        self.children.push(id);
        id
    }

    /// Reconciles an inline child: a node whose content this build creates, through `f`, instead
    /// of its own `Element::build`. The node keeps `el` (its props key decides
    /// [`InlineState::key_moved`]) and its `Local`, which `f` gets with the node's facts and a
    /// context for the node's own children. Its `BUILD` and `PATH` marks are cleared before `f`
    /// runs. Children `f` does not reach go, unless `f` calls [`BuildCx::keep_children`].
    ///
    /// Nothing is marked for layout, paint or access: `f` calls [`BuildCx::mark_changed`] or
    /// [`BuildCx::request_layout`] for what changed, so a kept node costs no phase work.
    pub fn inline<E: Element, R>(&mut self, el: E, f: impl FnOnce(&mut E::Local, InlineState, &mut BuildCx<'_>) -> R) -> R {
        let id = self.next_child();
        let node = self.node;
        self.inline_at(id, node, false, el, f)
    }

    /// Reconciles an inline child on `layer` of this node's window, placed by `spec`: a popup,
    /// menu, tooltip, toast, dialog or drag preview whose content this build creates, as
    /// [`BuildCx::inline`] does for a child in place. It is this node's for build and output (it
    /// goes when a build of this node leaves it out, or this node goes) and lies on the layer for
    /// layout, paint and input, above everything on the layers below. `key` names it among this
    /// node's children on that layer.
    pub fn inline_layer<E: Element, R>(
        &mut self,
        layer: Layer,
        key: impl Hash,
        spec: LayerSpec,
        el: E,
        f: impl FnOnce(&mut E::Local, InlineState, &mut BuildCx<'_>) -> R,
    ) -> R {
        let id = self.node.child(ChildKey::Layer(layer.index(), hash_key(&key)));
        debug_assert!(!self.layered.contains(&id), "a node shows two layer children with one key");
        self.layered.push(id);
        let Some(w) = self.rt.windows.get_mut(&self.window) else {
            return f(&mut el.init(self), InlineState::default(), self);
        };
        let parent = w.layer(layer);
        if w.layered.insert(id, spec) != Some(spec) {
            w.relayout = true;
        }
        // Listed before it builds, so what it puts on the same layer lies above it.
        self.list_on_layer(parent, id);
        self.inline_at(id, parent, true, el, f)
    }

    fn inline_at<E: Element, R>(
        &mut self,
        id: NodeId,
        parent: NodeId,
        on_layer: bool,
        el: E,
        f: impl FnOnce(&mut E::Local, InlineState, &mut BuildCx<'_>) -> R,
    ) -> R {
        let (owner, window) = (self.node, self.window);
        let props_key = el.props_key();
        let mut state = InlineState::default();
        let reused = match self.rt.tree.nodes.get_mut(&id) {
            Some(node) if node.inline && node.parent == Some(parent) && node.owner == Some(owner) => {
                let mut boxed = node.element.take();
                match boxed.as_mut().and_then(|b| b.as_any_mut().downcast_mut::<ElementBox<E>>()) {
                    Some(slot) => {
                        slot.el = el;
                        state.key_moved = node.props_key != props_key;
                        node.props_key = props_key;
                        Ok(boxed)
                    }
                    None => {
                        node.element = boxed;
                        Err(el)
                    }
                }
            }
            _ => Err(el),
        };
        let mut boxed: Box<dyn AnyElement> = match reused {
            Ok(Some(boxed)) => boxed,
            Ok(None) => unreachable!("a reused node holds its element"),
            Err(el) => {
                if self.rt.tree.nodes.contains_key(&id) {
                    self.rt.remove_subtree(id);
                    if on_layer {
                        self.list_on_layer(parent, id);
                    }
                }
                let depth = self.rt.tree.depth(owner) + 1;
                let mut node = Node::new(Some(parent), Some(owner), window, depth, None, props_key);
                node.inline = true;
                // The runtime places layer children itself, so their size never reaches the layer.
                node.boundary |= on_layer;
                self.rt.tree.nodes.insert(id, node);
                state.new = true;
                Box::new(ElementBox::new(el))
            }
        };
        let Some(node) = self.rt.tree.nodes.get_mut(&id) else { unreachable!("the node was just reconciled") };
        state.marked = node.dirty.has(Dirty::BUILD);
        state.path = node.dirty.has(Dirty::PATH);
        node.dirty.remove(Dirty::BUILD);
        node.dirty.remove(Dirty::PATH);
        let old = std::mem::take(&mut node.children);
        let old_layered = std::mem::take(&mut node.layered);
        let mut cx = BuildCx { rt: &mut *self.rt, node: id, window, children: Vec::new(), layered: Vec::new(), pos: 0, scope: None, keep: false };
        let slot = boxed.as_any_mut().downcast_mut::<ElementBox<E>>().expect("the element's type was checked");
        let local = match &mut slot.local {
            Some(local) => local,
            None => slot.local.insert(slot.el.init(&mut cx)),
        };
        let out = f(local, state, &mut cx);
        let BuildCx { children, layered, keep, .. } = cx;
        finish_children(self.rt, id, old, old_layered, children, layered, keep);
        if let Some(node) = self.rt.tree.nodes.get_mut(&id) {
            node.element = Some(boxed);
        }
        out
    }

    /// The inline child of `parent` that `keyed(key, |cx| cx.inline(el, ..))` reconciles, when it
    /// exists with an element of type `E`: its `Local` and marks, left as they are. A measuring run
    /// asks a kept child for its size this way, so it reaches and changes no node.
    pub fn peek<E: Element>(&self, parent: NodeId, key: impl Hash) -> Option<Peek<'_, E::Local>> {
        let id = parent.child(ChildKey::Keyed(hash_key(&key), 0));
        let node = self.rt.tree.nodes.get(&id)?;
        if !node.inline || node.parent != Some(parent) {
            return None;
        }
        let local = node.element.as_ref()?.as_any().downcast_ref::<ElementBox<E>>()?.local.as_ref()?;
        let state = InlineState { new: false, key_moved: false, marked: node.dirty.has(Dirty::BUILD), path: node.dirty.has(Dirty::PATH) };
        Some(Peek { node: id, local, state })
    }

    /// The inline child of this node on `layer` that `inline_layer(layer, key, ..)` reconciles, when
    /// it exists with an element of type `E`: its `Local` and marks, left as they are. An owner
    /// reads a layer child's dismissal this way before deciding to show it again.
    pub fn peek_layer<E: Element>(&self, layer: Layer, key: impl Hash) -> Option<Peek<'_, E::Local>> {
        let id = self.node.child(ChildKey::Layer(layer.index(), hash_key(&key)));
        let node = self.rt.tree.nodes.get(&id)?;
        if !node.inline || node.owner != Some(self.node) {
            return None;
        }
        let local = node.element.as_ref()?.as_any().downcast_ref::<ElementBox<E>>()?.local.as_ref()?;
        let state = InlineState { new: false, key_moved: false, marked: node.dirty.has(Dirty::BUILD), path: node.dirty.has(Dirty::PATH) };
        Some(Peek { node: id, local, state })
    }

    /// The frame time, read without recording a read: a build that schedules itself with
    /// [`BuildCx::request_build_at`] reads the clock this way.
    pub fn now(&self) -> f64 {
        self.rt.time
    }

    /// This node builds again at frame time `at`, and no frame runs for it before then; it
    /// replaces an earlier build wake of this node. `None` cancels it.
    pub fn request_build_at(&mut self, at: Option<f64>) {
        let node = self.node;
        self.rt.set_build_wake(node, at);
    }

    /// This node gets [`crate::Event::Hover`] when the pointer enters or leaves it or a node under
    /// it (a tooltip's owner times its delay from it).
    pub fn hover_events(&mut self, on: bool) {
        if let Some(n) = self.rt.tree.nodes.get_mut(&self.node) {
            n.hover_events = on;
        }
    }

    /// This inline node runs again in a later build round of this frame: what its build decided
    /// from turned out stale (a reused measurement its content no longer matches).
    pub fn rebuild(&mut self) {
        let node = self.node;
        self.rt.tree.mark_build(node);
    }

    /// The build leaves this node's children and layer children as they were: an inline node whose
    /// content is kept. Call it before reconciling any child.
    pub fn keep_children(&mut self) {
        debug_assert!(self.children.is_empty() && self.layered.is_empty(), "keep_children after reconciling a child");
        self.keep = true;
    }

    /// This node's outputs changed: its layout, paint and access node run in this frame.
    pub fn mark_changed(&mut self) {
        let node = self.node;
        self.rt.access_dirty(node);
        self.rt.tree.mark_layout(node);
        self.rt.tree.mark_paint(node);
    }

    /// This node's layout runs in this frame (its children moved).
    pub fn request_layout(&mut self) {
        let node = self.node;
        self.rt.tree.mark_layout(node);
    }

    /// Counts a run of a contract component's hooks in the frame's report.
    pub fn count_run(&mut self) {
        self.rt.counters.runs += 1;
    }

    /// Counts a run of a scope's closure in the frame's report.
    pub fn count_scope(&mut self) {
        self.rt.counters.scopes += 1;
    }

    /// Shows `el` on `layer` of this node's window, placed by `spec`, above everything on the
    /// layers below. It is this node's child for build and output: it stays while this node shows
    /// it in each build and goes when a build leaves it out or this node goes. Keys name it among
    /// this node's children on the same layer.
    pub fn layer<E: Element>(&mut self, layer: Layer, key: impl Hash, spec: LayerSpec, el: E) -> E::Output {
        let id = self.node.child(ChildKey::Layer(layer.index(), hash_key(&key)));
        if self.layered.contains(&id) {
            debug_assert!(false, "a node shows two layer children with one key");
            return E::Output::default();
        }
        self.layered.push(id);
        let Some(w) = self.rt.windows.get_mut(&self.window) else {
            return E::Output::default();
        };
        let parent = w.layer(layer);
        if w.layered.insert(id, spec) != Some(spec) {
            w.relayout = true;
        }
        // Listed before it builds, so what it puts on the same layer lies above it.
        self.list_on_layer(parent, id);
        let out = reconcile(self.rt, id, parent, self.node, self.window, el);
        self.list_on_layer(parent, id);
        out
    }

    fn list_on_layer(&mut self, layer: NodeId, id: NodeId) {
        if let Some(p) = self.rt.tree.nodes.get_mut(&layer)
            && !p.children.contains(&id)
        {
            p.children.push(id);
        }
    }
}

/// Reconciles node `id` under `parent`, owned by `owner`: the kept node takes the new element and
/// rebuilds when its props key moved or it is build-dirty; otherwise a new node replaces it.
fn reconcile<E: Element>(rt: &mut Runtime, id: NodeId, parent: NodeId, owner: NodeId, window: WindowId, el: E) -> E::Output {
    let props_key = el.props_key();
    let kept = match rt.tree.nodes.get_mut(&id) {
        Some(node) if node.parent == Some(parent) && node.owner == Some(owner) => {
            match node.element.as_mut().and_then(|b| b.as_any_mut().downcast_mut::<ElementBox<E>>()) {
                Some(slot) => {
                    slot.el = el;
                    let changed = node.props_key != props_key;
                    node.props_key = props_key;
                    Ok(changed || node.dirty.has(Dirty::BUILD))
                }
                None => Err(el),
            }
        }
        _ => Err(el),
    };
    let rebuild = match kept {
        Ok(rebuild) => rebuild,
        Err(el) => {
            let in_layer = rt.tree.nodes.get(&parent).is_some_and(Node::is_fixed);
            if rt.tree.nodes.contains_key(&id) {
                rt.remove_subtree(id);
            }
            let depth = rt.tree.depth(owner) + 1;
            let mut node = Node::new(Some(parent), Some(owner), window, depth, Some(Box::new(ElementBox::new(el))), props_key);
            // The runtime places layer children itself, so their size never reaches the layer.
            node.boundary = in_layer;
            rt.tree.nodes.insert(id, node);
            true
        }
    };
    if rebuild {
        build_node(rt, id);
    }
    rt.tree.nodes.get_mut(&id).and_then(|n| n.outbox.take()).and_then(|b| b.downcast::<E::Output>().ok()).map(|b| *b).unwrap_or_default()
}

fn gone(old: Vec<NodeId>, new: &[NodeId]) -> Vec<NodeId> {
    if old.as_slice() == new {
        return Vec::new();
    }
    if new.len() > 16 {
        let keep: IdSet = new.iter().copied().collect();
        old.into_iter().filter(|c| !keep.contains(c)).collect()
    } else {
        old.into_iter().filter(|c| !new.contains(c)).collect()
    }
}

/// Runs one node's build and reconciles its children and layer children. Children with unchanged
/// props and no dirty mark are not entered.
pub(crate) fn build_node(rt: &mut Runtime, id: NodeId) {
    let Some(node) = rt.tree.nodes.get_mut(&id) else {
        return;
    };
    node.dirty.remove(Dirty::BUILD);
    let window = node.window;
    let Some(mut el) = node.element.take() else {
        return;
    };
    let old = std::mem::take(&mut node.children);
    let old_layered = std::mem::take(&mut node.layered);
    let mut cx = BuildCx {
        rt,
        node: id,
        window,
        children: Vec::with_capacity(old.len()),
        layered: Vec::with_capacity(old_layered.len()),
        pos: 0,
        scope: None,
        keep: false,
    };
    el.build(&mut cx);
    let BuildCx { children, layered, keep, .. } = cx;
    finish_children(rt, id, old, old_layered, children, layered, keep);
    if let Some(node) = rt.tree.nodes.get_mut(&id) {
        node.element = Some(el);
    }
    rt.counters.built += 1;
    rt.access_dirty(id);
    rt.tree.mark_layout(id);
    rt.tree.mark_paint(id);
}

/// Runs an inline root's build: `content` creates its children. Marks its layout only when its
/// child list changed; the children mark what they changed themselves.
pub(crate) fn build_inline_root(rt: &mut Runtime, id: NodeId, content: &mut dyn FnMut(WindowId, &mut BuildCx<'_>)) {
    let Some(node) = rt.tree.nodes.get_mut(&id) else {
        return;
    };
    node.dirty.remove(Dirty::BUILD);
    node.dirty.remove(Dirty::PATH);
    let window = node.window;
    let Some(mut el) = node.element.take() else {
        return;
    };
    let old = std::mem::take(&mut node.children);
    let old_layered = std::mem::take(&mut node.layered);
    let mut cx = BuildCx { rt, node: id, window, children: Vec::new(), layered: Vec::new(), pos: 0, scope: None, keep: false };
    // Creates the root's `Local` on the first build; the element's own build adds nothing.
    el.build(&mut cx);
    content(window, &mut cx);
    let BuildCx { children, layered, keep, .. } = cx;
    if let Some(node) = rt.tree.nodes.get_mut(&id) {
        node.element = Some(el);
    }
    let changed = !keep && (children != old || layered != old_layered);
    finish_children(rt, id, old, old_layered, children, layered, keep);
    if changed {
        rt.tree.mark_layout(id);
    }
}

/// Stores a build's children on `id` and drops the old ones it did not reach; with `keep`, the old
/// lists stay.
fn finish_children(rt: &mut Runtime, id: NodeId, old: Vec<NodeId>, old_layered: Vec<NodeId>, new: Vec<NodeId>, new_layered: Vec<NodeId>, keep: bool) {
    let (new, new_layered) = if keep {
        (old, old_layered)
    } else {
        for child in gone(old, &new).into_iter().chain(gone(old_layered, &new_layered)) {
            rt.remove_subtree(child);
        }
        (new, new_layered)
    };
    if let Some(node) = rt.tree.nodes.get_mut(&id) {
        node.children = new;
        node.layered = new_layered;
    }
}
