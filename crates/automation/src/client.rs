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
    /// Connect and send `auth`; `Err` with the server's error text when the token is refused.
    pub fn connect(addr: SocketAddr, token: &str) -> Result<Self, String> {
        // pc: apps/photocraft-cli/src/automation.rs:14 (commands)
        todo!()
    }

    /// Send `method` with `params`; `Ok(result)` or `Err(error)` from the reply.
    pub fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        // pc: apps/photocraft-cli/src/automation.rs:14
        todo!()
    }
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod client_tests;
