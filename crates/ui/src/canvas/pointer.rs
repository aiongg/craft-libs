//! [`PenInput`] and [`CanvasButtons`]: pointer helpers for a canvas. Pen pressure, tilt, barrel
//! rotation and the eraser end, read from egui's events plus a platform feed; and the per-pass
//! button summary tools and overlays share.
//!
//! Sources: egui's `Event::Touch { force }` (Windows pen and touch, web touch) and, where winit
//! drops tablet data (macOS, X11), a [`PenFeed`] the app's platform layer writes before each pass
//! (`craft-platform::pen` is the egui-free feed; PhotoCraft's `photocraft-tablet` crate fills it).
//! A mouse always reads as pressure 1 with no tilt. Preferences › Tools › Use Tablet Pressure off
//! makes a pen read like a mouse. Flipping the pen to its eraser end selects the Eraser tool and
//! flipping back restores the previous tool: the app does the switch on
//! [`PenInput::take_eraser_flip`].
//!
//! Automation simulates a pen through [`ToolInput`](super::ToolInput)'s pressure, tilt and rotation.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use egui::{Event, PointerButton, Response};

/// One stylus reading. Tilt in degrees (-90..90, W3C Pointer Events convention), rotation in
/// degrees 0..360, pressure 0..=1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PenSample {
    pub pressure: f32,
    pub tilt: [f32; 2],
    pub rotation: f32,
    /// The eraser end is toward the surface.
    pub eraser: bool,
    /// Wall time of the reading in milliseconds, for stroke timing.
    pub time_ms: f64,
}

impl PenSample {
    /// Pressure 1, no tilt, not the eraser: a mouse.
    pub const MOUSE: PenSample = PenSample { pressure: 1.0, tilt: [0.0, 0.0], rotation: 0.0, eraser: false, time_ms: 0.0 };

    /// Non-finite or out-of-range fields clamped into range.
    // pc: crates/ui-egui/src/input/stylus.rs:55 `sanitized`.
    pub fn sanitized(self) -> Self {
        todo!()
    }
}

/// Samples written by a platform reader on another thread and drained by [`PenInput`] each pass.
#[derive(Clone, Debug, Default)]
pub struct PenFeed(Arc<Mutex<VecDeque<PenSample>>>);

impl PenFeed {
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a reading (from the platform event monitor or the web runner).
    // pc: crates/ui-egui/src/input/stylus.rs:87 `set`.
    pub fn push(&self, sample: PenSample) {
        todo!()
    }

    /// Every reading since the last drain, oldest first.
    // pc: crates/ui-egui/src/input/stylus.rs:104 `drain`.
    pub fn drain(&self) -> Vec<PenSample> {
        todo!()
    }
}

/// The pen reader an app keeps for its window: one per window, updated once per pass before the
/// canvas runs.
#[derive(Clone, Debug, Default)]
pub struct PenInput {
    feed: Option<PenFeed>,
    /// Samples read this pass, oldest first.
    samples: Vec<PenSample>,
    /// The latest reading, kept between passes while a pen is in use.
    last: Option<PenSample>,
    /// A pen was seen (any force or feed sample): pressure 0 then means "no force", not a mouse.
    pen_seen: bool,
    /// Preferences › Tools › Use Tablet Pressure.
    use_pressure: bool,
    /// Pressure curve control points (x in, y out), identity when empty.
    curve: Vec<[f32; 2]>,
    /// The eraser end flipped this pass: `Some(true)` toward the surface, `Some(false)` away.
    flip: Option<bool>,
}

impl PenInput {
    pub fn new() -> Self {
        Self { use_pressure: true, ..Self::default() }
    }

    /// Read a platform feed in addition to egui's touch force.
    pub fn with_feed(mut self, feed: PenFeed) -> Self {
        self.feed = Some(feed);
        self
    }

    /// Preferences › Tools › Use Tablet Pressure.
    pub fn use_pressure(&mut self, on: bool) {
        self.use_pressure = on;
    }

    /// Brush Settings' pressure curve (points of (in, out) in 0..=1).
    // pc: crates/ui-egui/src/input/stylus.rs:276 `set_pressure_curve`.
    pub fn set_pressure_curve(&mut self, points: &[[f32; 2]]) {
        todo!()
    }

    /// Read this pass's events (`ui.input(|i| i.events.clone())`) and the feed. Called once per
    /// pass before the canvas.
    // pc: crates/ui-egui/src/input/stylus.rs:191 `update`.
    pub fn update(&mut self, events: &[Event]) {
        todo!()
    }

    /// The reading to use for a pointer sample now: the latest pen reading through the curve, or
    /// [`PenSample::MOUSE`] when no pen is in use or the preference is off.
    // pc: crates/ui-egui/src/input/stylus.rs:263 `sample`, :353 `pressure`.
    pub fn sample(&self) -> PenSample {
        todo!()
    }

    /// Every reading of this pass, oldest first, for a stroke that wants all of them.
    pub fn samples(&self) -> &[PenSample] {
        &self.samples
    }

    /// The eraser end flipped this pass: `Some(true)` toward the surface (select the Eraser),
    /// `Some(false)` away (restore the tool). Read once.
    // pc: crates/ui-egui/src/input/stylus.rs:321 `take_end_flip`, :330 `sync_eraser_tool`.
    pub fn take_eraser_flip(&mut self) -> Option<bool> {
        todo!()
    }

    /// Whether a pen is in use (a force or feed sample was seen since the last mouse move).
    pub fn pen_in_use(&self) -> bool {
        self.pen_seen
    }
}

/// The canvas's buttons this pass, from its `Response`: what the tool host, the overlays and the
/// app's pasteboard click read instead of each inspecting the pointer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CanvasButtons {
    /// The primary button went down on the canvas this pass.
    pub started: bool,
    /// The primary button is down and the pointer moved past the drag threshold.
    pub dragged: bool,
    /// The primary button came up this pass after a drag.
    pub stopped: bool,
    /// The primary button came up this pass without a drag.
    pub clicked: bool,
    /// The secondary button was pressed (a context menu, the brush picker).
    pub secondary: bool,
    /// The middle button is down (a pan on every tool).
    pub middle_down: bool,
    /// Double click.
    pub double: bool,
}

impl CanvasButtons {
    /// Read the canvas response.
    // pc: crates/ui-egui/src/canvas/paint_mouse.rs:90 `Buttons`.
    pub fn read(canvas: &Response) -> Self {
        todo!()
    }

    /// Whether `button` is the one a tool follows: the primary, or the secondary when the
    /// preference makes a right drag erase.
    pub fn follows(button: PointerButton, secondary_paints: bool) -> bool {
        todo!()
    }
}
