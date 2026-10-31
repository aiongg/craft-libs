#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! Frame pacing and background work by window state.
//!
//! - [`FrameCap`] (desktop): at most N frames per second when presenting without vsync. Nothing
//!   else paces frames then on X11, and a 1000 Hz mouse makes an app draw about 900 frames a
//!   second.
//! - [`WindowState`]: whether the window is drawn. A minimized or covered window runs only
//!   `App::logic`, with no vsync to pace it and with `ctx.input` frozen at the last drawn pass.
//! - [`WorkBudget`] and [`WorkSlice`]: how long one call may spend on background work (an export,
//!   a bounce, an index build) in each window state, or that it pauses while hidden.
//! - [`repaint_in`] and [`paint_while_hidden`]: ask for the next frame at a time, and for the UI
//!   pass to run while hidden (a screenshot or an automation client needs the painted pass).
//!
//! ```ignore
//! // App::logic
//! let state = WindowState::current(ctx);
//! if let Some(slice) = EXPORT_BUDGET.slice(state, &self.clock) {
//!     while slice.more(&self.clock) && export.step() {}
//!     ctx.request_repaint();
//! }
//! ```

use std::time::Duration;

use crate::clock::Clock;

/// Whether the window is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowState {
    /// On screen, or not known to be hidden (a host that reports neither minimized nor occluded).
    Shown,
    /// Minimized or fully covered: only `App::logic` runs, and `ctx.input` repeats the last
    /// drawn pass's pointer and keys, so a gesture in progress ends here.
    Hidden,
}

impl WindowState {
    /// The state `info` reports: hidden when minimized or occluded (`ViewportInfo::visible`).
    pub fn of(info: &egui::ViewportInfo) -> Self {
        // soundcraft: crates/ui-egui/src/app.rs:159
        todo!()
    }

    /// The state of the viewport `ctx` is running.
    pub fn current(ctx: &egui::Context) -> Self {
        // lightcraft: crates/ui-egui/src/app.rs:339
        todo!()
    }
}

/// A frame-rate cap for presenting without vsync: waiting out the rest of a short minimum
/// interval adds at most that interval of input age, far less than a vsync queue.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug)]
#[expect(dead_code, reason = "skeleton: removed when the bodies land")]
pub struct FrameCap {
    min: Duration,
    next: Option<std::time::Instant>,
}

#[cfg(not(target_arch = "wasm32"))]
impl FrameCap {
    /// At most `max_fps` frames per second (0 counts as 1). CadCraft caps at 240 when it presents
    /// without vsync (`apps/cadcraft/src/main.rs:342`).
    pub fn new(max_fps: u32) -> Self {
        // cadcraft: crates/ui-egui/src/gpu.rs:71
        todo!()
    }

    /// How long a frame starting at `now` waits; records when the following one may start.
    pub fn wait_at(&mut self, now: std::time::Instant) -> Duration {
        // cadcraft: crates/ui-egui/src/gpu.rs:76
        todo!()
    }

    /// Block until this frame may start. Call once per frame, at its start.
    pub fn wait(&mut self) {
        // cadcraft: crates/ui-egui/src/gpu.rs:83
        todo!()
    }
}

/// How long one call may spend on a background job in each window state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorkBudget {
    /// Per call while the window is drawn; the frame waits for it, so keep it under a frame.
    pub shown: Duration,
    /// Per call while hidden; `None` pauses the job until the window is shown again (a
    /// slideshow, a preview that only the window shows).
    pub hidden: Option<Duration>,
}

impl WorkBudget {
    /// 30 ms a call, shown or hidden: SoundCraft's bounce slice.
    // soundcraft: crates/ui-egui/src/bounce.rs:51
    pub const DEFAULT: Self = Self { shown: Duration::from_millis(30), hidden: Some(Duration::from_millis(30)) };

    /// The slice this call may spend in `state`, from `clock`'s now; `None` while the job pauses.
    pub fn slice(&self, state: WindowState, clock: &Clock) -> Option<WorkSlice> {
        // new: soundcraft's per-call slice with lightcraft's pause while hidden (app.rs:338)
        todo!()
    }
}

impl Default for WorkBudget {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// One call's share of background work: do units while [`Self::more`] says so.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorkSlice {
    /// Clock seconds the slice ends at.
    until: f64,
}

impl WorkSlice {
    /// Whether there is time for another unit of work. A manual clock that doesn't move gives
    /// one unit per slice, so a test steps a job unit by unit.
    pub fn more(&self, clock: &Clock) -> bool {
        // soundcraft: crates/ui-egui/src/bounce.rs:50
        todo!()
    }
}

/// Draw another frame `secs` from now; a negative `secs` means now, a non-finite one asks for
/// nothing. An idle window draws no frames, so a timer that must fire asks for its own.
#[track_caller]
pub fn repaint_in(ctx: &egui::Context, secs: f64) {
    // figcraft: crates/ui-egui/src/clock.rs:30
    todo!()
}

/// Run and paint the next pass even when the window is hidden, for a screenshot or an automation
/// client waiting on the widget tree. Holds for one frame; call it each frame it is needed.
pub fn paint_while_hidden(ctx: &egui::Context) {
    // new: the suite's egui fork's ViewportCommand::RequestPaintWhileHidden (viewport.rs:1239)
    todo!()
}

#[cfg(test)]
#[path = "pacing_tests.rs"]
mod pacing_tests;
