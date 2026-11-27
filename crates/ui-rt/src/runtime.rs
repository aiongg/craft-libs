use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use craft_host::target::Os;
use craft_theme::Tokens;
use emath::{Pos2, Rect, Vec2};
use smallvec::SmallVec;
use web_time::Instant;

use crate::build::{BuildCx, build_inline_root, build_node};
use crate::display::{ImageStore, Media, SurfaceKey};
use crate::drag::{DragInfo, Session};
use crate::element::{Element, ElementBox, InlineRoot};
use crate::event::CloseReason;
use crate::frame::{FrameTree, merge_damage, node_damage, parent_space};
use crate::geom::{Margin, is_empty};
use crate::gesture::WheelOptions;
use crate::id::{IdMap, NodeId, WindowId};
use crate::input::{ClipboardRequest, Interact, InteractFact};
use crate::layer::Layer;
use crate::layout::{layout_pass, place_layers};
use crate::paint::paint_pass;
use crate::platform::{Cursor, ImeRequest, PlatformEvent, WindowCommand, WindowEvent, WindowSpec};
use crate::state::{Access, Reads, State, Store, Writes};
use crate::text::TextEngine;
use crate::tree::{Node, Role, Tree};
use crate::window::{Env, Window, WindowGeometry, Windows};

/// Each runtime numbers its display-list versions in its own range, so a renderer that outlives a
/// runtime (a bench resetting the document) never mistakes a new node's list for a kept one.
static NEXT_EPOCH: AtomicU64 = AtomicU64::new(0);

/// Build and layout alternate while layout writes cells that builds read (the virtual list hook).
/// Work left after this many rounds runs in the next frame.
const MAX_ROUNDS: u32 = 4;

/// What one frame ran, over every window. The counts let checks assert that a frame ran only what
/// it had to.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameReport {
    pub build_ms: f64,
    pub layout_ms: f64,
    pub paint_ms: f64,
    /// Build and layout rounds (more than one when layout asked for a rebuild).
    pub rounds: u32,
    pub built: u32,
    pub laid_out: u32,
    pub painted: u32,
    /// Runs of contract component hooks inside inline builds ([`crate::BuildCx::count_run`]).
    pub runs: u32,
    /// Runs of scope closures inside inline builds, the window roots included
    /// ([`crate::BuildCx::count_scope`]).
    pub scopes: u32,
    /// Runs of contract component `measure` hooks ([`crate::BuildCx::count_measure`]).
    pub measures: u32,
    /// Access nodes contract runs built ([`crate::BuildCx::count_access_build`]).
    pub access_builds: u32,
    pub access_ms: f64,
    /// Nodes in the frame's `TreeUpdate`s.
    pub access_nodes: u32,
}

#[derive(Default)]
pub(crate) struct Counters {
    pub(crate) built: u32,
    pub(crate) laid_out: u32,
    pub(crate) painted: u32,
    pub(crate) runs: u32,
    pub(crate) scopes: u32,
    pub(crate) measures: u32,
    pub(crate) access_builds: u32,
}

/// The cells every window shares.
#[derive(Clone, Copy)]
struct Shared {
    tokens: State<Arc<Tokens>>,
    time: State<f64>,
    wakes: State<u64>,
    drag: State<Option<DragInfo>>,
}

/// Gives a new window its root.
type SetRoot = Box<dyn FnOnce(&mut Runtime, WindowId)>;

/// A window a node asked for ([`crate::EventCx::open_window`]); the driver opens it.
pub(crate) struct WindowOpen {
    pub(crate) spec: WindowSpec,
    pub(crate) root: SetRoot,
    pub(crate) requester: NodeId,
}

/// The retained runtime of every window of one app.
pub struct Runtime {
    pub(crate) tree: Tree,
    pub(crate) store: Store,
    shared: Shared,
    pub(crate) windows: Windows,
    pub(crate) images: ImageStore,
    pub(crate) counters: Counters,
    pub(crate) version_clock: u64,
    /// The step's input time in seconds; every event of a step shares it.
    pub(crate) time: f64,
    /// Per-node interaction cells, created when paint first reads one.
    pub(crate) interact: IdMap<State<Interact>>,
    /// Per-node cells of single interaction facts, created when a build first reads one.
    pub(crate) interact_facts: IdMap<[Option<State<bool>>; InteractFact::COUNT]>,
    /// Per-node cells of the part of the node the pointer hovers, created when paint first reads one.
    pub(crate) hovered_parts: IdMap<State<Option<u64>>>,
    /// Per-node cells of the pointer's place while it hovers the node or a node under it, created
    /// when a build first reads one.
    pub(crate) hover_points: IdMap<State<Option<Pos2>>>,
    pub(crate) clipboard: Vec<(WindowId, ClipboardRequest)>,
    /// Nodes that repaint at a frame time: (node, time), one entry per node.
    pub(crate) wakes: Vec<(NodeId, f64)>,
    /// Nodes that lay out again at a frame time (`LayoutCx::time_until`): (node, time).
    pub(crate) layout_wakes: Vec<(NodeId, f64)>,
    /// Nodes that build again at a frame time (`BuildCx::request_build_at`,
    /// `EventCx::request_build_at`): (node, time), one entry per node.
    pub(crate) build_wakes: Vec<(NodeId, f64)>,
    /// Nodes showing each image or surface.
    pub(crate) media: HashMap<Media, SmallVec<[NodeId; 2]>>,
    pub(crate) window_commands: Vec<(WindowId, WindowCommand)>,
    pub(crate) window_opens: Vec<WindowOpen>,
    /// Open windows, bottom of the stack first: the last focused is on top, with the windows it
    /// owns above it.
    pub(crate) z_order: Vec<WindowId>,
    pub(crate) drag: Option<Session>,
    pub(crate) cursor_requests: Vec<(WindowId, Cursor)>,
    /// The system the app runs on (the browser's on the web): drop operations and the Ctrl+wheel
    /// pinch depend on it.
    pub(crate) os: Os,
    pub(crate) wheel: WheelOptions,
    /// A window lost key focus since the last frame: if none has it at the next frame, the app
    /// lost it, and layer children that go when the user turns away get [`crate::DismissReason::Away`].
    pub(crate) away_check: bool,
}

impl Reads for Runtime {
    fn access(&mut self) -> Access<'_> {
        self.store.access()
    }
}

impl Writes for Runtime {
    fn store_mut(&mut self) -> &mut Store {
        &mut self.store
    }
}

impl Runtime {
    /// A runtime with no window yet; [`Runtime::add_window`] or [`Runtime::set_root`] add them.
    pub fn new(tokens: Tokens) -> Runtime {
        let mut store = Store::default();
        let shared = Shared { tokens: store.create(Arc::new(tokens)), time: store.create(0.0), wakes: store.create(0), drag: store.create(None) };
        Runtime {
            tree: Tree::default(),
            store,
            shared,
            windows: Windows::new(),
            images: ImageStore::default(),
            counters: Counters::default(),
            version_clock: NEXT_EPOCH.fetch_add(1, Ordering::Relaxed) << 32,
            time: 0.0,
            interact: IdMap::default(),
            interact_facts: IdMap::default(),
            hovered_parts: IdMap::default(),
            hover_points: IdMap::default(),
            clipboard: Vec::new(),
            wakes: Vec::new(),
            layout_wakes: Vec::new(),
            build_wakes: Vec::new(),
            media: HashMap::new(),
            window_commands: Vec::new(),
            window_opens: Vec::new(),
            z_order: Vec::new(),
            drag: None,
            cursor_requests: Vec::new(),
            os: Os::current(),
            wheel: WheelOptions::default(),
            away_check: false,
        }
    }

    pub fn os(&self) -> Os {
        self.os
    }

    /// Sets the system the app runs on (a web platform reports the browser's).
    pub fn set_os(&mut self, os: Os) {
        self.os = os;
    }

    pub fn wheel_options(&self) -> WheelOptions {
        self.wheel
    }

    pub fn set_wheel_options(&mut self, options: WheelOptions) {
        self.wheel = options;
    }

    /// Records that `window` is attached to `owner` and stacks above it (a floating panel window).
    pub fn set_window_owner(&mut self, window: WindowId, owner: Option<WindowId>) {
        if let Some(w) = self.windows.get_mut(&window) {
            w.owner = owner;
        }
        if let Some(owner) = owner
            && self.z_order.last() == Some(&owner)
        {
            self.raise(owner);
        }
    }

    /// Puts `window` on top of the stack, with the windows it owns above it.
    fn raise(&mut self, window: WindowId) {
        if !self.windows.contains_key(&window) {
            return;
        }
        self.z_order.retain(|&w| w != window);
        self.z_order.push(window);
        let owned: Vec<WindowId> = self.z_order.iter().copied().filter(|w| self.windows.get(w).is_some_and(|x| x.owner == Some(window))).collect();
        self.z_order.retain(|w| !owned.contains(w));
        self.z_order.extend(owned);
    }

    pub(crate) fn drag_cell(&self) -> State<Option<DragInfo>> {
        self.shared.drag
    }

    pub(crate) fn queue_window<E: Element>(&mut self, requester: NodeId, spec: WindowSpec, root: E) {
        self.window_opens.push(WindowOpen { spec, root: Box::new(move |rt: &mut Runtime, window| rt.set_root(window, root)), requester });
    }

    pub(crate) fn queue_inline_window(&mut self, requester: NodeId, spec: WindowSpec) {
        self.window_opens.push(WindowOpen { spec, root: Box::new(|rt: &mut Runtime, window| rt.set_inline_root(window)), requester });
    }

    pub(crate) fn take_window_opens(&mut self) -> Vec<WindowOpen> {
        std::mem::take(&mut self.window_opens)
    }

    /// Hands `ev` to `node` alone, as the runtime's own events reach a node (the driver tells a
    /// node its window opened).
    pub(crate) fn notify(&mut self, node: NodeId, ev: &crate::event::Event) {
        self.deliver(node, node, ev);
    }

    /// Adds `window` with its fixed nodes (root and layers) and no app root. Nothing happens when
    /// it is there already.
    pub fn add_window(&mut self, window: WindowId, viewport: Vec2, scale: f32) {
        if self.windows.contains_key(&window) {
            return;
        }
        let env = Env {
            window,
            viewport: self.store.create(viewport),
            scale: self.store.create(scale),
            safe_area: self.store.create(Margin::ZERO),
            window_focused: self.store.create(true),
            tokens: self.shared.tokens,
            time: self.shared.time,
            wakes: self.shared.wakes,
            drag: self.shared.drag,
            modifiers: self.store.create(craft_keymap::chord::Mods::NONE),
            caps_lock: self.store.create(false),
            num_lock: self.store.create(false),
            held_keys: self.store.create(Vec::new()),
            pointer: self.store.create(crate::window::PointerFacts::default()),
        };
        let w = Window::new(window, env, viewport);
        let mut root = Node::new(None, None, window, 0, None, 0);
        root.role = Role::WindowRoot;
        root.children = w.layers.to_vec();
        root.frame.size = viewport;
        root.owned = vec![
            env.viewport.slot_ref(),
            env.scale.slot_ref(),
            env.safe_area.slot_ref(),
            env.window_focused.slot_ref(),
            env.modifiers.slot_ref(),
            env.caps_lock.slot_ref(),
            env.num_lock.slot_ref(),
            env.held_keys.slot_ref(),
            env.pointer.slot_ref(),
        ];
        self.tree.nodes.insert(w.root, root);
        for (layer, id) in Layer::ALL.into_iter().zip(w.layers) {
            let mut node = Node::new(Some(w.root), None, window, 0, None, 0);
            node.role = Role::Layer(layer);
            node.frame.size = viewport;
            self.tree.nodes.insert(id, node);
        }
        self.windows.insert(window, w);
        self.z_order.push(window);
    }

    /// Removes `window` with every node in it. The driver calls it on [`WindowEvent::Closed`].
    pub fn remove_window(&mut self, window: WindowId) {
        let Some(root) = self.windows.get(&window).map(|w| w.root) else {
            return;
        };
        if self.drag.as_ref().is_some_and(|s| s.origin == window) {
            self.cancel_drag(craft_behavior::PressCancel::Host);
        }
        self.remove_subtree(root);
        self.windows.remove(&window);
        self.z_order.retain(|&w| w != window);
        self.wakes.retain(|&(n, _)| self.tree.nodes.contains_key(&n));
        self.layout_wakes.retain(|&(n, _)| self.tree.nodes.contains_key(&n));
        self.build_wakes.retain(|&(n, _)| self.tree.nodes.contains_key(&n));
    }

    /// The open windows.
    pub fn windows(&self) -> impl Iterator<Item = WindowId> + '_ {
        self.windows.keys().copied()
    }

    /// Replaces `window`'s app root with `root`, adding the window (empty size, scale 1 until the
    /// platform reports them) when it is new.
    pub fn set_root<E: Element>(&mut self, window: WindowId, root: E) {
        self.install_root(window, root, false);
    }

    /// Gives `window` an inline app root: its content is the closure the host passes to
    /// [`Runtime::frame_with`] in each frame that must reach it (a node under it is marked, or a
    /// read in the closure changed). Adds the window as [`Runtime::set_root`] does.
    pub fn set_inline_root(&mut self, window: WindowId) {
        self.install_root(window, InlineRoot, true);
    }

    /// Builds `window`'s inline root in the next frame although nothing under it was marked and
    /// no read changed: the host's own state, which the closure reads, moved.
    pub fn mark_root(&mut self, window: WindowId) {
        if let Some(w) = self.windows.get(&window) {
            let id = w.app_root;
            self.tree.mark_build(id);
        }
    }

    fn install_root<E: Element>(&mut self, window: WindowId, root: E, inline: bool) {
        self.add_window(window, Vec2::ZERO, 1.0);
        let Some(w) = self.windows.get(&window) else { return };
        let (id, content) = (w.app_root, w.layer(Layer::Content));
        self.remove_subtree(id);
        let props_key = root.props_key();
        let mut node = Node::new(Some(content), None, window, 1, Some(Box::new(ElementBox::new(root))), props_key);
        node.boundary = true;
        node.inline = inline;
        self.tree.nodes.insert(id, node);
        if let Some(layer) = self.tree.nodes.get_mut(&content) {
            layer.children = vec![id];
        }
        self.tree.mark_build(id);
        if let Some(w) = self.windows.get_mut(&window) {
            w.changes.full = true;
            w.access.full = true;
        }
    }

    pub fn store(&self) -> &Store {
        &self.store
    }

    pub fn store_mut(&mut self) -> &mut Store {
        &mut self.store
    }

    /// `window`'s environment cells.
    ///
    /// # Panics
    /// When the window is not open.
    pub fn env(&self, window: WindowId) -> Env {
        self.windows[&window].env
    }

    /// The environment of the window `node` is in.
    pub(crate) fn env_of(&self, node: NodeId) -> Env {
        let window = self.tree.window_of(node);
        match window.and_then(|w| self.windows.get(&w)) {
            Some(w) => w.env,
            None => self.windows.values().next().map(|w| w.env).expect("a node's window is open"),
        }
    }

    pub fn images(&self) -> &ImageStore {
        &self.images
    }

    pub fn images_mut(&mut self) -> &mut ImageStore {
        &mut self.images
    }

    /// The cells and the images together, for a host that delivers background results into both.
    pub fn store_and_images_mut(&mut self) -> (&mut Store, &mut ImageStore) {
        (&mut self.store, &mut self.images)
    }

    pub(crate) fn tokens_cell(&self) -> State<Arc<Tokens>> {
        self.shared.tokens
    }

    pub fn set_tokens(&mut self, tokens: Tokens) {
        self.shared.tokens.set(&mut self.store, Arc::new(tokens));
    }

    /// `window`'s app root, once [`Runtime::set_root`] gave it one.
    pub fn root(&self, window: WindowId) -> Option<NodeId> {
        let id = self.windows.get(&window)?.app_root;
        self.tree.nodes.contains_key(&id).then_some(id)
    }

    /// `window`'s fixed root: its children are the window's layers, bottom first, and the app root
    /// is the content layer's child.
    pub fn window_node(&self, window: WindowId) -> Option<NodeId> {
        Some(self.windows.get(&window)?.root)
    }

    /// `node`'s parent: its layer for a layer child, the layer's window root for a layer.
    pub fn parent(&self, node: NodeId) -> Option<NodeId> {
        self.tree.nodes.get(&node)?.parent
    }

    /// `node`'s children in order (a layer's children are its layer children, bottom first).
    pub fn children(&self, node: NodeId) -> &[NodeId] {
        self.tree.nodes.get(&node).map_or(&[], |n| n.children.as_slice())
    }

    /// The `Local` of `node` when its element is an `E`. A node whose own hook is running (the
    /// build or the event handler that asks) has none to give.
    pub fn local<E: Element>(&self, node: NodeId) -> Option<&E::Local> {
        self.tree.nodes.get(&node)?.element.as_ref()?.as_any().downcast_ref::<ElementBox<E>>()?.local.as_ref()
    }

    /// The window a node is in.
    pub fn window_of(&self, node: NodeId) -> Option<WindowId> {
        self.tree.window_of(node)
    }

    /// `node`'s rect in window coordinates.
    pub fn window_rect(&self, node: NodeId) -> Option<Rect> {
        let n = self.tree.nodes.get(&node)?;
        let origin = crate::hit::window_origin(&self.tree, node);
        Some(Rect::from_min_size(origin.to_pos2(), n.frame.size))
    }

    /// Applies one platform event. Surface events that concern the renderer are the driver's.
    pub fn handle_event(&mut self, ev: &PlatformEvent) {
        match ev {
            PlatformEvent::Window(window, ev) => self.handle_window_event(*window, ev),
            PlatformEvent::Wake => {
                let wakes = self.shared.wakes;
                wakes.update(&mut self.store, |v| *v = v.wrapping_add(1));
            }
            PlatformEvent::Resume => self.invalidate_pixels(),
            PlatformEvent::Suspend => {}
        }
    }

    fn handle_window_event(&mut self, window: WindowId, ev: &WindowEvent) {
        if matches!(ev, WindowEvent::Closed) {
            self.remove_window(window);
            return;
        }
        let Some(w) = self.windows.get_mut(&window) else {
            return;
        };
        let env = w.env;
        match ev {
            WindowEvent::Scale(scale) => {
                env.scale.set_if_changed(&mut self.store, *scale);
                w.changes.full = true;
            }
            WindowEvent::Resize(size) => {
                let size = *size;
                if w.viewport != size {
                    w.viewport = size;
                    w.changes.full = true;
                    w.relayout = true;
                    let (root, layers, app_root) = (w.root, w.layers, w.app_root);
                    env.viewport.set(&mut self.store, size);
                    for id in std::iter::once(root).chain(layers) {
                        if let Some(n) = self.tree.nodes.get_mut(&id) {
                            n.frame.size = size;
                        }
                    }
                    self.tree.mark_layout(app_root);
                }
            }
            WindowEvent::SafeArea(margin) => env.safe_area.set_if_changed(&mut self.store, *margin),
            WindowEvent::LockKeys { caps_lock, num_lock } => {
                env.caps_lock.set_if_changed(&mut self.store, *caps_lock);
                env.num_lock.set_if_changed(&mut self.store, *num_lock);
            }
            WindowEvent::Focus(focused) => {
                env.window_focused.set_if_changed(&mut self.store, *focused);
                // The IME session follows the window with key focus.
                w.input.focus_moved = true;
                self.away_check = !*focused;
                if *focused {
                    self.raise(window);
                } else {
                    self.cancel_press(window, craft_behavior::PressCancel::FocusLost);
                }
            }
            WindowEvent::Moved(pos) => w.position = Some(*pos),
            WindowEvent::State { maximized, fullscreen } => (w.maximized, w.fullscreen) = (*maximized, *fullscreen),
            WindowEvent::FrameInset(inset) => w.frame_inset = *inset,
            WindowEvent::AccessActivated => self.set_access_enabled(window, true),
            WindowEvent::SurfaceCreated => w.changes.full = true,
            WindowEvent::SurfaceLost => {}
            WindowEvent::CloseRequested => self.request_close(window, CloseReason::Window),
            WindowEvent::Closed => {}
            // Pasted images and the system theme reach nodes through the clipboard and env cell
            // services that give them a consumer.
            WindowEvent::PasteImage(_) | WindowEvent::Theme(_) => {}
            WindowEvent::Pointer(_)
            | WindowEvent::Wheel { .. }
            | WindowEvent::Key { .. }
            | WindowEvent::Text(_)
            | WindowEvent::ImePreedit { .. }
            | WindowEvent::ImeCommit(_)
            | WindowEvent::ImeEnabled
            | WindowEvent::Paste(_)
            | WindowEvent::Pinch { .. }
            | WindowEvent::Files(_)
            | WindowEvent::Access(_) => self.route_input(window, ev),
        }
    }

    /// The one close path: the user's close request ([`WindowEvent::CloseRequested`]) and
    /// [`Runtime::quit`] both end here. A window another window's app-modal child blocks refuses
    /// (its blocking window takes focus); a window with a close guard ([`crate::PaintCx::close_guard`])
    /// asks its topmost guard, which closes the window or keeps it; any other window closes.
    pub fn request_close(&mut self, window: WindowId, reason: CloseReason) {
        if !self.windows.contains_key(&window) {
            return;
        }
        if let Some(modal) = self.blocked_by(window) {
            self.window_command(modal, WindowCommand::Focus);
            return;
        }
        match self.windows.get(&window).and_then(|w| w.close_guards.last().copied()) {
            Some(guard) => {
                self.deliver(guard, guard, &crate::event::Event::CloseRequested(reason));
            }
            None => self.window_command(window, WindowCommand::Close),
        }
    }

    /// Asks every window to close, the top of the stack first, each once, through
    /// [`Runtime::request_close`]. The app has quit when its last window is gone.
    pub fn quit(&mut self) {
        let windows: Vec<WindowId> = self.z_order.iter().rev().copied().collect();
        for window in windows {
            self.request_close(window, CloseReason::Quit);
        }
    }

    /// The places of `window` the operating system acts on ([`crate::PaintCx::window_zone`]), each
    /// with the input shapes above it. The host publishes them to the platform's hit test.
    pub fn window_zones(&self, window: WindowId) -> Vec<crate::hit::WindowZone> {
        self.windows.get(&window).map(|w| crate::hit::window_zones(&self.tree, w)).unwrap_or_default()
    }

    /// The name the app gave `window` ([`WindowSpec::key`]).
    pub fn window_key(&self, window: WindowId) -> Option<&str> {
        self.windows.get(&window)?.key.as_deref()
    }

    /// Records the app's name for `window`.
    pub fn set_window_key(&mut self, window: WindowId, key: Option<String>) {
        if let Some(w) = self.windows.get_mut(&window) {
            w.key = key;
        }
    }

    /// Where `window` is and how big, as the platform last reported.
    pub fn window_geometry(&self, window: WindowId) -> Option<WindowGeometry> {
        let w = self.windows.get(&window)?;
        Some(WindowGeometry {
            size: w.viewport,
            scale: *self.store.peek(w.env.scale),
            position_px: w.position,
            maximized: w.maximized,
            fullscreen: w.fullscreen,
            frame_inset: w.frame_inset,
        })
    }

    /// Asks the platform to change a window ([`crate::Driver::step`] passes it on).
    pub fn window_command(&mut self, window: WindowId, cmd: WindowCommand) {
        self.window_commands.push((window, cmd));
    }

    pub(crate) fn take_window_commands(&mut self) -> Vec<(WindowId, WindowCommand)> {
        std::mem::take(&mut self.window_commands)
    }

    /// Something is dirty or some window's pixels are stale.
    pub fn needs_frame(&self) -> bool {
        self.store.has_pending()
            || self.tree.has_work()
            || !self.images.changes().is_empty()
            || !self.images.touched.is_empty()
            || self
                .windows
                .values()
                .any(|w| w.changes.any() || w.relayout || w.input.focus_moved || (self.tree.nodes.contains_key(&w.app_root) && w.access.pending()))
    }

    /// Something reads the frame time, so the next frame must come even without input.
    pub fn wants_time(&self) -> bool {
        self.store.has_readers(self.shared.time)
    }

    pub fn set_time(&mut self, time: f64) {
        self.shared.time.set(&mut self.store, time);
    }

    /// Queues the paint of every node whose wake time (`PaintCx::time_until`) is at or before
    /// `now`, the layout of every node whose layout wake (`LayoutCx::time_until`) is, and the build
    /// of every node whose build wake is.
    pub fn wake_due(&mut self, now: f64) {
        let tree = &mut self.tree;
        self.build_wakes.retain(|&(id, at)| {
            let due = at <= now;
            if due {
                tree.mark_build(id);
            }
            !due
        });
        self.wakes.retain(|&(id, at)| {
            let due = at <= now;
            if due {
                tree.mark_paint(id);
            }
            !due
        });
        self.layout_wakes.retain(|&(id, at)| {
            let due = at <= now;
            if due {
                tree.mark_layout(id);
            }
            !due
        });
    }

    /// The earliest frame time a node asked to repaint, lay out or build at.
    pub fn next_wake(&self) -> Option<f64> {
        self.wakes.iter().chain(&self.layout_wakes).chain(&self.build_wakes).map(|&(_, at)| at).reduce(f64::min)
    }

    /// `node` builds at frame time `at` (an inline node: its owners take `PATH`); it keeps the
    /// earlier of this and a pending build wake. `None` asks for nothing.
    pub(crate) fn set_build_wake(&mut self, node: NodeId, at: Option<f64>) {
        let Some(at) = at else { return };
        match self.build_wakes.iter_mut().find(|(n, _)| *n == node) {
            Some((_, was)) => *was = was.min(at),
            None => self.build_wakes.push((node, at)),
        }
    }

    /// Every pixel of every window is drawn in the next frame (new surfaces, lost content).
    pub fn invalidate_pixels(&mut self) {
        for w in self.windows.values_mut() {
            w.changes.full = true;
        }
    }

    /// Every pixel of `window` is drawn in the next frame.
    pub fn invalidate_window(&mut self, window: WindowId) {
        if let Some(w) = self.windows.get_mut(&window) {
            w.changes.full = true;
        }
    }

    /// The app-drawn surface `key` has new content inside `rect` (the item's coordinates; all of
    /// it when `None`): the draws that show it are damaged and renderers see the key in
    /// [`crate::FrameChanges::surfaces`]. No display list is re-recorded.
    pub fn invalidate_surface(&mut self, key: SurfaceKey, rect: Option<Rect>) {
        self.damage_media(Media::Surface(key), rect);
    }

    fn damage_media(&mut self, media: Media, part: Option<Rect>) {
        let Some(users) = self.media.get(&media).cloned() else {
            return;
        };
        for id in users {
            let Some(node) = self.tree.nodes.get(&id) else { continue };
            let Some(viewport) = self.windows.get(&node.window).map(|w| w.viewport) else { continue };
            let (parent_origin, parent_clip) = parent_space(&self.tree, id, viewport);
            let origin = parent_origin + node.frame.offset;
            let clip = node.frame.clip.map_or(parent_clip, |c| parent_clip.intersect(c.rect.translate(origin)));
            let rects: SmallVec<[Rect; 2]> = node
                .media
                .iter()
                .filter(|(m, _)| *m == media)
                .map(|(_, r)| match part {
                    Some(p) => p.translate(r.min.to_vec2()).intersect(*r),
                    None => *r,
                })
                .map(|r| r.translate(origin).intersect(clip))
                .filter(|r| !is_empty(*r))
                .collect();
            let window = node.window;
            if let Some(w) = self.windows.get_mut(&window) {
                w.damage.extend(rects);
                if let Media::Surface(key) = media
                    && !w.changes.surfaces.contains(&key)
                {
                    w.changes.surfaces.push(key);
                }
            }
        }
    }

    /// Hands `output` to the owner's next build: `show` returns it there.
    pub fn emit<O: 'static>(&mut self, node: NodeId, output: O) {
        let Some(n) = self.tree.nodes.get_mut(&node) else {
            return;
        };
        n.outbox = Some(Box::new(output));
        if let Some(owner) = n.owner {
            self.tree.mark_build(owner);
        }
    }

    /// Build, layout and paint over dirty nodes only, then damage, per window.
    /// An inline root that needs a build stays queued: hosts with inline roots call
    /// [`Runtime::frame_with`].
    pub fn frame(&mut self, text: &mut dyn TextEngine) -> FrameReport {
        self.run_frame(text, None)
    }

    /// [`Runtime::frame`] with the content of every inline root: `roots(window, cx)` runs inside
    /// the build pass for each window whose inline root is queued, as often as the frame's build
    /// rounds reach it.
    pub fn frame_with(&mut self, text: &mut dyn TextEngine, roots: &mut dyn FnMut(WindowId, &mut BuildCx<'_>)) -> FrameReport {
        self.run_frame(text, Some(roots))
    }

    fn run_frame(&mut self, text: &mut dyn TextEngine, mut roots: Option<&mut dyn FnMut(WindowId, &mut BuildCx<'_>)>) -> FrameReport {
        let mut report = FrameReport::default();
        self.counters = Counters::default();
        if std::mem::take(&mut self.away_check) && !self.windows.values().any(|w| *self.store.peek(w.env.window_focused)) {
            self.dismiss_away(None);
        }
        let mut held = Vec::new();
        for _ in 0..MAX_ROUNDS {
            self.apply_marks();
            let placing = self.windows.values().any(|w| w.relayout);
            if self.tree.build_q.is_empty() && self.tree.layout_q.is_empty() && !placing {
                break;
            }
            report.rounds += 1;
            let t = Instant::now();
            self.build_pass(&mut roots, &mut held, text);
            report.build_ms += ms(t);
            self.apply_marks();
            let t = Instant::now();
            layout_pass(self, text);
            place_layers(self, text);
            report.layout_ms += ms(t);
        }
        for run in self.store.take_unread(|node| self.tree.nodes.contains_key(&node)) {
            run();
        }
        let windows: Vec<WindowId> = self.windows.keys().copied().collect();
        for &w in &windows {
            self.repair_focus(w);
        }
        self.apply_marks();
        let t = Instant::now();
        paint_pass(self);
        // Content that moved or appeared under a still pointer changes the hover; the nodes it
        // reaches repaint in this frame.
        for &w in &windows {
            self.refresh_hover(w);
        }
        self.apply_marks();
        if !self.tree.paint_q.is_empty() {
            paint_pass(self);
        }
        report.paint_ms = ms(t);
        self.damage_touched_images();
        let t = Instant::now();
        self.settle_drag();
        for &w in &windows {
            self.refresh_cursor(w);
            self.collect_damage(w);
            report.access_nodes += self.update_access(w);
            self.sync_ime(w);
            if let Some(win) = self.windows.get_mut(&w) {
                win.input.focus_moved = false;
                win.relayout = false;
            }
        }
        report.access_ms = ms(t);
        report.built = self.counters.built;
        report.laid_out = self.counters.laid_out;
        report.painted = self.counters.painted;
        report.runs = self.counters.runs;
        report.scopes = self.counters.scopes;
        report.measures = self.counters.measures;
        report.access_builds = self.counters.access_builds;
        self.tree.build_q.extend(held);
        report
    }

    /// `window`'s frame for its renderer.
    ///
    /// # Panics
    /// When the window is not open.
    pub fn frame_tree(&self, window: WindowId) -> FrameTree<'_> {
        let w = &self.windows[&window];
        FrameTree {
            tree: &self.tree,
            images: &self.images,
            changes: &w.changes,
            window,
            root: w.root,
            viewport: w.viewport,
            scale: *self.store.peek(w.env.scale),
        }
    }

    /// Window-space rects (logical points) of `window` that changed in the last frame.
    pub fn damage(&self, window: WindowId) -> &[Rect] {
        self.windows.get(&window).map_or(&[], |w| w.damage.as_slice())
    }

    /// Whether `window` has anything to draw from the last frame.
    pub fn has_damage(&self, window: WindowId) -> bool {
        self.windows.get(&window).is_some_and(|w| !w.damage.is_empty() || w.changes.any())
    }

    /// Clears the frame's change lists once the renderer has drawn it.
    pub fn end_frame(&mut self) {
        for w in self.windows.values_mut() {
            w.changes.clear();
            w.damage.clear();
        }
        self.images.clear_changes();
    }

    pub fn take_access_update(&mut self, window: WindowId) -> Option<accesskit::TreeUpdate> {
        self.windows.get_mut(&window)?.access_update.take()
    }

    pub fn take_ime_requests(&mut self, window: WindowId) -> Vec<ImeRequest> {
        self.windows.get_mut(&window).map(|w| std::mem::take(&mut w.ime_requests)).unwrap_or_default()
    }

    fn apply_marks(&mut self) {
        for (node, phase) in std::mem::take(&mut self.store.pending) {
            self.tree.mark(node, phase);
        }
    }

    /// Builds the queued nodes, owners first. An inline root runs `roots` (or waits in `held`
    /// without it); other inline nodes are reached by their owners' builds.
    fn build_pass(&mut self, roots: &mut Option<&mut dyn FnMut(WindowId, &mut BuildCx<'_>)>, held: &mut Vec<NodeId>, text: &mut dyn TextEngine) {
        use crate::tree::Dirty;
        let queue = std::mem::take(&mut self.tree.build_q);
        let queue = self.tree.by_depth(queue);
        for id in queue {
            let Some(n) = self.tree.nodes.get(&id) else { continue };
            if n.inline {
                if n.owner.is_none() && (n.dirty.has(Dirty::BUILD) || n.dirty.has(Dirty::PATH)) {
                    match roots {
                        Some(roots) => build_inline_root(self, id, &mut **roots, &mut *text),
                        None => held.push(id),
                    }
                }
            } else if n.dirty.has(Dirty::BUILD) {
                build_node(self, id);
            }
        }
    }

    fn damage_touched_images(&mut self) {
        let touched = std::mem::take(&mut self.images.touched);
        for key in touched {
            self.damage_media(Media::Image(key), None);
        }
    }

    fn collect_damage(&mut self, window: WindowId) {
        let Some(w) = self.windows.get_mut(&window) else { return };
        let viewport = w.viewport;
        let tree = &mut self.tree;
        for &id in &w.changes.repainted {
            node_damage(tree, id, viewport, false, &mut w.damage);
        }
        for &id in w.changes.moved.iter().chain(w.restyled.iter()) {
            node_damage(tree, id, viewport, true, &mut w.damage);
        }
        w.restyled.clear();
        w.changes.removed.retain(|id| !tree.nodes.contains_key(id));
        if w.changes.full {
            w.damage.clear();
            w.damage.push(Rect::from_min_size(emath::Pos2::ZERO, viewport));
        }
        merge_damage(&mut w.damage);
    }

    /// Marks `id`'s access node for a rebuild in its window's next update.
    pub(crate) fn access_dirty(&mut self, id: NodeId) {
        if let Some(w) = self.tree.window_of(id).and_then(|w| self.windows.get_mut(&w))
            && w.access.enabled
        {
            w.access.dirty.insert(id);
        }
    }

    /// Records a node move for its window's renderer and damage.
    pub(crate) fn note_moved(&mut self, id: NodeId) {
        if let Some(w) = self.tree.window_of(id).and_then(|w| self.windows.get_mut(&w)) {
            w.changes.moved.push(id);
        }
    }

    /// Drops `id` and everything under it, with what it put on layers and the cells they own.
    /// Their last bounds become damage.
    pub(crate) fn remove_subtree(&mut self, id: NodeId) {
        let mut stack = vec![id];
        while let Some(id) = stack.pop() {
            let Some(node) = self.tree.nodes.remove(&id) else {
                continue;
            };
            stack.extend(node.children.iter().copied());
            stack.extend(node.layered.iter().copied());
            if let Some(parent) = node.parent
                && let Some(p) = self.tree.nodes.get_mut(&parent)
                && p.is_fixed()
            {
                p.children.retain(|&c| c != id);
            }
            self.forget_node(node.window, id);
            self.store.note_gone();
            self.build_wakes.retain(|&(n, _)| n != id);
            for (media, _) in &node.media {
                if let Some(users) = self.media.get_mut(media) {
                    users.retain(|&mut u| u != id);
                }
            }
            for (index, generation) in node.owned.into_iter().chain(node.slots.into_iter().map(|(_, at)| at)) {
                self.store.free_slot(index, generation);
            }
            if let Some(w) = self.windows.get_mut(&node.window) {
                w.damage.extend(node.abs);
                w.changes.removed.push(id);
                if w.layered.remove(&id).is_some() {
                    w.relayout = true;
                }
            }
        }
    }
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}
