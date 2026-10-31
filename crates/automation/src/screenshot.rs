#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! `ui.screenshot`: bounded, cancellable capture requests over [`craft_ui::capture::CaptureSlot`].
//!
//! The slot correlates one capture with one image. This queue owns what the slot leaves to the
//! app: request identity (token, output path, reply), deadlines and settling, the admission budget
//! ([`MAX_PENDING`] requests, [`MAX_IMAGE_BYTES`] per image, [`MAX_TOTAL_BYTES`] in all), the
//! document identity fixed at admission, the geometry epoch recorded at issue so an image is saved
//! only for the layout it was rendered from (a size, scale, zoom or minimised change between issue
//! and collection is a retry, at most [`MAX_RETRIES`]), and a monotonic clock. A request that
//! arrives before the first UI pass recorded the geometry waits instead of failing.
//!
//! Phases: [`Captures::issue`] runs in the UI pass (settling counts real layout passes; a
//! discarded pass is skipped); [`Captures::note_geometry`] and [`Captures::drain`] run in the
//! logic phase. The image is encoded and written on the UI thread by
//! [`ControlChannel::finish_captures`](crate::control::ControlChannel::finish_captures), never
//! inside the callback.

use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use craft_ui::capture::CaptureSlot;
use egui::{ColorImage, Context, ScreenshotCallback};

use crate::control::ControlResponse;

/// Requests outstanding at once (issued, completed, waiting for geometry or waiting to be issued).
pub const MAX_PENDING: usize = 4;
/// Largest single image the slot may retain, and the largest a request may reserve.
pub const MAX_IMAGE_BYTES: usize = 128 * 1024 * 1024;
/// Aggregate reserved bytes over every outstanding request.
pub const MAX_TOTAL_BYTES: usize = 256 * 1024 * 1024;
/// A request expires after this long, including retries and settling.
pub const DEADLINE: Duration = Duration::from_secs(8);
/// UI passes a request waits before it is issued, on top of the animation delay.
pub const SETTLE_PASSES: u32 = 3;
/// Retries after the geometry changed while a capture was in flight.
pub const MAX_RETRIES: u8 = 2;

/// The document a screenshot belongs to, fixed at admission. `generation` changes whenever a
/// different document becomes active; `revision` is recorded for attribution only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DocumentId {
    pub generation: u64,
    pub revision: u64,
}

impl DocumentId {
    pub fn same_document(self, other: Self) -> bool {
        // pc: crates/ui-egui/src/screenshot.rs:73
        todo!()
    }
}

/// The layout a capture is rendered from: viewport size in points, pixels per point, zoom, the
/// texture side limit and whether the window is minimised.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CaptureGeometry {
    pub size_points: [f32; 2],
    pub pixels_per_point: f32,
    pub zoom: f32,
    pub max_texture_side: usize,
    pub minimized: bool,
    pub document: Option<DocumentId>,
}

impl CaptureGeometry {
    /// Read from `ctx` this pass.
    pub fn current(ctx: &Context, document: Option<DocumentId>) -> Self {
        // pc: crates/ui-egui/src/screenshot.rs:101
        todo!()
    }

    /// The image size in pixels, `None` when minimised or over the texture limit.
    pub fn expected_pixels(&self) -> Option<[usize; 2]> {
        // pc: crates/ui-egui/src/screenshot.rs:139
        todo!()
    }

    /// RGBA bytes of [`Self::expected_pixels`].
    pub fn expected_bytes(&self) -> Option<usize> {
        // pc: crates/ui-egui/src/screenshot.rs:153
        todo!()
    }
}

/// One outstanding request; holds the strong slot.
pub struct PendingCapture {
    token: u64,
    path: Option<String>,
    reply: Option<Sender<ControlResponse>>,
    deadline: Instant,
    ready_at: Instant,
    settle_passes: u32,
    geometry: CaptureGeometry,
    epoch: u64,
    issued_epoch: Option<u64>,
    reserved_bytes: usize,
    document: Option<DocumentId>,
    slot: Option<CaptureSlot>,
    retries: u8,
}

impl PendingCapture {
    pub fn token(&self) -> u64 {
        self.token
    }

    pub fn issued(&self) -> bool {
        self.issued_epoch.is_some()
    }
}

/// A request that is over: an image to save and reply, or an error.
pub struct Finished {
    pub token: u64,
    pub path: Option<String>,
    pub reply: Option<Sender<ControlResponse>>,
    pub result: Result<Arc<ColorImage>, String>,
}

/// A request admitted before any geometry was recorded.
struct WaitingAdmit {
    token: u64,
    path: Option<String>,
    reply: Sender<ControlResponse>,
    deadline: Instant,
    settle_ms: f64,
    document: Option<DocumentId>,
}

/// The screenshot request queue and its admission budget.
#[derive(Default)]
pub struct Captures {
    records: Vec<PendingCapture>,
    waiting: Vec<WaitingAdmit>,
    geometry: Option<CaptureGeometry>,
    epoch: u64,
}

impl Captures {
    pub fn pending(&self) -> usize {
        self.records.len() + self.waiting.len()
    }

    pub fn reserved_bytes(&self) -> usize {
        // pc: crates/ui-egui/src/screenshot.rs:256
        todo!()
    }

    pub fn any_pending(&self) -> bool {
        self.pending() > 0
    }

    /// Requests not yet issued (the app keeps repainting while any).
    pub fn any_unissued(&self) -> bool {
        // pc: crates/ui-egui/src/screenshot.rs:266
        todo!()
    }

    pub fn last_geometry(&self) -> Option<CaptureGeometry> {
        self.geometry
    }

    /// Record this pass's geometry; advances and returns the epoch when it changed.
    pub fn note_geometry(&mut self, geometry: CaptureGeometry) -> u64 {
        // pc: crates/ui-egui/src/screenshot.rs:276
        todo!()
    }

    /// Admit a request against the budget. `Err` returns the reply sender with the error text,
    /// PhotoCraft's word for word: `screenshot token {token} is already pending`,
    /// `too many screenshots pending (at most {MAX_PENDING})`,
    /// `the viewport is not capturable ({w}×{h} points at {ppp} px/pt, texture side {side})`,
    /// `a {N} MiB screenshot is over the {MAX_IMAGE_BYTES / 1 MiB} MiB per-image limit` (`N` the
    /// expected bytes / 1 MiB, integer division), or
    /// `screenshot budget exhausted: {total} bytes reserved of a {MAX_TOTAL_BYTES}-byte aggregate limit`
    /// (pc `screenshot.rs:299`, `:302`, `:544`, `:549`, `:558`). With no geometry yet the request
    /// waits.
    pub fn admit(
        &mut self,
        token: u64,
        path: Option<String>,
        reply: Sender<ControlResponse>,
        document: Option<DocumentId>,
        settle_ms: f64,
        now: Instant,
    ) -> Result<(), (Sender<ControlResponse>, String)> {
        // pc: crates/ui-egui/src/screenshot.rs:288
        todo!()
    }

    /// Move waiting requests into the queue once geometry is known; expire the rest.
    pub fn admit_waiting(&mut self, now: Instant) {
        // pc: crates/ui-egui/src/screenshot.rs:313
        todo!()
    }

    /// One real UI pass happened (settling).
    pub fn note_pass(&mut self) {
        // pc: crates/ui-egui/src/screenshot.rs:384
        todo!()
    }

    /// UI phase: issue due requests through `send` (a fresh slot and callback each), return the
    /// ones that expired meanwhile.
    pub fn issue(&mut self, ctx: &Context, now: Instant, document: Option<DocumentId>, send: impl FnMut(ScreenshotCallback)) -> Vec<Finished> {
        // pc: crates/ui-egui/src/screenshot.rs:392
        todo!()
    }

    /// Logic phase: collect completed images (retrying after an epoch advance), expire the late.
    pub fn drain(&mut self, now: Instant, current: Option<CaptureGeometry>, document: Option<DocumentId>) -> Vec<Finished> {
        // pc: crates/ui-egui/src/screenshot.rs:448
        todo!()
    }

    /// Fail every outstanding request with `reason` (quit, device lost).
    pub fn cancel_all(&mut self, reason: &str) -> Vec<Finished> {
        // pc: crates/ui-egui/src/screenshot.rs:520
        todo!()
    }
}

/// The settle delay for `animation_time`: a base that lets popup animations end plus the
/// animation time itself.
pub fn settle_ms(animation_time: f32) -> f64 {
    // pc: crates/ui-egui/src/screenshot.rs:636
    todo!()
}

/// Send the capture command for the root viewport.
pub fn send_command(ctx: &Context, callback: ScreenshotCallback) {
    // pc: crates/ui-egui/src/screenshot.rs:646
    todo!()
}

/// Encode unmultiplied RGBA pixels as an 8-bit RGBA PNG (the `png` crate, default
/// compression); an encoder error is its message. PhotoCraft encodes through its codecs crate.
pub fn encode_png(width: usize, height: usize, rgba: &[u8]) -> Result<Vec<u8>, String> {
    // pc: apps/photocraft/src/services.rs:694 (the encode_png service)
    todo!()
}

/// Writes a screenshot's PNG bytes to an automation path: the app's file authority (PhotoCraft's
/// `automation_write`, limited to its authorized workspace). `Err` is the reply's error text.
pub type PngWriter<'a> = dyn FnMut(&str, &[u8]) -> Result<(), String> + 'a;

/// The reply for a finished image, PhotoCraft's shapes:
/// - an image over [`MAX_IMAGE_BYTES`] as RGBA (or whose size overflows):
///   `{"ok": false, "error": "screenshot {token} failed: the image exceeds the {MAX_IMAGE_BYTES / 1 MiB} MiB encode budget"}`;
/// - no `path`: `{"ok": true, "result": {"base64": <standard base64 of the PNG>, "mime": "image/png", "width", "height"}}`,
///   with no size limit beyond the encode budget;
/// - a `path`: the PNG goes to `write(path, bytes)`, the app's file authority (PhotoCraft's
///   `automation_write`, limited to its authorized workspace), and the reply is
///   `{"ok": true, "result": {"path", "width", "height"}}`;
/// - an encode or write error `e`: `{"ok": false, "error": e}`.
///
/// Flatten the pixels to RGBA and drop the image before encoding, so the two copies don't overlap.
pub fn screenshot_reply(token: u64, image: Arc<ColorImage>, path: Option<&str>, write: &mut PngWriter<'_>) -> ControlResponse {
    // pc: crates/ui-egui/src/app/screenshot.rs:32 (finish_captures) and control/inspect.rs:115 (save_screenshot_rgba)
    todo!()
}

#[cfg(test)]
#[path = "screenshot_tests.rs"]
mod screenshot_tests;
