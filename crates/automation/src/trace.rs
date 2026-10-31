#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! Frame timing trace: every frame of a gesture (a brush stroke, a drag) as a row, so a tester on
//! any platform reports a per-stage breakdown instead of one input-to-pixel number.
//!
//! Set the app's trace variable before launching ([`FrameTrace::from_env`]; PhotoCraft keeps
//! `PHOTOCRAFT_STROKE_TRACE`): `1` prints a [`Summary`] to stderr when each gesture ends; a file
//! path also appends every frame of every gesture to that CSV. The last summary is in the control
//! channel's `perf` output under `frameTrace` (PhotoCraft's `strokeTrace`, renamed with
//! `gesture_ms`; spec Q-S3T-5). The trace keeps no clock: each row is the
//! frame's [`FrameStages`], so the slow-frame ring and the trace read one record.

use crate::profiler::FrameStages;

/// One frame of a gesture.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Row {
    pub t: f64,
    pub interval: f64,
    pub moves: usize,
    pub gesture: f64,
    pub canvas: f64,
    pub ui: f64,
    pub cpu_prev: Option<f64>,
}

/// `[p50, p95]` per stage over one gesture's frames.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub frames: usize,
    pub backend: String,
    pub interval: [f64; 2],
    pub moves: [f64; 2],
    pub gesture: [f64; 2],
    pub canvas: [f64; 2],
    pub ui: [f64; 2],
    pub cpu: [f64; 2],
    pub wait: [f64; 2],
}

impl Summary {
    /// One stderr line.
    pub fn line(&self) -> String {
        // pc: crates/ui-egui/src/tools/stroke_trace.rs:69
        todo!()
    }
}

/// Where the trace goes.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Output {
    #[default]
    Off,
    Stderr,
    Csv(std::path::PathBuf),
}

/// The trace state: the current gesture's rows and the last summary.
#[derive(Debug, Default)]
pub struct FrameTrace {
    output: Output,
    last_start: Option<f64>,
    rows: Vec<Row>,
    gestures: usize,
    /// The last finished gesture's summary.
    pub last: Option<Summary>,
}

impl FrameTrace {
    /// Reads the environment variable `var`.
    pub fn from_env(var: &str) -> Self {
        // pc: crates/ui-egui/src/tools/stroke_trace.rs:124
        todo!()
    }

    /// `v` trimmed: `""`, `"0"`, `"off"` or `"false"` is off; `"1"`, `"stderr"` or `"true"` prints
    /// summaries; anything else is a CSV path. Always off on wasm (no clock there).
    pub fn from_value(v: &str) -> Self {
        // pc: crates/ui-egui/src/tools/stroke_trace.rs:135
        todo!()
    }

    pub fn is_on(&self) -> bool {
        !matches!(self.output, Output::Off)
    }

    /// Turn summaries on or off (the control channel's `ui.set`); never sets a path.
    pub fn set_on(&mut self, on: bool) {
        // pc: crates/ui-egui/src/tools/stroke_trace.rs:147 (ui.set strokeTrace)
        todo!()
    }

    /// Record `stages` when `in_gesture`; the first frame after a gesture ends summarises it,
    /// with `backend` named in the summary.
    pub fn frame(&mut self, stages: &FrameStages, in_gesture: bool, backend: impl FnOnce() -> String) {
        // pc: crates/ui-egui/src/tools/stroke_trace.rs:160 (frame)
        todo!()
    }
}

/// `[p50, p95]` of `v`, nearest rank, rounded to 0.01; zeros when empty.
pub fn percentiles(v: Vec<f64>) -> [f64; 2] {
    // pc: crates/ui-egui/src/tools/stroke_trace.rs:111
    todo!()
}

#[cfg(test)]
#[path = "trace_tests.rs"]
mod trace_tests;
