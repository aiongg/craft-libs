//! Behaviour tests of `control`.

use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use serde_json::json;

use super::*;

// An expired queued request is replied `expired_error` and never dispatched; a dispatched one that
// times out gets `timeout_error`.
#[test]
fn deadlines_distinguish_expired_from_timed_out() {
    let (tx, rx) = channel();
    let mut channel = ControlChannel::new(rx);

    let (mut stale, stale_reply) = ControlRequest::new("ui.inspect", json!({}));
    stale.deadline = Some(Instant::now() - Duration::from_secs(1));
    let stale_dispatch = stale.dispatch.clone();
    let (live, _live_reply) = ControlRequest::new("ui.key", json!({}));
    let live_dispatch = live.dispatch.clone();
    tx.send(stale).unwrap();
    tx.send(live).unwrap();

    let next = channel.next_request().expect("the live request is dispatched");
    assert_eq!(next.method, "ui.key");
    assert!(channel.next_request().is_none());
    assert_eq!(stale_reply.try_recv().unwrap(), json!({"ok": false, "error": expired_error()}));
    assert_eq!(stale_dispatch.give_up(), expired_error());
    assert_eq!(live_dispatch.give_up(), timeout_error());
    assert!(timeout_error().starts_with("timeout:") && expired_error().starts_with("timeout:"));

    // A request the transport abandoned before the UI thread claimed it is not run.
    let (abandoned, abandoned_reply) = ControlRequest::new("ui.click", json!({}));
    let _ = abandoned.dispatch.give_up();
    tx.send(abandoned).unwrap();
    assert!(channel.next_request().is_none());
    assert_eq!(abandoned_reply.try_recv().unwrap()["error"], json!(expired_error()));
}

fn park(channel: &mut ControlChannel, outcome: Reply) -> Receiver<ControlResponse> {
    let (req, reply) = ControlRequest::new("m", json!({}));
    channel.settle(req, outcome, None, 0.0);
    reply
}

// AfterInput waiters are answered after input_processed; AfterJob waiters with the job's result.
#[test]
fn waiters_reply_when_input_or_job_settles() {
    let (_tx, rx) = channel();
    let mut channel = ControlChannel::new(rx);
    let input = park(&mut channel, Reply::AfterInput);
    let cancelled = park(&mut channel, Reply::AfterInput);
    let job = park(&mut channel, Reply::AfterJob(7));
    let other = park(&mut channel, Reply::AfterJob(8));
    let done = park(&mut channel, Reply::Done(json!({"ok": true, "result": 1})));
    assert_eq!(done.try_recv().unwrap()["result"], json!(1));
    assert_eq!(channel.waiting(), 4);
    assert!(input.try_recv().is_err() && job.try_recv().is_err());

    channel.input_processed();
    assert_eq!(input.try_recv().unwrap(), json!({"ok": true, "result": null}));
    assert!(cancelled.try_recv().is_ok());

    channel.job_finished(7, &Err("cancelled".into()));
    assert_eq!(job.try_recv().unwrap(), json!({"ok": false, "error": "cancelled"}));
    assert!(other.try_recv().is_err());
    channel.job_finished(8, &Ok(json!("x")));
    assert_eq!(other.try_recv().unwrap(), json!({"ok": true, "result": "x"}));
    assert_eq!(channel.waiting(), 0);

    let waiting = park(&mut channel, Reply::AfterInput);
    channel.input_cancelled("gone");
    assert_eq!(waiting.try_recv().unwrap(), json!({"ok": false, "error": "input cancelled: gone"}));
}

// Cancelling with only waiters left (the input was already consumed) still fails them.
#[test]
fn cancel_input_fails_waiters_without_queued_input() {
    let (_tx, rx) = channel();
    let mut channel = ControlChannel::new(rx);
    let mut input = SyntheticInput::default();
    assert!(!channel.cancel_input(&mut input, "idle"));
    let waiting = park(&mut channel, Reply::AfterInput);
    assert!(channel.cancel_input(&mut input, "lost"));
    assert_eq!(waiting.try_recv().unwrap(), json!({"ok": false, "error": "input cancelled: lost"}));
    input.type_text("x");
    assert!(channel.cancel_input(&mut input, "again") && input.is_empty());
}

// A screenshot the capture budget refuses is replied to at once, and a finished capture is answered through the writer.
#[test]
fn settle_refuses_a_screenshot_over_budget_and_finishes_the_others() {
    let (_tx, rx) = channel();
    let mut control = ControlChannel::new(rx);
    control.captures.note_geometry(crate::screenshot::CaptureGeometry {
        size_points: [12_000.0, 12_000.0],
        pixels_per_point: 1.0,
        zoom: 1.0,
        max_texture_side: 16384,
        minimized: false,
        document: None,
    });
    let (req, reply) = ControlRequest::new("ui.screenshot", json!({}));
    control.settle(req, Reply::Screenshot { token: 1, path: None }, None, 0.0);
    let refused = reply.try_recv().expect("refused at once");
    assert_eq!(refused["ok"], false);
    assert!(refused["error"].as_str().unwrap().contains("per-image limit"), "{refused}");
    assert_eq!(control.waiting(), 0);

    let (tx, done) = channel::<ControlResponse>();
    let finished = vec![crate::screenshot::Finished { token: 2, path: None, reply: Some(tx), result: Err("capture failed".into()) }];
    control.finish_captures(finished, &mut |_, _| Ok(()));
    assert_eq!(done.try_recv().unwrap(), json!({"ok": false, "error": "capture failed"}));
}
