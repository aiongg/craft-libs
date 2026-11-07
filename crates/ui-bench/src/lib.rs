//! Performance scenarios of the UI library (spec 22.7) and the harness that measures them.
//!
//! The scenario list, the measurement and the report are shared. A library supplies a
//! [`Library`]: one binary per library (`legacy`, and the foundation's) runs [`run_all`] over it
//! and writes `baseline/<library>.json`.

mod alloc;
mod harness;
mod report;
mod scenario;

pub use alloc::{CountingAlloc, allocations};
pub use harness::{Library, MEASURED_FRAMES, Scene, SceneBuild, WARMUP_FRAMES, run_all, screen_rect};
pub use report::{Measurement, Report, ScenarioResult, gate_failures, print_table, read_json, write_json};
pub use scenario::{SCENARIOS, Scenario};
