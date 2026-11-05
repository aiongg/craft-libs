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

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, PoisonError};

use egui::{Event, PointerButton, Response};

/// One stylus reading. Tilt in degrees (-90..90, W3C Pointer Events convention), rotation in
/// degrees 0..360, pressure 0..=1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PenSample {
    #[allow(missing_docs)]
    pub pressure: f32,
    #[allow(missing_docs)]
    pub tilt: [f32; 2],
    #[allow(missing_docs)]
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
        let finite = |v: f32, lo: f32, hi: f32, default: f32| if v.is_finite() { v.clamp(lo, hi) } else { default };
        Self {
            pressure: finite(self.pressure, 0.0, 1.0, 1.0),
            tilt: self.tilt.map(|t| finite(t, -90.0, 90.0, 0.0)),
            // `rem_euclid` rounds a tiny negative angle up to exactly 360.
            rotation: if self.rotation.is_finite() { self.rotation.rem_euclid(360.0) % 360.0 } else { 0.0 },
            eraser: self.eraser,
            time_ms: if self.time_ms.is_finite() { self.time_ms } else { 0.0 },
        }
    }
}

/// Most readings a [`PenFeed`] keeps between two passes (a stalled UI drops the oldest).
const FEED_QUEUE: usize = 1024;

/// Samples written by a platform reader on another thread and drained by [`PenInput`] each pass.
#[derive(Clone, Debug, Default)]
pub struct PenFeed(Arc<Mutex<VecDeque<PenSample>>>);

impl PenFeed {
    /// An empty feed.
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a reading (from the platform event monitor or the web runner).
    // pc: crates/ui-egui/src/input/stylus.rs:87 `set`.
    pub fn push(&self, sample: PenSample) {
        let mut queue = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if queue.len() >= FEED_QUEUE {
            queue.pop_front();
        }
        queue.push_back(sample.sanitized());
    }

    /// Every reading since the last drain, oldest first.
    // pc: crates/ui-egui/src/input/stylus.rs:104 `drain`.
    pub fn drain(&self) -> Vec<PenSample> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).drain(..).collect()
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
    /// The pen end last seen, so a flip is reported once.
    eraser_end: bool,
}

impl PenInput {
    /// A reader with pressure enabled.
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
        self.curve = points.iter().filter(|p| p[0].is_finite() && p[1].is_finite()).map(|p| [p[0].clamp(0.0, 1.0), p[1].clamp(0.0, 1.0)]).collect();
        self.curve.sort_by(|a, b| a[0].total_cmp(&b[0]));
        // Outputs never fall, so the curve stays monotone.
        let mut floor = 0.0f32;
        for point in &mut self.curve {
            floor = floor.max(point[1]);
            point[1] = floor;
        }
        self.curve.dedup_by(|b, a| (a[0] - b[0]).abs() < 1e-6);
        if self.curve.len() < 2 {
            self.curve.clear();
        }
    }

    /// Read this pass's events (`ui.input(|i| i.events.clone())`) and the feed. Called once per
    /// pass before the canvas.
    // pc: crates/ui-egui/src/input/stylus.rs:191 `update`.
    pub fn update(&mut self, events: &[Event]) {
        self.flip = None;
        let mut samples = self.feed.as_ref().map(PenFeed::drain).unwrap_or_default();
        let from_feed = !samples.is_empty();
        let mut touched = false;
        let mut moved = false;
        for event in events {
            match event {
                Event::Touch { phase: egui::TouchPhase::Start | egui::TouchPhase::Move, force, .. } => {
                    touched = true;
                    let force = force.filter(|f| f.is_finite()).map(|f| f.clamp(0.0, 1.0));
                    // Only a pen moves without touching and only a pen (or a force-sensing screen)
                    // reports a force.
                    if force.is_some() {
                        self.pen_seen = true;
                    }
                    // A pen pressed lighter than its first level reports no force at all (winit
                    // maps 0 to `None`): that is pressure 0, never a mouse's 1.
                    if let Some(pressure) = force.or(self.pen_seen.then_some(0.0)) {
                        samples.push(PenSample { pressure, ..PenSample::MOUSE });
                    }
                }
                Event::Touch { .. } => touched = true,
                Event::PointerMoved(_) => moved = true,
                _ => {}
            }
        }
        // A pen reports pressure 0 while it hovers and as it lifts: a pass that also has contact
        // samples keeps only those, so a stroke's last moves keep the pressure the pen left with.
        if samples.iter().any(|s| s.pressure > 0.0) {
            samples.retain(|s| s.pressure > 0.0);
        }
        if from_feed {
            self.pen_seen = true;
        }
        if samples.is_empty() && moved && !touched {
            // The mouse is moving: the pen left.
            self.pen_seen = false;
            self.last = None;
        }
        if let Some(latest) = samples.last().copied() {
            self.pen_seen = true;
            if latest.eraser != self.eraser_end {
                self.eraser_end = latest.eraser;
                self.flip = Some(latest.eraser);
            }
            self.last = Some(latest);
        }
        self.samples = samples;
    }

    /// The reading to use for a pointer sample now: the latest pen reading through the curve, or
    /// [`PenSample::MOUSE`] when no pen is in use or the preference is off.
    // pc: crates/ui-egui/src/input/stylus.rs:263 `sample`, :353 `pressure`.
    pub fn sample(&self) -> PenSample {
        match self.last {
            Some(last) if self.use_pressure && self.pen_seen => PenSample { pressure: self.through_curve(last.pressure), ..last },
            _ => PenSample::MOUSE,
        }
    }

    /// Every reading of this pass, oldest first, for a stroke that wants all of them.
    pub fn samples(&self) -> &[PenSample] {
        &self.samples
    }

    /// The eraser end flipped this pass: `Some(true)` toward the surface (select the Eraser),
    /// `Some(false)` away (restore the tool). Read once.
    // pc: crates/ui-egui/src/input/stylus.rs:321 `take_end_flip`, :330 `sync_eraser_tool`.
    pub fn take_eraser_flip(&mut self) -> Option<bool> {
        self.flip.take()
    }

    /// `pressure` through the monotone cubic (Fritsch-Carlson) of the curve points; identity
    /// without a curve.
    fn through_curve(&self, pressure: f32) -> f32 {
        let (xs, ys): (Vec<f32>, Vec<f32>) = self.curve.iter().map(|p| (p[0], p[1])).unzip();
        let n = xs.len();
        if n < 2 {
            return pressure;
        }
        let x = pressure.clamp(0.0, 1.0);
        let secants: Vec<f32> = (0..n - 1).map(|k| (ys[k + 1] - ys[k]) / (xs[k + 1] - xs[k])).collect();
        let mut tangents = vec![0.0f32; n];
        tangents[0] = secants[0];
        tangents[n - 1] = secants[n - 2];
        for k in 1..n - 1 {
            tangents[k] = if secants[k - 1] * secants[k] <= 0.0 { 0.0 } else { (secants[k - 1] + secants[k]) / 2.0 };
        }
        // Limit the tangents so the cubic never overshoots.
        for (k, &secant) in secants.iter().enumerate() {
            if secant == 0.0 {
                tangents[k] = 0.0;
                tangents[k + 1] = 0.0;
                continue;
            }
            let (a, b) = (tangents[k] / secant, tangents[k + 1] / secant);
            let s = a * a + b * b;
            if s > 9.0 {
                let t = 3.0 / s.sqrt();
                tangents[k] = t * a * secant;
                tangents[k + 1] = t * b * secant;
            }
        }
        if x <= xs[0] {
            return ys[0];
        }
        if x >= xs[n - 1] {
            return ys[n - 1];
        }
        let k = xs.partition_point(|&v| v <= x).saturating_sub(1).min(n - 2);
        let h = xs[k + 1] - xs[k];
        let t = (x - xs[k]) / h;
        let (t2, t3) = (t * t, t * t * t);
        let y = (2.0 * t3 - 3.0 * t2 + 1.0) * ys[k]
            + (t3 - 2.0 * t2 + t) * h * tangents[k]
            + (-2.0 * t3 + 3.0 * t2) * ys[k + 1]
            + (t3 - t2) * h * tangents[k + 1];
        y.clamp(ys[k], ys[k + 1])
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
        let middle_down = canvas.dragged_by(PointerButton::Middle) || (canvas.contains_pointer() && canvas.ctx.input(|i| i.pointer.middle_down()));
        Self {
            started: canvas.drag_started_by(PointerButton::Primary),
            dragged: canvas.dragged_by(PointerButton::Primary),
            stopped: canvas.drag_stopped_by(PointerButton::Primary),
            clicked: canvas.clicked_by(PointerButton::Primary),
            secondary: canvas.contains_pointer() && canvas.ctx.input(|i| i.pointer.button_pressed(PointerButton::Secondary)),
            middle_down,
            double: canvas.double_clicked(),
        }
    }

    /// Whether `button` is the one a tool follows: the primary, or the secondary when the
    /// preference makes a right drag erase.
    pub fn follows(button: PointerButton, secondary_paints: bool) -> bool {
        button == PointerButton::Primary || (secondary_paints && button == PointerButton::Secondary)
    }
}
