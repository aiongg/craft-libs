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

pub mod access;
mod container;
mod key;
mod lazy;
mod node;
mod response;
mod run;
mod scope;
mod scroll;
pub mod skeleton;
mod sweep;
mod ui;

pub use container::{Container, show_container};
pub use key::ComponentKey;
pub use lazy::Lazy;
pub use node::{NodeInput, Pointer};
pub use response::{EditResponse, Outcome, Response};
pub use run::{Component, Cx, MeasureCx, Paint, PartBuilder, PartsCx, ValueOf, show_leaf};
pub use scope::{MemoView, window_root};
pub use ui::Ui;
