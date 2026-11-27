//! The component contract on craft-ui-rt: the bridge `docs/contract-rt.md` describes (option K).
//! Component hooks run in the build of the scope that shows the component; ui-rt keeps their
//! outputs per node (size, children's places, paint variants per pointer state, hit shape, access
//! node, input queue) and replays them in its own phases. A node whose type, component key and
//! place are unchanged and that is not marked runs no hook: a hover over it repaints it by picking
//! a kept variant, and a settled scope returns its kept value.
//!
//! Layout happens in the build: the `Ui` cursor places children by the body's `BodyLayout`, a
//! container's regions are nodes of their own, a measuring run asks kept children for their size
//! from the node size cache, a scroll body moves kept content by its offset in layout, and `Lazy`
//! children out of view keep only their extent.
//!
//! Input: `Response` with logged interaction reads, parts kept as hit shapes, the press phases
//! folded into per-part `PartResult`s, value gestures through the binding and toggle sweeps, with
//! `Slider`, `Toggle` and `ScrollArea` stand-ins.
//!
//! Focus and keys: stops, focus scopes and roving groups kept with the node; Tab, F6 and roving
//! moves walk the kept nodes; a key goes to the focused node's queue when its kept `KeyUse` takes
//! it, else up its path to the scopes and the window root (`focus`, `keys`).

pub mod access;
mod container;
mod cursor;
mod dialog;
mod drag;
mod env;
mod focus;
mod key;
mod keys;
mod lazy;
mod memory;
mod motion;
mod node;
mod overlay;
pub mod pending;
mod response;
mod run;
mod scope;
mod scroll;
pub mod skeleton;
mod sweep;
pub mod textures;
mod tip;
mod toast;
mod ui;

pub use container::{Container, show_container};
pub use dialog::show_dialog;
pub use drag::{DragSource, DropRule};
pub use env::{EnvCells, EnvScope, LayerFrame, ThemeBase};
pub use key::ComponentKey;
pub use keys::KeyPress;
pub use lazy::Lazy;
pub use motion::{Interpolate, MotionSpec, Tween, loop_phase, spec};
pub use node::{NodeInput, Pointer};
pub use overlay::{OverlayUi, PopupHandle};
pub use pending::{Delivery, JobBoard, Pending};
pub use response::{EditResponse, Outcome, Response};
pub use run::{Component, Cx, MeasureCx, Paint, PartBuilder, PartsCx, ValueOf, show_leaf};
pub use scope::{MemoView, WindowEnv, window_root, window_root_with};
pub use textures::{TextureCache, TextureKey, TextureLook};
pub use tip::DELAY as TOOLTIP_DELAY;
pub use toast::{ToastHost, ToastQueue};
pub use ui::Ui;
