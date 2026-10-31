#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! The JSON-lines wire format and the parameter validators every method uses.
//!
//! Request: `{"id": 1, "method": "ui.click", "params": {"x": 10, "y": 20}}` on one line. Reply:
//! `{"id": 1, "ok": true, "result": …}` or `{"id": 1, "ok": false, "error": "…"}` on one line,
//! at most [`MAX_RESPONSE_BYTES`]. Validators reject unknown fields, wrong types, unknown choices
//! and out-of-range numbers before a method acts, naming the parameter
//! (`ui.click: 'button' must be one of left, right, secondary, middle`).

use std::io::Write;

use serde_json::{Map, Value};

/// Longest reply, in bytes; a longer one is replaced by an error reply naming the method.
pub const MAX_RESPONSE_BYTES: usize = 8 << 20;
/// Largest PNG the screenshot reply inlines as base64; larger images need a `path`.
pub const MAX_PNG_BYTES: usize = 5 << 20;

/// One parsed request line.
#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    pub id: Value,
    pub method: String,
    pub params: Value,
}

impl Request {
    /// Parse a line: `id` defaults to `null`, `method` to `""`, `params` to `{}`.
    pub fn parse(line: &str) -> Result<Self, String> {
        // pc: apps/photocraft/src/control_server.rs:110
        todo!()
    }
}

/// `{"id", "ok": true, "result"}`.
pub fn reply_ok(id: &Value, result: Value) -> Value {
    // pc: crates/ui-egui/src/control/params.rs:58
    todo!()
}

/// `{"id", "ok": false, "error"}`.
pub fn reply_err(id: &Value, error: impl std::fmt::Display) -> Value {
    // pc: crates/ui-egui/src/control/params.rs:62
    todo!()
}

/// Write `reply` as one line, replacing a reply over [`MAX_RESPONSE_BYTES`] by an error reply.
pub fn write_reply(out: &mut impl Write, reply: &Value) -> std::io::Result<()> {
    // pc: crates/automation/src/budgets.rs:81
    todo!()
}

/// The pointer buttons a request may name.
pub const BUTTONS: [&str; 4] = ["left", "right", "secondary", "middle"];
/// The modifier flags of `ui.key`.
pub const KEY_MODIFIERS: [&str; 4] = ["shift", "alt", "command", "ctrl"];
/// The modifier flags of `ui.pointer` (`space` is the momentary pan).
pub const POINTER_MODIFIERS: [&str; 5] = ["shift", "alt", "command", "ctrl", "space"];

/// Typed readers over a request's `params` object; every error names the field.
#[derive(Clone, Copy, Debug)]
pub struct Params<'a>(pub &'a Value);

impl Params<'_> {
    /// `key` as a bool, `None` when absent.
    pub fn bool_field(&self, key: &str) -> Result<Option<bool>, String> {
        // pc: crates/ui-egui/src/control/params.rs:67
        todo!()
    }

    /// `key` as a non-negative integer.
    pub fn uint_field(&self, key: &str) -> Result<Option<u64>, String> {
        // pc: crates/ui-egui/src/control/params.rs:76
        todo!()
    }

    /// `key` as a finite number.
    pub fn num_field(&self, key: &str) -> Result<Option<f64>, String> {
        // pc: crates/ui-egui/src/control/params.rs:85
        todo!()
    }

    /// `key` as a finite number within `f32` range.
    pub fn f32_field(&self, key: &str) -> Result<Option<f64>, String> {
        // pc: crates/ui-egui/src/control/params.rs:99
        todo!()
    }

    /// `key` as a string.
    pub fn str_field(&self, key: &str) -> Result<Option<&str>, String> {
        // pc: crates/ui-egui/src/control/params.rs:183 (input_choice without the accepted list)
        todo!()
    }

    /// The params must be an object with only `fields`; `what` names the method in errors.
    pub fn only(&self, what: &str, fields: &[&str]) -> Result<&Map<String, Value>, String> {
        // pc: crates/ui-egui/src/control/params.rs:160 (input_object)
        todo!()
    }

    /// `key` as a number within `range` (inclusive), `name` in the error.
    pub fn number_in(&self, key: &str, name: &str, range: Option<(f64, f64)>) -> Result<Option<f64>, String> {
        // pc: crates/ui-egui/src/control/params.rs:174 (input_number)
        todo!()
    }

    /// `key` as one of `accepted`.
    pub fn choice<'v>(&'v self, key: &str, name: &str, accepted: &[&str]) -> Result<Option<&'v str>, String> {
        // pc: crates/ui-egui/src/control/params.rs:183 (input_choice)
        todo!()
    }

    /// `modifiers` as an object of the `flags` only, each a bool.
    pub fn modifiers(&self, flags: &[&str]) -> Result<egui::Modifiers, String> {
        // pc: crates/ui-egui/src/control/params.rs:194 (input_modifiers), :223 (modifiers_from)
        todo!()
    }

    /// Deserialize `T` from the whole object; unknown fields are errors.
    pub fn whole<T: serde::de::DeserializeOwned>(&self, what: &str) -> Result<T, String> {
        // pc: crates/ui-egui/src/control/params.rs:132 (whole_object)
        todo!()
    }

    /// `current` serialized, the object's fields merged over it, deserialized back: a partial
    /// `ui.set`-style update.
    pub fn merged_into<T: serde::Serialize + serde::de::DeserializeOwned>(&self, what: &str, current: &T) -> Result<T, String> {
        // pc: crates/ui-egui/src/control/params.rs:109 (merged_object)
        todo!()
    }
}

#[cfg(test)]
#[path = "protocol_tests.rs"]
mod protocol_tests;
