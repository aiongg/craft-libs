//! One window's part of the runtime: its environment cells, fixed nodes (root and layers), frame
//! changes and damage, input, IME and access state. Nodes of every window live in one tree and
//! share one store, so a cell written in one window reruns its readers in every window.

use std::collections::BTreeMap;
use std::sync::Arc;

use craft_theme::Tokens;
use emath::{Pos2, Rect, Vec2};

use crate::access::AccessState;
use crate::drag::DragInfo;
use crate::frame::FrameChanges;
use crate::geom::Margin;
use crate::id::{ChildKey, IdMap, NodeId, WindowId};
use crate::input::InputState;
use crate::layer::{Layer, LayerSpec};
use crate::platform::ImeRequest;
use crate::state::State;

/// The cells every node can read. Each is an ordinary cell, so a scale change reruns only the
/// phases that read the scale, in the window that changed. `tokens`, `time` and `wakes` are the
/// same cells in every window.
#[derive(Clone, Copy, Debug)]
pub struct Env {
    pub window: WindowId,
    /// The window's logical size.
    pub viewport: State<Vec2>,
    pub scale: State<f32>,
    pub safe_area: State<Margin>,
    pub window_focused: State<bool>,
    pub tokens: State<Arc<Tokens>>,
    /// Seconds; set before a frame only while something reads it.
    pub time: State<f64>,
    /// Moves on every [`crate::PlatformEvent::Wake`]: a node that drains a background job's
    /// channel reads it in build and reruns when the job wakes the loop.
    pub wakes: State<u64>,
    /// The running drag ([`DragInfo`]), the same cell in every window: a dock shows its drop
    /// zones while a panel drag runs.
    pub drag: State<Option<DragInfo>>,
    /// The modifiers held at the window's last key, pointer or wheel event.
    pub modifiers: State<craft_keymap::chord::Mods>,
    /// The caps lock state as the platform last reported it ([`crate::WindowEvent::LockKeys`]).
    pub caps_lock: State<bool>,
    /// The num lock state as the platform last reported it.
    pub num_lock: State<bool>,
    /// The keys held down in the window, in the order they went down; emptied when the window loses focus.
    pub held_keys: State<Vec<crate::Key>>,
    /// Where the pointer is in the window and which buttons are down.
    pub pointer: State<PointerFacts>,
}

/// The pointer as a window last saw it: written by the runtime's input handling, read through
/// [`Env::pointer`]. A reader of it runs again at every move; hover and press facts of one node
/// ([`crate::InteractFact`]) are the cheap reads.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PointerFacts {
    /// The mouse or pen position, `None` while it is outside the window; the last touch contact's
    /// while it is down.
    pub pos: Option<Pos2>,
    buttons: u8,
}

impl PointerFacts {
    /// Whether `button` is down.
    pub fn held(&self, button: craft_behavior::PointerButton) -> bool {
        self.buttons & (1 << button as u8) != 0
    }

    pub(crate) fn set_held(&mut self, button: craft_behavior::PointerButton, down: bool) {
        let bit = 1 << button as u8;
        if down {
            self.buttons |= bit;
        } else {
            self.buttons &= !bit;
        }
    }
}

/// Where a window is and how big, as the platform last reported.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowGeometry {
    /// The inner size in logical points.
    pub size: Vec2,
    pub scale: f32,
    /// The surface's top left on the virtual desktop in physical pixels, once the platform told.
    pub position_px: Option<Pos2>,
    /// As the platform last reported ([`crate::WindowEvent::State`]).
    pub maximized: bool,
    pub fullscreen: bool,
    /// Surface top left to outer top left, physical pixels ([`crate::WindowEvent::FrameInset`]).
    pub frame_inset: Vec2,
}

pub(crate) struct Window {
    pub(crate) env: Env,
    pub(crate) root: NodeId,
    /// One node per [`Layer`], in `Layer::ALL` order.
    pub(crate) layers: [NodeId; 8],
    /// The app root's id on the content layer.
    pub(crate) app_root: NodeId,
    pub(crate) viewport: Vec2,
    pub(crate) changes: FrameChanges,
    /// Window-space rects (logical points) that changed in the last frame.
    pub(crate) damage: Vec<Rect>,
    /// Nodes whose clip or opacity changed in this frame's paint: everything under them may look
    /// different.
    pub(crate) restyled: Vec<NodeId>,
    pub(crate) input: InputState,
    pub(crate) access: AccessState,
    pub(crate) access_update: Option<accesskit::TreeUpdate>,
    pub(crate) ime_requests: Vec<ImeRequest>,
    /// How each layer child is placed, by node.
    pub(crate) layered: IdMap<LayerSpec>,
    /// A layout ran in this window in the current frame: layer children are placed again.
    pub(crate) relayout: bool,
    /// The surface's top left on the virtual desktop in physical pixels, once the platform told.
    pub(crate) position: Option<Pos2>,
    pub(crate) maximized: bool,
    pub(crate) fullscreen: bool,
    pub(crate) frame_inset: Vec2,
    /// The window it stacks above and moves with.
    pub(crate) owner: Option<WindowId>,
    /// Nodes that take drops anywhere in the window ([`crate::PaintCx::window_drop_target`]), in
    /// the order they registered.
    pub(crate) window_drops: Vec<NodeId>,
    /// Where focus goes when a node goes with focus inside it, by that node
    /// ([`crate::BuildCx::set_focus_return`]).
    pub(crate) focus_returns: IdMap<NodeId>,
    /// Nodes that take focus into their content once the build pass has made it, and whether a
    /// text input goes first ([`crate::BuildCx::focus_into`]).
    pub(crate) focus_entries: Vec<(NodeId, bool)>,
    /// Nodes that answer the window's close requests ([`crate::PaintCx::close_guard`]), in the order
    /// they registered.
    pub(crate) close_guards: Vec<NodeId>,
    /// The app's name for the window ([`crate::WindowSpec::key`]).
    pub(crate) key: Option<String>,
}

impl Window {
    pub(crate) fn new(id: WindowId, env: Env, viewport: Vec2) -> Window {
        let root = id.root();
        let layers = Layer::ALL.map(|l| root.child(ChildKey::Fixed(u32::from(l.index()))));
        let app_root = layers[0].child(ChildKey::Fixed(0));
        Window {
            env,
            root,
            layers,
            app_root,
            viewport,
            changes: FrameChanges { full: true, ..FrameChanges::default() },
            damage: Vec::new(),
            restyled: Vec::new(),
            input: InputState::default(),
            access: AccessState::default(),
            access_update: None,
            ime_requests: Vec::new(),
            layered: IdMap::default(),
            relayout: true,
            position: None,
            maximized: false,
            fullscreen: false,
            frame_inset: Vec2::ZERO,
            owner: None,
            window_drops: Vec::new(),
            focus_returns: IdMap::default(),
            focus_entries: Vec::new(),
            close_guards: Vec::new(),
            key: None,
        }
    }

    pub(crate) fn layer(&self, layer: Layer) -> NodeId {
        self.layers[layer.index() as usize]
    }
}

pub(crate) type Windows = BTreeMap<WindowId, Window>;
