//! Behaviour tests of `server`.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

use serde_json::{Value, json};

use super::*;

const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn read_json(reader: &mut impl BufRead) -> Option<Value> {
    let mut line = String::new();
    (reader.read_line(&mut line).ok()? > 0).then(|| serde_json::from_str(&line).unwrap())
}

// A client that fails auth is closed; an authenticated request reaches the channel.
#[test]
fn server_authenticates_then_forwards() {
    let (rx, addr) = start(0, TOKEN.into(), egui::Context::default()).unwrap();
    assert_ne!(addr.port(), 0);

    let mut bad = TcpStream::connect(addr).unwrap();
    bad.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let mut bad_reader = BufReader::new(bad.try_clone().unwrap());
    writeln!(bad, "{}", json!({"id": 1, "method": "auth", "params": {"token": "f".repeat(64)}})).unwrap();
    assert_eq!(read_json(&mut bad_reader).unwrap()["ok"], false);
    assert!(read_json(&mut bad_reader).is_none(), "a failed auth closes the connection");
    assert!(rx.try_recv().is_err());

    let mut stream = TcpStream::connect(addr).unwrap();
    stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    writeln!(stream, "{}", json!({"id": 1, "method": "auth", "params": {"token": TOKEN}})).unwrap();
    assert_eq!(read_json(&mut reader).unwrap()["ok"], true);
    let handler = std::thread::spawn(move || {
        let req = rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(req.method, "test.echo");
        assert_eq!(req.params, json!({"a": 1}));
        assert!(req.deadline.is_some());
        req.reply.send(json!({"ok": true, "result": "hi"})).unwrap();
    });
    writeln!(stream, "{}", json!({"id": 7, "method": "test.echo", "params": {"a": 1}})).unwrap();
    let reply = read_json(&mut reader).unwrap();
    assert_eq!((reply["id"].clone(), reply["result"].clone()), (json!(7), json!("hi")));
    handler.join().unwrap();
}
