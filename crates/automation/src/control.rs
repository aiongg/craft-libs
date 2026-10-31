#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! The UI-thread side of the control channel.
//!
//! A transport sends [`ControlRequest`]s; once per frame, between the input and the UI pass, the
//! app drains them through [`ControlChannel::next_request`], runs its own method table on each
//! (`engine.execute`, `ui.inspect`, `ui.set`, `ui.dialog.*`, `ui.screenshot`, …) and hands the
//! [`Outcome`] to [`ControlChannel::settle`]. The channel replies at once, after the queued
//! synthetic input has been processed ([`Outcome::AfterInput`], so a following `ui.inspect` sees
//! the effect), when a background job ends ([`Outcome::AfterJob`]), or when a screenshot completes
//! ([`Outcome::Screenshot`], through [`crate::screenshot::Captures`]).
//!
//! Deadlines: a request still queued when its [`REPLY_DEADLINE`] passes is dropped before dispatch
//! and the transport replies [`expired_error`] (safe to re-send); one that was dispatched and gave
//! no reply in time gets [`timeout_error`] (it may still apply). [`Dispatch`] settles which of the
//! two happened, once, between the UI thread and the transport.

use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::screenshot::{Captures, DocumentId, Finished};

/// A reply: the JSON object the transport writes as one line.
pub type ControlResponse = Value;

/// A background job's id in `jobs.list`, `jobs.cancel` and [`Outcome::AfterJob`].
pub type JobId = u64;

/// The per-request reply deadline.
pub const REPLY_DEADLINE: Duration = Duration::from_secs(60);

/// The error for a dispatched request with no reply in [`REPLY_DEADLINE`]: the command may still
/// run and apply, so re-sending it blindly could apply an edit twice.
pub fn timeout_error() -> String {
    // pc: crates/ui-egui/src/control.rs:65
    todo!()
}

/// The error for a request still queued when its deadline passed: dropped before dispatch, safe to
/// re-send. Same `timeout` prefix as [`timeout_error`], which machines match on.
pub fn expired_error() -> String {
    // pc: crates/ui-egui/src/control.rs:75
    todo!()
}

/// One request from a transport, with where its reply goes.
pub struct ControlRequest {
    pub method: String,
    pub params: Value,
    pub reply: Sender<ControlResponse>,
    /// Requests still queued at this instant are rejected; dispatched work continues.
    pub deadline: Option<Instant>,
    /// Shared with the transport, so when it stops waiting it knows whether the command ran.
    pub dispatch: Dispatch,
}

impl ControlRequest {
    /// A request with no deadline and a fresh reply channel (tests, in-process clients).
    pub fn new(method: impl Into<String>, params: Value) -> (Self, Receiver<ControlResponse>) {
        // pc: crates/ui-egui/src/control.rs:93
        todo!()
    }

    /// `self` with its deadline [`REPLY_DEADLINE`] from now.
    pub fn with_deadline(self) -> Self {
        // pc: apps/photocraft/src/control_server.rs:113
        todo!()
    }
}

/// Whether a request was dispatched, settled once between the UI thread (about to run it) and the
/// transport (giving up on its reply).
#[derive(Clone, Debug, Default)]
pub struct Dispatch(std::sync::Arc<std::sync::atomic::AtomicU8>);

impl Dispatch {
    /// The UI thread claims the request to run it: false once the transport gave up on it.
    pub fn start(&self) -> bool {
        // pc: crates/ui-egui/src/control.rs:112
        todo!()
    }

    /// The transport stops waiting: [`expired_error`] when the command can no longer run,
    /// [`timeout_error`] when it was dispatched and may still apply.
    pub fn give_up(&self) -> String {
        // pc: crates/ui-egui/src/control.rs:118
        todo!()
    }
}

/// What the app's method table answers for one request.
pub enum Outcome {
    /// Reply with this JSON now.
    Done(Value),
    /// A `ui.screenshot`: admit `token` to the queue; `path` is where the app's write authority
    /// puts the PNG, `None` inlines it as base64 in the reply.
    Screenshot { token: u64, path: Option<String> },
    /// Synthetic input was queued: reply once the app has processed all of it.
    AfterInput,
    /// A command started a background job and the caller waits: reply with its result when it
    /// has been applied, or with its error or `cancelled`.
    AfterJob(JobId),
}

impl Outcome {
    /// `Done` with an `ok` or `error` reply from a method's result.
    pub fn from_result(r: Result<Value, String>) -> Self {
        // pc: crates/ui-egui/src/control.rs:143 (wrap)
        todo!()
    }
}

/// The requests waiting for the app, and the replies waiting on input, jobs and screenshots.
///
/// Per frame: `while let Some(req) = channel.next_request() { let outcome = app.handle(&req); channel.settle(ctx, req, outcome, document); }`
/// in the logic phase; [`Self::input_processed`] after the pass that consumed the synthetic
/// input; [`Self::job_finished`] when a job ends; [`Self::captures`] driven by the app's UI and
/// logic phases as `screenshot` documents.
pub struct ControlChannel {
    rx: Receiver<ControlRequest>,
    input_waiters: Vec<Sender<ControlResponse>>,
    job_waiters: Vec<(JobId, Sender<ControlResponse>)>,
    /// The screenshot queue; the app calls its `issue` and `drain` from its UI and logic phases.
    pub captures: Captures,
}

impl ControlChannel {
    /// Attach a transport's receiver (TCP, stdin, tests).
    pub fn new(rx: Receiver<ControlRequest>) -> Self {
        // pc: crates/ui-egui/src/app/control.rs:5
        todo!()
    }

    /// The next request to run, claimed through [`Dispatch::start`]. Expired or abandoned
    /// requests are replied with [`expired_error`] here and skipped. `None` when the queue is
    /// empty this frame.
    pub fn next_request(&mut self) -> Option<ControlRequest> {
        // pc: crates/ui-egui/src/app/control.rs:23
        todo!()
    }

    /// Reply now, or park the reply on input, a job or a screenshot. `settle_ms` is the
    /// screenshot settle delay ([`crate::screenshot::settle_ms`] of the animation time);
    /// `document` the identity a screenshot is attributed to.
    pub fn settle(&mut self, req: ControlRequest, outcome: Outcome, document: Option<DocumentId>, settle_ms: f64) {
        // pc: crates/ui-egui/src/app/control.rs:31
        todo!()
    }

    /// The pass that consumed the queued synthetic input ended: reply
    /// `{"ok": true, "result": null}` to every [`Outcome::AfterInput`] waiter. Call it only after a
    /// pass that reached the end of the UI with the synthetic queue empty; an early-returning pass
    /// keeps the waiters.
    pub fn input_processed(&mut self) {
        // pc: crates/ui-egui/src/app/frame.rs:207
        todo!()
    }

    /// The queued synthetic input was dropped (the window lost it, a modal took over): reply
    /// `{"ok": false, "error": "input cancelled: {reason}"}` to every [`Outcome::AfterInput`]
    /// waiter.
    pub fn input_cancelled(&mut self, reason: &str) {
        // pc: crates/ui-egui/src/app/input.rs:112
        todo!()
    }

    /// Job `job` ended: reply to its waiters with `{"ok": true, "result": value}` for `Ok(value)`
    /// and `{"ok": false, "error": e}` for `Err(e)` (a cancelled job is `Err("cancelled")`).
    pub fn job_finished(&mut self, job: JobId, result: &Result<Value, String>) {
        // pc: crates/ui-egui/src/jobs_ui.rs:458 (on_event)
        todo!()
    }

    /// Reply to finished screenshots with [`crate::screenshot::screenshot_reply`] (a capture
    /// error `e` as `{"ok": false, "error": e}`); `write` is the app's file authority for replies
    /// with a `path`.
    pub fn finish_captures(&mut self, finished: Vec<Finished>, write: &mut crate::screenshot::PngWriter<'_>) {
        // pc: crates/ui-egui/src/app/screenshot.rs:32
        todo!()
    }

    /// Waiters of every kind, for `ui.inspect`'s `pending` count and tests.
    pub fn waiting(&self) -> usize {
        // new: PhotoCraft reads the three vectors directly
        todo!()
    }
}

#[cfg(test)]
#[path = "control_tests.rs"]
mod control_tests;
