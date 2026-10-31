//! A JSON-lines client for integration tests and CLIs.

use std::io::{BufRead, BufReader, Write};
use std::net::SocketAddr;

use serde_json::{Value, json};

use crate::security::{AUTH_METHOD, IO_TIMEOUT};

/// One authenticated connection.
pub struct ControlClient {
    stream: std::net::TcpStream,
    reader: BufReader<std::net::TcpStream>,
    next_id: u64,
}

impl ControlClient {
    /// Connect (read timeout [`crate::security::IO_TIMEOUT`]), send
    /// `{"id": 1, "method": "auth", "params": {"token": token}}` and read one reply line; `Err` with
    /// the reply's `error` text when it isn't `ok`, or the I/O error's message.
    pub fn connect(addr: SocketAddr, token: &str) -> Result<Self, String> {
        let stream = std::net::TcpStream::connect(addr).map_err(|e| e.to_string())?;
        stream.set_read_timeout(Some(IO_TIMEOUT)).map_err(|e| e.to_string())?;
        stream.set_write_timeout(Some(IO_TIMEOUT)).map_err(|e| e.to_string())?;
        let reader = BufReader::new(stream.try_clone().map_err(|e| e.to_string())?);
        let mut client = Self { stream, reader, next_id: 2 };
        client.exchange(1, AUTH_METHOD, json!({"token": token}))?;
        Ok(client)
    }

    /// Send `{"id": n, "method", "params"}` as one line (ids count up from 2) and read one reply
    /// line; `Ok(result)` (`null` when absent) for `"ok": true`, `Err(error)` otherwise.
    pub fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        self.exchange(id, method, params)
    }

    fn exchange(&mut self, id: u64, method: &str, params: Value) -> Result<Value, String> {
        writeln!(self.stream, "{}", json!({"id": id, "method": method, "params": params})).map_err(|e| e.to_string())?;
        self.stream.flush().map_err(|e| e.to_string())?;
        let mut line = String::new();
        if self.reader.read_line(&mut line).map_err(|e| e.to_string())? == 0 {
            return Err("connection closed".into());
        }
        let mut reply: Value = serde_json::from_str(&line).map_err(|e| format!("bad reply: {e}"))?;
        if reply.get("ok") == Some(&Value::Bool(true)) {
            Ok(reply.get_mut("result").map(Value::take).unwrap_or(Value::Null))
        } else {
            Err(reply.get("error").and_then(Value::as_str).unwrap_or("request failed").to_string())
        }
    }
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod client_tests;
