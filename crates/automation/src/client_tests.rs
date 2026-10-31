//! Behaviour tests of `client`.

use std::time::Duration;

use serde_json::json;

use super::*;
use crate::server::start;

const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

// call returns Ok(result) for ok replies and Err(error) otherwise.
#[test]
fn call_maps_replies() {
    let (rx, addr) = start(0, TOKEN.into(), egui::Context::default()).unwrap();
    assert!(ControlClient::connect(addr, &"f".repeat(64)).is_err(), "a wrong token is refused");
    let mut client = ControlClient::connect(addr, TOKEN).unwrap();
    let handler = std::thread::spawn(move || {
        for method in ["good", "bare", "bad"] {
            let req = rx.recv_timeout(Duration::from_secs(5)).unwrap();
            assert_eq!(req.method, method);
            let reply = match method {
                "good" => json!({"ok": true, "result": {"n": 1}}),
                "bare" => json!({"ok": true}),
                _ => json!({"ok": false, "error": "nope"}),
            };
            req.reply.send(reply).unwrap();
        }
    });
    assert_eq!(client.call("good", json!({})), Ok(json!({"n": 1})));
    assert_eq!(client.call("bare", json!({})), Ok(serde_json::Value::Null));
    assert_eq!(client.call("bad", json!({})), Err("nope".to_string()));
    handler.join().unwrap();
}

// A stale reply left by a timed-out call is skipped, and a transport failure closes the client.
#[test]
fn replies_are_matched_by_id() {
    use std::io::{BufRead, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = std::io::BufReader::new(stream.try_clone().unwrap());
        let mut out = stream;
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        writeln!(out, "{}", json!({"id": 1, "ok": true})).unwrap();
        line.clear();
        reader.read_line(&mut line).unwrap();
        writeln!(out, "{}", json!({"id": 99, "ok": true, "result": "stale"})).unwrap();
        writeln!(out, "{}", json!({"id": 2, "ok": true, "result": "fresh"})).unwrap();
        // Closing the connection ends the third call.
    });
    let mut client = ControlClient::connect(addr, TOKEN).unwrap();
    assert_eq!(client.call("a", json!({})), Ok(json!("fresh")));
    server.join().unwrap();
    assert!(client.call("b", json!({})).is_err(), "the closed connection fails the call");
    assert_eq!(client.call("c", json!({})), Err("connection closed".to_string()), "a failed client stays closed");
}
