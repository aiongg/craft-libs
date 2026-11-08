//! `ControlChannel`, the loopback server and the input parameters of the protocol.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use craft_automation::control::{ControlChannel, ControlRequest, ControlResponse, Reply, expired_error, timeout_error};
use craft_automation::screenshot::{CaptureGeometry, Finished};
use craft_automation::security::{AUTH_DEADLINE, MAX_REQUEST_BYTES};
use craft_automation::server;
use craft_automation::synthetic::{Click, KeyPress, move_from_params};
use craft_behavior::PointerButton;
use craft_keymap::Key;
use craft_ui::runtime::windows::WindowHost;
use craft_ui::testing::trial;
use egui::{Context, ViewportId};
use serde_json::{Value, json};

/// An expired queued request is replied `expired_error` and never dispatched; a dispatched one that times out gets
/// `timeout_error`.
#[trial]
fn deadlines_distinguish_expired_from_timed_out() {
    let (tx, rx) = channel();
    let mut channel = ControlChannel::new(rx);

    let (mut stale, stale_reply) = ControlRequest::new("ui.inspect", json!({}));
    stale.deadline = Some(Instant::now() - Duration::from_secs(1));
    let stale_dispatch = stale.dispatch.clone();
    let (live, _live_reply) = ControlRequest::new("ui.key", json!({}));
    let live_dispatch = live.dispatch.clone();
    tx.send(stale).expect("sent");
    tx.send(live).expect("sent");

    let next = channel.next_request().expect("the live request is dispatched");
    assert_eq!(next.method, "ui.key");
    assert!(channel.next_request().is_none());
    assert_eq!(stale_reply.try_recv().expect("a reply"), json!({"ok": false, "error": expired_error()}));
    assert_eq!(stale_dispatch.give_up(), expired_error());
    assert_eq!(live_dispatch.give_up(), timeout_error());
    assert!(timeout_error().starts_with("timeout:") && expired_error().starts_with("timeout:"));

    // A request the transport abandoned before the UI thread claimed it is not run.
    let (abandoned, abandoned_reply) = ControlRequest::new("ui.click", json!({}));
    let _ = abandoned.dispatch.give_up();
    tx.send(abandoned).expect("sent");
    assert!(channel.next_request().is_none());
    assert_eq!(abandoned_reply.try_recv().expect("a reply")["error"], json!(expired_error()));
}

fn park(channel: &mut ControlChannel, outcome: Reply) -> Receiver<ControlResponse> {
    let (req, reply) = ControlRequest::new("m", json!({}));
    channel.settle(req, outcome, None, 0.0);
    reply
}

/// `AfterInput` waiters are answered after `input_processed`; `AfterJob` waiters with the job's result.
#[trial]
fn waiters_reply_when_input_or_job_settles() {
    let (_tx, rx) = channel();
    let mut channel = ControlChannel::new(rx);
    let input = park(&mut channel, Reply::AfterInput);
    let cancelled = park(&mut channel, Reply::AfterInput);
    let job = park(&mut channel, Reply::AfterJob(7));
    let other = park(&mut channel, Reply::AfterJob(8));
    let done = park(&mut channel, Reply::Done(json!({"ok": true, "result": 1})));
    assert_eq!(done.try_recv().expect("a reply")["result"], json!(1));
    assert_eq!(channel.waiting(), 4);
    assert!(input.try_recv().is_err() && job.try_recv().is_err());

    channel.input_processed();
    assert_eq!(input.try_recv().expect("a reply"), json!({"ok": true, "result": null}));
    assert!(cancelled.try_recv().is_ok());

    channel.job_finished(7, &Err("cancelled".into()));
    assert_eq!(job.try_recv().expect("a reply"), json!({"ok": false, "error": "cancelled"}));
    assert!(other.try_recv().is_err());
    channel.job_finished(8, &Ok(json!("x")));
    assert_eq!(other.try_recv().expect("a reply"), json!({"ok": true, "result": "x"}));
    assert_eq!(channel.waiting(), 0);

    let waiting = park(&mut channel, Reply::AfterInput);
    channel.input_cancelled("gone");
    assert_eq!(waiting.try_recv().expect("a reply"), json!({"ok": false, "error": "input cancelled: gone"}));
}

/// Cancelling with only waiters left (the input was already consumed) still fails them, and input the host still
/// holds counts as pending.
#[trial]
fn cancel_input_fails_waiters_without_queued_input() {
    let (_tx, rx) = channel();
    let mut channel = ControlChannel::new(rx);
    let host = WindowHost::of(&Context::default());
    assert!(!channel.cancel_input(&host, "idle"));
    let waiting = park(&mut channel, Reply::AfterInput);
    assert!(channel.cancel_input(&host, "lost"));
    assert_eq!(waiting.try_recv().expect("a reply"), json!({"ok": false, "error": "input cancelled: lost"}));
    host.inject_text(ViewportId::ROOT, "x");
    assert!(!host.injected_idle());
    assert!(channel.cancel_input(&host, "again") && host.injected_idle());
}

/// A screenshot the capture budget refuses is replied to at once, and a finished capture is answered through the
/// writer.
#[trial]
fn settle_refuses_a_screenshot_over_budget_and_finishes_the_others() {
    let (_tx, rx) = channel();
    let mut control = ControlChannel::new(rx);
    control.captures.note_geometry(CaptureGeometry {
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
    assert!(refused["error"].as_str().expect("an error").contains("per-image limit"), "{refused}");
    assert_eq!(control.waiting(), 0);

    let (tx, done) = channel::<ControlResponse>();
    let finished = vec![Finished { token: 2, path: None, reply: Some(tx), result: Err("capture failed".into()) }];
    control.finish_captures(finished, &mut |_, _| Ok(()));
    assert_eq!(done.try_recv().expect("a reply"), json!({"ok": false, "error": "capture failed"}));
}

const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn read_json(reader: &mut impl BufRead) -> Option<Value> {
    let mut line = String::new();
    (reader.read_line(&mut line).ok()? > 0).then(|| serde_json::from_str(&line).expect("a JSON line"))
}

fn host() -> WindowHost {
    WindowHost::of(&Context::default())
}

/// A client that fails auth is closed; an authenticated request reaches the channel.
#[trial]
fn server_authenticates_then_forwards() {
    let (rx, addr) = server::start(0, TOKEN.into(), host()).expect("started");
    assert_ne!(addr.port(), 0);

    let mut bad = TcpStream::connect(addr).expect("connected");
    bad.set_read_timeout(Some(Duration::from_secs(5))).expect("timeout");
    let mut bad_reader = BufReader::new(bad.try_clone().expect("cloned"));
    writeln!(bad, "{}", json!({"id": 1, "method": "auth", "params": {"token": "f".repeat(64)}})).expect("written");
    assert_eq!(read_json(&mut bad_reader).expect("a reply")["ok"], false);
    assert!(read_json(&mut bad_reader).is_none(), "a failed auth closes the connection");
    assert!(rx.try_recv().is_err());

    let mut stream = TcpStream::connect(addr).expect("connected");
    stream.set_read_timeout(Some(Duration::from_secs(5))).expect("timeout");
    let mut reader = BufReader::new(stream.try_clone().expect("cloned"));
    writeln!(stream, "{}", json!({"id": 1, "method": "auth", "params": {"token": TOKEN}})).expect("written");
    assert_eq!(read_json(&mut reader).expect("a reply")["ok"], true);
    let handler = std::thread::spawn(move || {
        let req = rx.recv_timeout(Duration::from_secs(5)).expect("a request");
        assert_eq!(req.method, "test.echo");
        assert_eq!(req.params, json!({"a": 1}));
        assert!(req.deadline.is_some());
        req.reply.send(json!({"ok": true, "result": "hi"})).expect("replied");
    });
    writeln!(stream, "{}", json!({"id": 7, "method": "test.echo", "params": {"a": 1}})).expect("written");
    let reply = read_json(&mut reader).expect("a reply");
    assert_eq!((reply["id"].clone(), reply["result"].clone()), (json!(7), json!("hi")));
    handler.join().expect("the handler ran");
}

/// A connection that keeps sending without authenticating is closed at the deadline, whatever it sends, and nothing it
/// sent is dispatched.
#[trial]
fn an_unauthenticated_connection_is_closed_at_the_deadline_whatever_it_sends() {
    let (rx, addr) = server::start(0, TOKEN.into(), host()).expect("started");
    // Blank lines and partial frames, one byte every 20 ms: each read succeeds long before the idle timeout.
    let mut stream = TcpStream::connect(addr).expect("connected");
    let started = Instant::now();
    let mut closed = false;
    for i in 0..1000 {
        let byte: &[u8] = if i % 5 == 0 { b"\n" } else { b" " };
        if stream.write_all(byte).is_err() {
            closed = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let served = started.elapsed();
    assert!(closed, "the trickle did not keep the connection open");
    assert!(served < AUTH_DEADLINE + Duration::from_secs(3), "the trickle did not extend the deadline: {served:?}");
    assert!(rx.try_recv().is_err(), "nothing was dispatched");
}

/// A request line over `MAX_REQUEST_BYTES` without a newline gets an error reply and the connection is closed.
#[trial]
fn an_over_long_request_is_refused_and_closes_the_connection() {
    let (rx, addr) = server::start(0, TOKEN.into(), host()).expect("started");
    let mut stream = TcpStream::connect(addr).expect("connected");
    stream.set_read_timeout(Some(Duration::from_secs(5))).expect("timeout");
    let mut reader = BufReader::new(stream.try_clone().expect("cloned"));
    writeln!(stream, "{}", json!({"id": 1, "method": "auth", "params": {"token": TOKEN}})).expect("written");
    assert_eq!(read_json(&mut reader).expect("a reply")["ok"], true);
    // The server may close while this is still being written.
    let _ = stream.write_all(&vec![b'x'; MAX_REQUEST_BYTES + 16]);
    let reply = read_json(&mut reader).expect("a reply");
    assert_eq!(reply["ok"], false);
    assert!(reply["error"].as_str().expect("an error").starts_with("request exceeds"), "{reply}");
    assert!(read_json(&mut reader).is_none(), "the connection is closed");
    assert!(rx.try_recv().is_err());
}

/// `ui.click`, `ui.move` and `ui.key` parameters are validated, and every key name the protocol takes parses.
#[trial]
fn input_parameters_are_validated() {
    let click = Click::from_params(&json!({"x": 3, "y": 4, "button": "secondary", "count": 2})).expect("a click");
    assert_eq!(click, Click { at: egui::pos2(3.0, 4.0), button: PointerButton::Secondary, count: 2 });
    assert_eq!(Click::from_params(&json!({})).expect("a click").count, 1);
    assert_eq!(Click::from_params(&json!({"count": 257})).unwrap_err(), "`count` must be at most 256 (got 257)");
    assert!(Click::from_params(&json!({"z": 1})).unwrap_err().starts_with("unknown field `z`"));
    assert_eq!(move_from_params(&json!({"x": 1.5})).expect("a point"), egui::pos2(1.5, 0.0));

    let names = [
        "Enter",
        "Escape",
        "Tab",
        "Space",
        "Backspace",
        "Delete",
        "Insert",
        "Home",
        "End",
        "PageUp",
        "PageDown",
        "ArrowUp",
        "ArrowDown",
        "ArrowLeft",
        "ArrowRight",
        "A",
        "Z",
        "Num0",
        "Num9",
        "F1",
        "F5",
        "F12",
        "Minus",
        "Equals",
        "Comma",
        "Period",
        "Semicolon",
        "Slash",
        "Backslash",
        "OpenBracket",
        "CloseBracket",
    ];
    for name in names {
        assert!(KeyPress::from_params(&json!({"key": name})).is_ok(), "{name} is a key name the protocol takes");
    }
    let press = KeyPress::from_params(&json!({"key": "Enter", "shift": true})).expect("a key");
    assert_eq!((press.key, press.mods.shift), (Key::Enter, true));
    assert_eq!(KeyPress::from_params(&json!({})).unwrap_err(), "missing `key`");
    assert_eq!(KeyPress::from_params(&json!({"key": "Nope"})).unwrap_err(), "unknown key `Nope`");
}
