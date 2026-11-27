//! The component contract on craft-ui-rt: the bridge `docs/contract-rt.md` describes (option K).
//! Every module is crate-private: the public names live at their real paths (section 15.10).
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
//! moves walk the kept nodes; a key goes to the focused node's queue as an intent when its kept
//! `KeyUse` takes it, else up its path to the scopes (a dialog's Submit, the cancel rule) and the
//! window root, which runs the command the key is bound to (`focus`, `keys`, `intents`,
//! `commands`).
//!
//! Text: typed text and the platform's input method reach the focused text input through one IME
//! engine (`ime`); `text` has the editor state and the stand-in `TextField`.

pub(crate) mod access;
pub(crate) mod commands;
pub(crate) mod container;
pub(crate) mod cursor;
pub(crate) mod dialog;
pub(crate) mod drag;
pub(crate) mod env;
pub(crate) mod focus;
#[cfg(feature = "testing")]
pub(crate) mod harness;
pub(crate) mod ids;
pub(crate) mod ime;
pub(crate) mod intents;
pub(crate) mod items;
pub(crate) mod key;
pub(crate) mod keys;
pub(crate) mod memory;
pub(crate) mod motion;
pub(crate) mod node;
pub(crate) mod overlay;
pub(crate) mod pending;
pub(crate) mod response;
pub(crate) mod run;
pub(crate) mod scope;
pub(crate) mod scroll;
pub(crate) mod sweep;
pub(crate) mod text;
pub(crate) mod textures;
pub(crate) mod tip;
pub(crate) mod toast;
pub(crate) mod ui;
pub(crate) mod windows;

pub(crate) use crate::primitives::textures::TextureKey;
pub(crate) use commands::CommandBoard;
pub(crate) use drag::{DragSource, DropRule};
pub(crate) use env::{EnvCells, EnvScope, LayerFrame, ThemeBase};
pub(crate) use key::ComponentKey;
pub(crate) use motion::{Interpolate, MotionSpec, Tween, loop_phase, spec};
pub(crate) use node::KeptPart;
pub(crate) use node::{NodeInput, Pointer};
pub(crate) use overlay::{OverlayUi, PopupHandle};
pub(crate) use pending::{Delivery, JobBoard, Pending};
pub(crate) use run::{kept_part_id, show_leaf};
pub(crate) use scope::{WindowEnv, window_root, window_root_with};
pub(crate) use textures::{TextureCache, TextureLook};
pub(crate) use tip::DELAY as TOOLTIP_DELAY;
pub(crate) use toast::{ToastHost, ToastQueue};
pub(crate) use ui::{Ui, UiBuilder};
