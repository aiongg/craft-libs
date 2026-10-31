#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! Synthetic pointer and keyboard input: `ui.click`, `ui.move`, `ui.key`, `ui.type`.
//!
//! A request becomes a list of egui event batches; the app injects one batch per pass into its
//! `RawInput` ([`SyntheticInput::inject`]) so presses and releases land on separate passes, as real
//! input does. Screen points are in egui points. Document-coordinate `ui.pointer` events stay in
//! the app: they drive its tool host, not egui.

use egui::{Event, Modifiers, PointerButton, Pos2, RawInput};
use serde_json::Value;

/// Most clicks one `ui.click` may ask for (`count`).
pub const MAX_CLICKS: u64 = 256;

/// A validated `ui.click`.
#[derive(Clone, Debug, PartialEq)]
pub struct Click {
    pub at: Pos2,
    pub button: PointerButton,
    pub count: u64,
    pub modifiers: Modifiers,
}

impl Click {
    /// `{x, y, button?, count?, modifiers?}`; errors name the parameter.
    pub fn from_params(params: &Value) -> Result<Self, String> {
        // pc: crates/ui-egui/src/control/dispatch.rs ("ui.click"), params.rs:150
        todo!()
    }
}

/// A validated `ui.key`: an egui key name (`Enter`, `A`, `F5`) with modifiers.
#[derive(Clone, Debug, PartialEq)]
pub struct KeyPress {
    pub key: egui::Key,
    pub modifiers: Modifiers,
}

impl KeyPress {
    /// `{key, command?, shift?, alt?, ctrl?}`.
    pub fn from_params(params: &Value) -> Result<Self, String> {
        // pc: crates/ui-egui/src/control/dispatch.rs ("ui.key"), params.rs:153
        todo!()
    }
}

/// Queued event batches, one per pass.
#[derive(Debug, Default)]
pub struct SyntheticInput {
    batches: std::collections::VecDeque<Vec<Event>>,
}

impl SyntheticInput {
    /// Move, press and release `count` times at `at`.
    pub fn click(&mut self, click: &Click) {
        // pc: crates/ui-egui/src/control/dispatch.rs ("ui.click")
        todo!()
    }

    /// A pointer move to `at`.
    pub fn move_to(&mut self, at: Pos2) {
        // pc: crates/ui-egui/src/control/dispatch.rs ("ui.move")
        todo!()
    }

    /// Press and release the key, with the modifiers held across both.
    pub fn key(&mut self, key: &KeyPress) {
        // pc: crates/ui-egui/src/control/dispatch.rs ("ui.key")
        todo!()
    }

    /// `Event::Text` for `text`.
    pub fn type_text(&mut self, text: &str) {
        // pc: crates/ui-egui/src/control/dispatch.rs ("ui.type")
        todo!()
    }

    /// Append the next batch to `raw`; true while batches remain after it (the app keeps
    /// repainting until false, then settles the `AfterInput` waiters).
    pub fn inject(&mut self, raw: &mut RawInput) -> bool {
        // pc: crates/ui-egui/src/app.rs (synthetic input appended in the raw input hook)
        todo!()
    }

    pub fn is_empty(&self) -> bool {
        self.batches.is_empty()
    }
}

#[cfg(test)]
#[path = "synthetic_tests.rs"]
mod synthetic_tests;
