#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! The loopback JSON-lines TCP transport: one request per line, one reply per line.

use std::net::SocketAddr;
use std::sync::mpsc::Receiver;

use crate::control::ControlRequest;

/// Bind `127.0.0.1:port` (0 lets the OS pick; the returned address is the one a client needs),
/// authenticate each connection with `token`, forward requests and wake `ctx`. The app prints
/// `control server listening on <addr>`.
pub fn start(port: u16, token: String, ctx: egui::Context) -> std::io::Result<(Receiver<ControlRequest>, SocketAddr)> {
    // pc: apps/photocraft/src/control_server.rs:19
    todo!()
}

#[cfg(test)]
#[path = "server_tests.rs"]
mod server_tests;
