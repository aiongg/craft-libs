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

fn park(channel: &mut ControlChannel, outcome: Outcome) -> Receiver<ControlResponse> {
    let (req, reply) = ControlRequest::new("m", json!({}));
    channel.settle(req, outcome, None, 0.0);
    reply
}

// AfterInput waiters are answered after input_processed; AfterJob waiters with the job's result.
#[test]
fn waiters_reply_when_input_or_job_settles() {
    let (_tx, rx) = channel();
    let mut channel = ControlChannel::new(rx);
    let input = park(&mut channel, Outcome::AfterInput);
    let cancelled = park(&mut channel, Outcome::AfterInput);
    let job = park(&mut channel, Outcome::AfterJob(7));
    let other = park(&mut channel, Outcome::AfterJob(8));
    let done = park(&mut channel, Outcome::Done(json!({"ok": true, "result": 1})));
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

    let waiting = park(&mut channel, Outcome::AfterInput);
    channel.input_cancelled("gone");
    assert_eq!(waiting.try_recv().unwrap(), json!({"ok": false, "error": "input cancelled: gone"}));
}
