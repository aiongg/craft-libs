//! The component contract on craft-ui-rt: the bridge `docs/contract-rt.md` describes (option K).
//! Component hooks run in the build of the scope that shows the component; ui-rt keeps their
//! outputs per node (size, children's places, paint variants per pointer state, hit shape, access
//! node, input queue) and replays them in its own phases. A node whose type, component key and
//! place are unchanged and that is not marked runs no hook: a hover over it repaints it by picking
//! a kept variant, and a settled scope returns its kept value.
//!
//! This is the skeleton: the `Ui` cursor's vertical flow, scopes (window root, `MemoView`),
//! reconcile by `IdSource` with per-type ordinals, the component key, the retained node, input
//! queued with path marks and outputs in the same frame, with `IconButton` and `Stack` on it.
//!
//! The input half: `Response` with logged interaction reads, parts kept as hit shapes, the press
//! phases folded into per-part `PartResult`s, value gestures through the binding, toggle sweeps and
//! the wheel on a scrolling container's retained half, with `Slider`, `Toggle` and `ScrollArea`
//! stand-ins.

mod key;
mod node;
mod response;
mod run;
mod scope;
pub mod skeleton;
mod sweep;
mod ui;

pub use key::ComponentKey;
pub use node::{NodeInput, Pointer};
pub use response::{EditResponse, Outcome, Response};
pub use run::{Component, Container, Cx, MeasureCx, Paint, PartBuilder, PartsCx, ValueOf};
pub use scope::{MemoView, window_root};
pub use ui::Ui;
