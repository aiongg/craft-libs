#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! A JSON-lines client for integration tests and CLIs.

use std::net::SocketAddr;

use serde_json::Value;

/// One authenticated connection.
pub struct ControlClient {
    stream: std::net::TcpStream,
    next_id: u64,
}

impl ControlClient {
    /// Connect (read timeout [`crate::security::IO_TIMEOUT`]), send
    /// `{"id": 1, "method": "auth", "params": {"token": token}}` and read one reply line; `Err` with
    /// the reply's `error` text when it isn't `ok`, or the I/O error's message.
    pub fn connect(addr: SocketAddr, token: &str) -> Result<Self, String> {
        // new: PhotoCraft has no client type; the exchange is its test's, apps/photocraft/src/control_server.rs:203 (and crates/automation/src/bridge.rs:141 auth)
        todo!()
    }

    /// Send `{"id": n, "method", "params"}` as one line (ids count up from 2) and read one reply
    /// line; `Ok(result)` (`null` when absent) for `"ok": true`, `Err(error)` otherwise.
    pub fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        // new: as connect, apps/photocraft/src/control_server.rs:247
        todo!()
    }
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod client_tests;
