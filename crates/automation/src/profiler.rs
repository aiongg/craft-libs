#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! Where each frame's time goes, gathered every frame, and the slow ones kept.
//!
//! [`FrameStages`] is one frame's record; each stage has one writer (the raw input hook starts the
//! frame, the command runner adds commands, the canvas adds damage, composite and upload, the UI
//! pass closes it through [`SlowFrames::finish`]). A frame over [`SLOW_FRAME_MS`] goes into
//! [`SlowFrames`], the last [`SLOW_FRAMES`] of them, which the control channel's `perf` output
//! lists as `slowFrames`; one `info` log line per second names them. [`crate::trace`] reads the
//! same record, so an app times a frame in one place.

use std::collections::VecDeque;

/// A frame slower than this is kept.
pub const SLOW_FRAME_MS: f64 = 50.0;
/// Slow frames kept.
pub const SLOW_FRAMES: usize = 64;

/// One frame's stage times in milliseconds. Field names are the `perf` output's JSON keys.
#[derive(Clone, Debug, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameStages {
    /// Frame start on the app clock, ms.
    pub t: f64,
    pub frame_ms: f64,
    pub input_ms: f64,
    pub command_ms: f64,
    /// The command id that ran this frame, if any.
    pub command: String,
    pub damage_ms: f64,
    pub composite_ms: f64,
    pub upload_ms: f64,
    pub gpu_ms: Option<f64>,
    pub present_ms: Option<f64>,
    /// What the canvas refreshed (`"gpu full"`, `"cpu tile"`, `""` none).
    pub refresh: &'static str,
    pub refresh_px: u64,
    /// The render path name (`"gpu"`, `"cpu"`).
    pub path: &'static str,
    pub ui_ms: f64,
    /// The live gesture's own render time (a brush stroke).
    pub gesture_ms: f64,
    /// Pointer moves delivered this frame.
    pub moves: usize,
    /// eframe's CPU time for the previous frame.
    pub cpu_prev_ms: Option<f64>,
}

impl FrameStages {
    /// Begin the frame at `now` with `moves` pointer moves.
    pub fn start(&mut self, now: f64, moves: usize) {
        // pc: crates/ui-egui/src/frame_stages.rs:73
        todo!()
    }

    /// The UI pass begins: input time is `now - t`.
    pub fn ui_start(&mut self, now: f64, cpu_prev: Option<f32>) {
        // pc: crates/ui-egui/src/frame_stages.rs:80
        todo!()
    }

    /// A command `id` took `ms`.
    pub fn command(&mut self, id: &str, ms: f64) {
        // pc: crates/ui-egui/src/frame_stages.rs:89
        todo!()
    }

    /// The canvas refreshed `px` pixels of `kind`.
    pub fn refresh(&mut self, kind: &'static str, px: u64, composite_ms: f64, upload_ms: f64) {
        // pc: crates/ui-egui/src/frame_stages.rs:95
        todo!()
    }

    /// The sum of the measured stages.
    pub fn stage_ms(&self) -> f64 {
        // pc: crates/ui-egui/src/frame_stages.rs:103
        todo!()
    }

    /// One log line: `frame 83ms: input 2 command 60 (filter.blur) ui 12 …`.
    pub fn line(&self) -> String {
        // pc: crates/ui-egui/src/frame_stages.rs:116
        todo!()
    }
}

/// The last [`SLOW_FRAMES`] frames over the threshold.
#[derive(Debug)]
pub struct SlowFrames {
    frames: VecDeque<FrameStages>,
    threshold_ms: f64,
    prev_ui_ms: Option<f64>,
    logged_at: Option<f64>,
    unlogged: usize,
}

impl Default for SlowFrames {
    fn default() -> Self {
        Self { frames: VecDeque::with_capacity(SLOW_FRAMES), threshold_ms: SLOW_FRAME_MS, prev_ui_ms: None, logged_at: None, unlogged: 0 }
    }
}

impl SlowFrames {
    /// The kept frames, oldest first.
    pub fn frames(&self) -> impl DoubleEndedIterator<Item = &FrameStages> {
        self.frames.iter()
    }

    /// Keep frames slower than `ms` instead of [`SLOW_FRAME_MS`].
    pub fn set_threshold_ms(&mut self, ms: f64) {
        self.threshold_ms = ms;
    }

    /// Close frame `s` at `now`; a slow frame is kept and logged at most once a second.
    pub fn finish(&mut self, s: FrameStages, now: f64, gpu: bool, gpu_ms: Option<f64>) -> FrameStages {
        // pc: crates/ui-egui/src/frame_stages.rs:168
        todo!()
    }
}

impl serde::Serialize for SlowFrames {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // pc: crates/ui-egui/src/frame_stages.rs:204
        todo!()
    }
}

/// Monotonic milliseconds since the first call in this process: `craft_platform::clock::monotonic_ms`,
/// so profiler rows, traces, screenshot deadlines and the app's own timers share one origin.
pub fn now_ms() -> f64 {
    // pc: crates/ui-egui/src/canvas/gpu_canvas/perf.rs:12
    todo!()
}

#[cfg(test)]
#[path = "profiler_tests.rs"]
mod profiler_tests;
