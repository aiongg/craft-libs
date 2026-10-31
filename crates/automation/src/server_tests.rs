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

// A connection that keeps sending without authenticating is closed at the deadline, whatever it
// sends, and nothing it sent is dispatched.
#[test]
fn an_unauthenticated_connection_is_closed_at_the_deadline_whatever_it_sends() {
    use std::sync::mpsc::channel;
    use std::time::Instant;

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = channel::<ControlRequest>();
    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let started = Instant::now();
        serve(stream, "unused", &tx, &egui::Context::default(), Duration::from_millis(300));
        started.elapsed()
    });
    // Blank lines and partial frames, one byte every 20 ms: each read succeeds long before the
    // idle timeout.
    let mut stream = TcpStream::connect(addr).unwrap();
    for i in 0..150 {
        let byte: &[u8] = if i % 5 == 0 { b"\n" } else { b" " };
        if stream.write_all(byte).is_err() {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let served = server.join().unwrap();
    assert!(served < Duration::from_secs(3), "the trickle did not extend the deadline: {served:?}");
    assert!(rx.try_recv().is_err(), "nothing was dispatched");
}

// A request line over MAX_REQUEST_BYTES without a newline gets an error reply and the connection is closed.
#[test]
fn an_over_long_request_is_refused_and_closes_the_connection() {
    let (rx, addr) = start(0, TOKEN.into(), egui::Context::default()).unwrap();
    let mut stream = TcpStream::connect(addr).unwrap();
    stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    writeln!(stream, "{}", json!({"id": 1, "method": "auth", "params": {"token": TOKEN}})).unwrap();
    assert_eq!(read_json(&mut reader).unwrap()["ok"], true);
    // The server may close while this is still being written.
    let _ = stream.write_all(&vec![b'x'; crate::security::MAX_REQUEST_BYTES + 16]);
    let reply = read_json(&mut reader).unwrap();
    assert_eq!(reply["ok"], false);
    assert!(reply["error"].as_str().unwrap().starts_with("request exceeds"), "{reply}");
    assert!(read_json(&mut reader).is_none(), "the connection is closed");
    assert!(rx.try_recv().is_err());
}
