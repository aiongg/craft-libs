//! Checks for [`craft_ui_rt`] driven through the headless platform: scenarios of injected input
//! with expected snapshots ([`scenario`], [`observe`]), pixel comparison against a reference
//! ([`pixels`]) and the per-scenario frame cost table ([`bench`], [`alloc`]).
//!
//! A scenario is data: steps of input for one or more windows plus, per step, a check of the
//! snapshot the step produced. Nothing in it names a component, so a panel or a PhotoCraft shape
//! runs the same way: build a [`rig::Rig`] on its root element, write its steps, and compare the
//! snapshots with a reference run, with expectations, or both.

#![allow(missing_docs, reason = "a test helper module; the types are named for what they hold")]

pub mod alloc;
pub mod bench;
pub mod observe;
pub mod pixels;
pub mod rig;
pub mod scenario;
pub mod stats;

pub use bench::{BenchRow, bench, bench_table, bench_with};
pub use observe::{AccessNode, Snapshot, StepDiff, WindowSnapshot, first_difference};
pub use pixels::{PixelResult, PixelStats, Rule, compare_pixels};
pub use rig::{Rig, test_tokens};
pub use scenario::{Scenario, Step, StepRun};
