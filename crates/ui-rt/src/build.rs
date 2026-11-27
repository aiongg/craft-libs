use std::hash::Hash;
use std::sync::Arc;

use craft_theme::Tokens;

use crate::element::{AnyElement, Element, ElementBox, Memo};
use crate::id::{ChildKey, IdSet, NodeId, WindowId, hash_key};
use crate::layer::{Layer, LayerSpec};
use crate::runtime::{Hosted, HostedWindow, Runtime};
use crate::state::{Access, Phase, Reader, Reads, State, Store, Writes};
use crate::text::{RichText, TextEngine, TextLayout};
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
    /// The frame's text engine, under an inline root: content built inline measures its text in
    /// the build.
    text: Option<&'a mut dyn TextEngine>,
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

impl<'a> BuildCx<'a> {
    /// Takes the frame's text engine out of the build for a scope in which code that holds no
    /// build lays text out (a component hook between draws); [`Self::return_text_engine`] puts it
    /// back. Records no read: code that used the engine records it with [`Self::text_engine`]
    /// once the engine is back. `None` outside an inline root or while lent.
    pub fn lend_text_engine(&mut self) -> Option<&'a mut dyn TextEngine> {
        self.text.take()
    }

    /// Puts back the engine [`Self::lend_text_engine`] took.
    pub fn return_text_engine(&mut self, engine: Option<&'a mut dyn TextEngine>) {
        if engine.is_some() {
            self.text = engine;
        }
    }
}

impl BuildCx<'_> {
    pub fn node(&self) -> NodeId {
        self.node
    }

    pub fn window(&self) -> WindowId {
        self.window
    }

    /// Asks the platform to change this node's window (a build that shows a title or closes the
    /// window after a prompt).
    pub fn window_command(&mut self, cmd: crate::platform::WindowCommand) {
        let window = self.window;
        self.rt.window_command(window, cmd);
    }

    /// Asks for a new window whose content the host builds inline; the driver opens it after this
    /// step and this node then gets [`crate::Event::WindowOpened`].
    pub fn open_inline_window(&mut self, spec: crate::platform::WindowSpec) {
        let node = self.node;
        self.rt.queue_inline_window(node, spec);
    }

    /// The window `key` this node hosts: a window whose content this node's build creates
    /// ([`BuildCx::window_content`]), so content borrowed from the build (a container's body) shows
    /// in it. The first ask queues the window (`spec` makes its spec); the driver opens it after
    /// this step and this node builds again. Every build of this node that wants the window asks
    /// for it again: a build that leaves it out closes it, and so does this node's removal. A
    /// window the user closed stays [`HostedWindow::Closed`] while the node keeps asking.
    pub fn hosted_window(&mut self, key: impl Hash, spec: impl FnOnce() -> crate::platform::WindowSpec) -> HostedWindow {
        let (node, key) = (self.node, hash_key(&key));
        if let Some(hosted) = self.rt.hosted.iter_mut().find(|h| h.requester == node && h.key == key) {
            hosted.asked = true;
            return hosted.state;
        }
        self.rt.hosted.push(Hosted { requester: node, key, state: HostedWindow::Opening, asked: true });
        self.rt.queue_hosted_window(node, key, spec());
        HostedWindow::Opening
    }

    /// Where the window `key` this node hosts stands, without asking for it; `None` before the
    /// first ask, or after a build that left it out.
    pub fn hosted_state(&self, key: impl Hash) -> Option<HostedWindow> {
        let (node, key) = (self.node, hash_key(&key));
        self.rt.hosted.iter().find(|h| h.requester == node && h.key == key).map(|h| h.state)
    }

    /// Builds the content of `window`, a window this node hosts ([`BuildCx::hosted_window`]),
    /// inside this build: `content` creates the window root's children as a host's `roots`
    /// closure does for an inline root. Input and marks under the window build this node again.
    /// `None` when `window` is not open as this node's hosted window.
    pub fn window_content<R>(&mut self, window: WindowId, content: impl FnOnce(&mut BuildCx<'_>) -> R) -> Option<R> {
        let node = self.node;
        if !self.rt.hosted.iter().any(|h| h.requester == node && h.state == HostedWindow::Open(window)) {
            return None;
        }
        let root = self.rt.windows.get(&window)?.app_root;
        let text = self.text.as_deref_mut().map(|t| -> &mut dyn TextEngine { t });
        build_root(self.rt, root, text, |_, cx| content(cx))
    }

    pub(crate) fn runtime_mut(&mut self) -> &mut Runtime {
        self.rt
    }

    /// The runtime, read only: the tree and the nodes' `Local`s, for a service that walks them.
    pub fn runtime(&self) -> &Runtime {
        self.rt
    }

    /// A cell's value without recording a read: state the build looks at but does not depend on.
    pub fn peek_state<T: 'static>(&self, state: State<T>) -> &T {
        self.rt.store.peek(state)
    }

    pub fn env(&self) -> Env {
        self.rt.env_of(self.node)
    }

    pub fn tokens(&mut self) -> Arc<Tokens> {
        let tokens = self.env().tokens;
        tokens.get(self)
    }

    /// The window's scale, read in build: a scale change runs the reading node again.
    pub fn scale(&mut self) -> f32 {
        let scale = self.env().scale;
        scale.get(self)
    }

    /// The frame's text engine; `None` outside an inline root (an element's own build measures
    /// text in layout). Lending it records a read of the font stack's generation, so a stack
    /// change runs the node again.
    pub fn text_engine(&mut self) -> Option<&mut dyn TextEngine> {
        self.read_text_stack();
        self.text.as_deref_mut().map(|t| -> &mut dyn TextEngine { t })
    }

    /// Shapes `text` at the window's scale with the frame's engine; `None` outside an inline root.
    pub fn text_layout_rich(&mut self, text: &RichText) -> Option<TextLayout> {
        let scale = self.scale();
        self.read_text_stack();
        Some(self.text.as_deref_mut()?.layout_rich(text, scale))
    }

    fn read_text_stack(&mut self) {
        if self.text.is_some() {
            let stack = self.env().text_stack;
            stack.get(self);
        }
    }

    /// The text engine and the image store together, for code that lays out text and registers
    /// images while it records a paint (a component's run). Lending the engine records a read of
    /// the font stack's generation, as [`Self::text_engine`] does.
    pub fn paint_services(&mut self) -> (Option<&mut dyn TextEngine>, &mut crate::display::ImageStore) {
        self.read_text_stack();
        (self.text.as_deref_mut().map(|t| -> &mut dyn TextEngine { t }), &mut self.rt.images)
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

    /// A cell owned by this node under `key`, created with `init` on first use and freed with the
    /// node: per-node memory that needs no id map. The same key gives the same cell in every build.
    pub fn slot<T: 'static>(&mut self, key: impl Hash, init: impl FnOnce() -> T) -> State<T> {
        let key = hash_key(&(std::any::TypeId::of::<T>(), key));
        match self.rt.tree.nodes.contains_key(&self.node) {
            true => self.keyed_slot(self.node, key, init).expect("the node is in the tree"),
            false => self.rt.store.create(init()),
        }
    }

    /// [`slot`](Self::slot) on another node of the tree: the cell `node` owns under `key`, made on first use and freed
    /// with `node`. Code that finds a node through its own id map keeps a fact there that the node's readers track
    /// (an opener's open state, written by the popup it opens). `None` when `node` is not in the tree.
    pub fn slot_of<T: 'static>(&mut self, node: NodeId, key: impl Hash, init: impl FnOnce() -> T) -> Option<State<T>> {
        let key = hash_key(&(std::any::TypeId::of::<T>(), key));
        self.keyed_slot(node, key, init)
    }

    /// The cell `node` owns under `key` ([`slot_of`](Self::slot_of)) when it was made; never makes one. A node asks
    /// this of itself for a fact only some nodes carry, so the others own no cell.
    pub fn find_slot_of<T: 'static>(&self, node: NodeId, key: impl Hash) -> Option<State<T>> {
        let key = hash_key(&(std::any::TypeId::of::<T>(), key));
        let n = self.rt.tree.nodes.get(&node)?;
        n.slots.iter().find(|(k, _)| *k == key).map(|(_, at)| State::from_slot(*at))
    }

    fn keyed_slot<T: 'static>(&mut self, node: NodeId, key: u64, init: impl FnOnce() -> T) -> Option<State<T>> {
        let n = self.rt.tree.nodes.get(&node)?;
        if let Some((_, at)) = n.slots.iter().find(|(k, _)| *k == key) {
            return Some(State::from_slot(*at));
        }
        let state = self.rt.store.create(init());
        self.rt.tree.nodes.get_mut(&node)?.slots.push((key, state.slot_ref()));
        Some(state)
    }

    /// A derived cell owned by this node, freed with it. Its readers are marked when an input it
    /// read is written, at the start of the pass: a scope derives its env values from its parent's
    /// cells this way, so the nodes that read them rerun in the frame the host writes the input.
    pub fn derived<T: 'static>(&mut self, compute: impl Fn(&mut crate::state::DeriveCx<'_>) -> T + 'static) -> crate::state::Derived<T> {
        let derived = self.rt.store.derive(compute);
        if let Some(n) = self.rt.tree.nodes.get_mut(&self.node) {
            n.owned.push(derived.state().slot_ref());
        }
        derived
    }

    /// Provides `value` to this node and everything it owns: [`BuildCx::provided`] and
    /// [`crate::PaintCx::provided`] on those nodes find it. The nearest provider wins. Providing
    /// again replaces the value.
    pub fn provide<T: 'static>(&mut self, value: T) {
        let ty = std::any::TypeId::of::<T>();
        if let Some(n) = self.rt.tree.nodes.get_mut(&self.node) {
            let value: std::rc::Rc<dyn std::any::Any> = std::rc::Rc::new(value);
            match n.provided.iter_mut().find(|(t, _)| *t == ty) {
                Some(entry) => entry.1 = value,
                None => n.provided.push((ty, value)),
            }
        }
    }

    /// The nearest value of type `T` this node or a node above it provides.
    pub fn provided<T: Clone + 'static>(&self) -> Option<T> {
        self.rt.tree.provided::<T>(self.node)
    }

    /// The nearest value of type `T` a node above `node` provides, passing over what `node` provides
    /// itself: a request a node makes walks up through every watcher above it.
    pub fn provided_above<T: Clone + 'static>(&self, node: NodeId) -> Option<T> {
        self.rt.tree.provided_above::<T>(node)
    }

    /// Whether `node`'s inline build is running: this build happens inside it. A request for a
    /// node that is building reaches it in the run that is under way; marking it would build it
    /// again.
    pub fn is_building(&self, node: NodeId) -> bool {
        self.rt.tree.nodes.get(&node).is_some_and(|n| n.building)
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
        let text = self.text.as_deref_mut().map(|t| -> &mut dyn TextEngine { t });
        let mut cx =
            BuildCx { rt: &mut *self.rt, node: id, window, children: Vec::new(), layered: Vec::new(), pos: 0, scope: None, keep: false, text };
        let slot = boxed.as_any_mut().downcast_mut::<ElementBox<E>>().expect("the element's type was checked");
        let local = match &mut slot.local {
            Some(local) => local,
            None => slot.local.insert(slot.el.init(&mut cx)),
        };
        cx.rt.tree.nodes.get_mut(&id).expect("the node was just reconciled").building = true;
        let out = f(local, state, &mut cx);
        let BuildCx { children, layered, keep, .. } = cx;
        finish_children(self.rt, id, old, old_layered, children, layered, keep);
        if !self.rt.hosted.is_empty() {
            self.rt.settle_hosted(id, keep);
        }
        if let Some(node) = self.rt.tree.nodes.get_mut(&id) {
            node.building = false;
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

    /// This node builds again at frame time `at`. A node keeps its earliest pending build wake, so
    /// one deadline never pushes back another (a timeout and a dwell of one node); a wake no longer
    /// needed costs one build, which asks again for what it still waits on. `None` asks for nothing.
    pub fn request_build_at(&mut self, at: Option<f64>) {
        let node = self.node;
        self.rt.set_build_wake(node, at);
    }

    /// This node gets [`crate::Event::Hover`] when the pointer enters or leaves it or a node under
    /// it (a tooltip's owner times its delay from it), and [`crate::Event::HoverPart`] when the
    /// pointer moves to another of its parts meanwhile.
    pub fn hover_events(&mut self, on: bool) {
        if let Some(n) = self.rt.tree.nodes.get_mut(&self.node) {
            n.hover_events = on;
        }
    }

    /// While this node or a node under it holds the focus, a primary press no shape takes, or one on
    /// a shape that takes no focus, leaves the focus where it is (a trapping scope, a document
    /// canvas); elsewhere the first gives the focus up, and the second gives up the focus of a node
    /// that releases it ([`crate::Element::releases_focus`]).
    pub fn keep_focus_on_empty_presses(&mut self, on: bool) {
        if let Some(n) = self.rt.tree.nodes.get_mut(&self.node) {
            n.keeps_focus = on;
        }
    }

    /// A node under this one that gives its focus up on a press (a text field committing) hands it
    /// to this node, the nearest such above it, instead of to none: a pane, whose keys keep working
    /// after the click.
    pub fn take_released_focus(&mut self, on: bool) {
        if let Some(n) = self.rt.tree.nodes.get_mut(&self.node) {
            n.takes_released_focus = on;
        }
    }

    /// `node` builds in a later build round of this frame: a request addressed to it by id (a
    /// collection asked to start a rename) or a runtime-wide edit (a cancel of every running gesture)
    /// reached this build.
    pub fn request_build_of(&mut self, node: NodeId) {
        self.rt.tree.mark_build(node);
    }

    /// This node read the app model `source` (a `Tracked`'s source id) at `at`, its place in the order the caller
    /// gives every read and edit of a source on this thread: an edit of `source` after `at` in this frame, made before
    /// or after this call, runs this node again in a later build round of the frame.
    pub fn read_source(&mut self, source: u64, at: u64) {
        if self.rt.sources.read(source, self.node, at) {
            self.rebuild();
        }
    }

    /// An edit of the app model `source` at `at`: every node that read it before `at` in this frame runs again in a
    /// later build round. A node that only wrote it, or read it after the edit, runs nothing again.
    pub fn edit_source(&mut self, source: u64, at: u64) {
        for node in self.rt.sources.edit(source, at) {
            self.rt.tree.mark_build(node);
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

    /// Whether this node's window sends an access tree. A node builds its access node only then;
    /// turning the tree on rebuilds every node of the window once.
    pub fn access_enabled(&self) -> bool {
        self.rt.access_enabled(self.window)
    }

    /// `node`'s access node is rebuilt in this frame (a relation of it named a node that exists now).
    pub fn request_access_of(&mut self, node: NodeId) {
        self.rt.access_dirty(node);
    }

    /// Starts a drag session from this node's open press, as `EventCx::start_drag` does from an
    /// event: for a build that decided to drag (a run's `update` asking for it). Returns false (and
    /// starts nothing) when no press of the node's window runs or a drag already runs.
    pub fn start_drag(&mut self, start: crate::DragStart) -> bool {
        let node = self.node;
        self.rt.start_drag(node, start)
    }

    /// Ends the composition of `node`'s IME session ([`crate::ImeRequest::Reset`]): its editor
    /// refused an input method edit, or ended the composition itself, so the platform's view of the
    /// text is stale. Late events of the composition are dropped. Nothing happens when the
    /// window's session is not `node`'s. `node` is this node or the component a body reports for.
    pub fn interrupt_ime(&mut self, node: NodeId) {
        self.rt.interrupt_ime(node);
    }

    /// Puts `text` on the platform's clipboard for this node's window (a selection's Copy).
    pub fn set_clipboard(&mut self, text: impl Into<String>) {
        let window = self.window;
        self.rt.clipboard.push((window, crate::input::ClipboardRequest::Set(text.into())));
    }

    /// Asks the platform for the clipboard's text; it arrives as [`crate::Event::Paste`] at the
    /// focused node of this node's window (an editor's Paste).
    pub fn request_paste(&mut self) {
        let window = self.window;
        self.rt.clipboard.push((window, crate::input::ClipboardRequest::Read));
    }

    /// Ends `window`'s open press as the host did (`PressCancel::Host`): its owner gets the cancel.
    pub fn cancel_press(&mut self, reason: craft_behavior::PressCancel) {
        let window = self.window;
        self.rt.cancel_press(window, reason);
    }

    /// Announces `text` to assistive technology in this node's window: the texts of one frame
    /// become one live status node. The same text made twice in a frame is kept once; nothing
    /// is kept while the access tree is off.
    pub fn announce(&mut self, text: impl Into<String>, politeness: crate::access::Politeness) {
        let window = self.window;
        if let Some(w) = self.rt.windows.get_mut(&window).filter(|w| w.access.enabled) {
            w.access.announcer.announce(text.into(), politeness);
        }
    }

    /// This node's outputs changed: its layout, paint and access node run in this frame.
    pub fn mark_changed(&mut self) {
        let node = self.node;
        self.rt.access_dirty(node);
        self.rt.tree.mark_layout(node);
        self.rt.tree.mark_paint(node);
    }

    /// Puts `list` (in `node`'s local coordinates) in `node`'s paint slot `key` in place of what it held, and
    /// repaints `node`: its paint replays the lists it recorded with the slot's new items in place, and nothing
    /// builds. A slot the node's lists no longer reserve drops its items at that paint.
    pub fn fill_slot(&mut self, node: NodeId, key: u64, list: crate::display::DisplayList) {
        let Some(n) = self.rt.tree.nodes.get_mut(&node) else { return };
        let list = std::sync::Arc::new(list);
        match n.slot_lists.iter_mut().find(|(k, _)| *k == key) {
            Some((_, kept)) => *kept = list,
            None => n.slot_lists.push((key, list)),
        }
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

    /// Counts a run of a contract component's `measure` hook in the frame's report.
    pub fn count_measure(&mut self) {
        self.rt.counters.measures += 1;
    }

    /// Counts an access node a contract run built in the frame's report.
    pub fn count_access_build(&mut self) {
        self.rt.counters.access_builds += 1;
    }

    /// Counts a run of a scope's closure in the frame's report.
    pub fn count_scope(&mut self) {
        self.rt.counters.scopes += 1;
    }

    /// For the layer child being built: what its modal block lets through ([`crate::ModalReach`]),
    /// as its content decided in this build. Its owner's next show of it keeps it in the spec.
    pub fn set_layer_reach(&mut self, reach: Option<crate::ModalReach>) {
        let node = self.node;
        self.set_layer_reach_of(node, reach);
    }

    /// [`set_layer_reach`](Self::set_layer_reach) for the layer child `node` of this node's window, from another
    /// node's build: a canvas's host passed its running tool's gestures, which the modal over it lets through at once.
    pub fn set_layer_reach_of(&mut self, node: NodeId, reach: Option<crate::ModalReach>) {
        if let Some(spec) = self.rt.windows.get_mut(&self.window).and_then(|w| w.layered.get_mut(&node)) {
            spec.reach = reach;
        }
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

    /// Brings the layer child `node`, with the layer children opened from inside it on the same layer, in front of
    /// the other children of its layer, as a press on a floating window does. Nothing for a node that is not a layer
    /// child, or one in front already.
    pub fn raise_layer_child(&mut self, node: NodeId) {
        let Some(root) = self.rt.window_node(self.window) else { return };
        let Some(layer) = self.rt.tree.nodes.get(&node).and_then(|n| n.parent) else { return };
        if self.rt.tree.nodes.get(&layer).and_then(|n| n.parent) != Some(root) {
            return;
        }
        let Some(children) = self.rt.tree.nodes.get(&layer).map(|n| n.children.clone()) else { return };
        let tree = &self.rt.tree;
        // A layer child belongs to the family when its owner chain (parents, and from a layer child its owner) meets `node`.
        let in_family = |child: NodeId| {
            let mut at = child;
            loop {
                if at == node {
                    return true;
                }
                let Some(n) = tree.nodes.get(&at) else { return false };
                let up = match n.parent {
                    Some(parent) if tree.nodes.get(&parent).and_then(|p| p.parent) == Some(root) => n.owner,
                    parent => parent,
                };
                match up {
                    Some(up) if up != root => at = up,
                    _ => return false,
                }
            }
        };
        let (family, rest): (Vec<NodeId>, Vec<NodeId>) = children.iter().partition(|&&child| in_family(child));
        let raised: Vec<NodeId> = rest.into_iter().chain(family).collect();
        if raised == children {
            return;
        }
        if let Some(n) = self.rt.tree.nodes.get_mut(&layer) {
            n.children = raised;
        }
        self.rt.tree.mark_layout(layer);
        self.rt.tree.mark_paint(layer);
        if let Some(w) = self.rt.windows.get_mut(&self.window) {
            w.relayout = true;
        }
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
        text: None,
    };
    el.build(&mut cx);
    let BuildCx { children, layered, keep, .. } = cx;
    finish_children(rt, id, old, old_layered, children, layered, keep);
    if !rt.hosted.is_empty() {
        rt.settle_hosted(id, keep);
    }
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
pub(crate) fn build_inline_root(rt: &mut Runtime, id: NodeId, content: &mut dyn FnMut(WindowId, &mut BuildCx<'_>), text: &mut dyn TextEngine) {
    let _ = build_root(rt, id, Some(text), |window, cx| content(window, cx));
}

/// The build of the inline root `id`, by the host's `roots` closure or a hosting node's build
/// (`BuildCx::window_content`); `None` when the node is gone.
fn build_root<R>(
    rt: &mut Runtime,
    id: NodeId,
    text: Option<&mut dyn TextEngine>,
    content: impl FnOnce(WindowId, &mut BuildCx<'_>) -> R,
) -> Option<R> {
    let node = rt.tree.nodes.get_mut(&id)?;
    node.dirty.remove(Dirty::BUILD);
    node.dirty.remove(Dirty::PATH);
    let window = node.window;
    let mut el = node.element.take()?;
    let old = std::mem::take(&mut node.children);
    let old_layered = std::mem::take(&mut node.layered);
    // A reborrow for the build alone: the engine outlives this call, the build's borrow of `rt` must not.
    let text = text.map(|t| -> &mut dyn TextEngine { t });
    let mut cx = BuildCx { rt, node: id, window, children: Vec::new(), layered: Vec::new(), pos: 0, scope: None, keep: false, text };
    // Creates the root's `Local` on the first build; the element's own build adds nothing.
    el.build(&mut cx);
    let out = content(window, &mut cx);
    let BuildCx { children, layered, keep, .. } = cx;
    if let Some(node) = rt.tree.nodes.get_mut(&id) {
        node.element = Some(el);
    }
    let changed = !keep && (children != old || layered != old_layered);
    finish_children(rt, id, old, old_layered, children, layered, keep);
    if !rt.hosted.is_empty() {
        rt.settle_hosted(id, keep);
    }
    if changed {
        rt.tree.mark_layout(id);
    }
    Some(out)
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
