use std::hash::Hash;
use std::sync::Arc;

use craft_theme::Tokens;

use crate::element::{Element, ElementBox, Memo};
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
        reconcile(self.rt, id, self.node, self.node, self.window, el)
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
    };
    el.build(&mut cx);
    let (new, new_layered) = (cx.children, cx.layered);
    for child in gone(old, &new).into_iter().chain(gone(old_layered, &new_layered)) {
        rt.remove_subtree(child);
    }
    if let Some(node) = rt.tree.nodes.get_mut(&id) {
        node.children = new;
        node.layered = new_layered;
        node.element = Some(el);
    }
    rt.counters.built += 1;
    rt.access_dirty(id);
    rt.tree.mark_layout(id);
    rt.tree.mark_paint(id);
}
