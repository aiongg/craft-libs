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
