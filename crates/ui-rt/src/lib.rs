//! craft's retained UI runtime: state cells with read tracking per phase, one persistent node tree
//! for every window with keyed reconcile, a layer stack per window, one-pass constraint layout,
//! display lists kept per node, the frame tree with damage, input routing, incremental AccessKit,
//! and the run loop. Windows, GPUs and text shaping sit behind [`Platform`], [`Renderer`] and
//! [`TextEngine`]; this crate depends on none of them (tests/boundary.rs).
//!
//! A frame runs only what changed: a state write marks the (node, phase) pairs that read the cell,
//! build reruns only build-dirty nodes, layout climbs only to the nearest relayout boundary, and
//! paint re-records only paint-dirty nodes. With nothing dirty there is no frame.
//!
//! Every event, surface, root, frame and access tree belongs to a [`WindowId`]. Within a window,
//! input reaches nodes through hit shapes recorded in paint (top layer first, topmost shape first;
//! the `Down`'s handler keeps the press), key focus in tree order with bubbling to ancestors, and
//! an IME session that follows the focused text node. A node shows children on its window's layers
//! ([`BuildCx::layer`]): popups, menus, tooltips, dialogs, drag ghosts and toasts, placed against
//! anchors after layout and dismissed by presses outside them. Drags carry a payload across
//! components and windows to drop targets, OS file drags included; pinches come from touchpads,
//! Ctrl+wheel steps and two touch contacts; cursors come from the nodes under the pointer. The AccessKit tree is incremental:
//! a frame sends only the access nodes of nodes that built, moved or resized, plus the focus.
//!
//! Public geometry is emath's; kurbo is used for paths only.

mod access;
mod build;
mod cursor;
mod display;
mod drag;
mod driver;
mod element;
mod event;
mod frame;
mod geom;
mod gesture;
mod hit;
mod id;
mod input;
mod layer;
mod layout;
mod paint;
mod platform;
pub mod record;
mod render;
mod runtime;
mod state;
mod text;
mod tree;
mod window;

pub use access::{AccessCx, AccessEntry};
pub use build::{BuildCx, InlineState};
pub use display::{
    Clip, Dash, DisplayList, Fill, FillRule, Filter, ImageChange, ImageKey, ImageOptions, ImageStore, Item, Media, Mesh, RgbaImage, Stroke,
    StrokeAlign, SurfaceKey, TextHighlight, Vertex,
};
pub use drag::{DragData, DragGhost, DragInfo, DragOutcome, DragPointer, DragStart, DropInput, DropPhase};
pub use driver::{Driver, StepReport};
pub use element::{Element, Memo, Show};
pub use event::{AccessInput, Event, EventCx, Gesture, Handled, KeyInput, PointerInput, WheelInput};
pub use frame::{Draw, FrameChanges, FrameNode, FrameTree, Part};
pub use geom::{Constraints, CornerRadii, Margin, RectU, SizeU, damage_pixels};
pub use gesture::{PINCH_CONTINUES, WheelOptions};
pub use hit::{Hit, HitShape};
pub use id::{NodeId, WindowId, hash_key};
pub use input::{Interact, InteractFact, LONG_PRESS_TIME, TOUCH_SLOP};
pub use layer::{DismissReason, Layer, LayerSpec, OutsidePress, Placement, Side};
pub use layout::{LayoutCx, virtual_band};
pub use paint::PaintCx;
pub use platform::{
    Clipboard, Cursor, FileDrag, ImePurpose, ImeRequest, NativeWindow, OsFile, OsFiles, Platform, PlatformEvent, PointerEvent, PointerPhase,
    SurfaceHandle, SurfaceHost, SystemTheme, Waker, WindowCommand, WindowEvent, WindowHandles, WindowSpec,
};
pub use render::{ReadBack, ReadBackId, RenderStats, Renderer};
pub use runtime::{FrameReport, Runtime};
pub use state::{Access, DeriveCx, Derived, Phase, Reads, State, Store, Writes};
pub use text::{FontSpec, Overflow, TextEngine, TextHandle, TextLayout, TextSpec, overflow_text};
pub use window::Env;

pub use craft_behavior::{
    CopyKeys, CursorIcon, DropOperation, PointerButton, PointerKind, PointerSample, PressCancel, Sense, WheelPhase, WheelSource, WheelUnit,
};
pub use craft_host::target::Os;
pub use craft_keymap::chord::{Key, Mods};
pub use craft_theme::{Rgba, Tokens};
pub use emath::{Align, Pos2, Rect, Vec2, pos2, vec2};
pub use kurbo::BezPath;

#[cfg(test)]
extern crate self as craft_ui_rt;

#[cfg(test)]
#[path = "../tests/main.rs"]
mod integration;
