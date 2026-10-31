#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! Real input on an egui_kittest harness (`screen-testing` feature, desktop): clicks, drags,
//! wheel turns, key presses and typing that arrive one event per frame as a platform sends them,
//! and pumps that wait for the app's background work.
//!
//! kittest's own `hover_at`/`drag_at`/`drop_at` queue events without running frames, so a press
//! and its release can land in one pass and a widget that reacts on the press frame never sees
//! it. [`HarnessExt`] runs a frame after each event. Its pumps sleep 1 ms between frames, so
//! worker threads (a render, an import) get the CPU, and a timeout ends a wait that would hang.
//!
//! ```ignore
//! use craft_ui::testing::HarnessExt as _;
//!
//! harness.click_at(row.center());
//! harness.drag(tab.center(), pos2(160.0, 140.0), Modifiers::NONE);
//! harness.press(KeyboardShortcut::new(Modifiers::COMMAND, Key::Z));
//! harness.type_text("Layer 2");
//! assert!(harness.pump_idle(Duration::from_secs(10), |h| h.state().busy()), "still busy");
//! ```

use std::time::Duration;

use egui::{KeyboardShortcut, Modifiers, PointerButton, Pos2, Vec2};
use egui_kittest::Harness;

/// Pointer moves a [`HarnessExt::drag`] makes between press and release.
// designcraft: crates/ui-egui/src/panel_docking/tests.rs:79
pub const DRAG_STEPS: usize = 8;
/// Frames in a row without work before [`HarnessExt::pump_idle`] calls the app idle, so layout and
/// short UI animations finish.
// lightcraft: crates/ui-egui/src/headless.rs:423
pub const QUIET_FRAMES: usize = 12;
/// Sleep between pumped frames.
// lightcraft: crates/ui-egui/src/headless.rs:418
pub const PUMP_SLEEP: Duration = Duration::from_millis(1);

/// Input and waiting helpers on [`Harness`]. Implemented for every harness; import it with
/// `use craft_ui::testing::HarnessExt as _`.
pub trait HarnessExt {
    /// A primary click at `pos`: move there, press, release, one frame each, then 3 frames.
    fn click_at(&mut self, pos: Pos2);

    /// A click of `button` at `pos` with `modifiers` held throughout: modifiers changed, move
    /// (1 frame), press (1 frame), release (1 frame), modifiers released, 3 frames.
    fn click_at_with(&mut self, pos: Pos2, button: PointerButton, modifiers: Modifiers);

    /// A primary drag from `from` to `to` with `modifiers` held: move to `from` (1 frame), press
    /// (1 frame), [`DRAG_STEPS`] evenly spaced moves (1 frame each), release at `to`, 3 frames.
    fn drag(&mut self, from: Pos2, to: Pos2, modifiers: Modifiers);

    /// The mouse wheel turned over `pos` by `delta` points (`MouseWheelUnit::Point`, phase
    /// `Move`): positive `delta` moves the content right and down. Then 10 frames.
    fn wheel_at(&mut self, pos: Pos2, delta: Vec2);

    /// `shortcut` pressed and released: the key down with its modifiers on one frame, up on the
    /// next (`physical_key` is the logical key), then 2 frames.
    fn press(&mut self, shortcut: KeyboardShortcut);

    /// `text` typed into the focused widget as one `Event::Text`, then 2 frames. For an input
    /// method's composition use [`compose`](super::compose).
    fn type_text(&mut self, text: &str);

    /// Run frames until `done` holds, sleeping [`PUMP_SLEEP`] between them; `false` when
    /// `timeout` passes first. Checks `done` before the first frame.
    fn pump_until(&mut self, timeout: Duration, done: impl FnMut(&Self) -> bool) -> bool;

    /// Run frames until `busy` has been false for [`QUIET_FRAMES`] frames in a row (sleeping
    /// [`PUMP_SLEEP`] after each busy one); `false` when `timeout` passes first.
    fn pump_idle(&mut self, timeout: Duration, busy: impl FnMut(&Self) -> bool) -> bool;
}

impl<State> HarnessExt for Harness<'_, State> {
    fn click_at(&mut self, pos: Pos2) {
        // designcraft: crates/ui-egui/src/lib.rs:399, as click_at_with(pos, Primary, NONE)
        todo!()
    }

    fn click_at_with(&mut self, pos: Pos2, button: PointerButton, modifiers: Modifiers) {
        // pc: crates/ui-egui/src/panels/layer_tree_ui/tests.rs:61
        todo!()
    }

    fn drag(&mut self, from: Pos2, to: Pos2, modifiers: Modifiers) {
        // designcraft: crates/ui-egui/src/panel_docking/tests.rs:74
        todo!()
    }

    fn wheel_at(&mut self, pos: Pos2, delta: Vec2) {
        // designcraft: crates/ui-egui/src/lib.rs:363
        todo!()
    }

    fn press(&mut self, shortcut: KeyboardShortcut) {
        // new: egui-winit's down and up on separate frames, as the keymap tests send them
        todo!()
    }

    fn type_text(&mut self, text: &str) {
        // cadcraft: crates/ui-egui/tests/it/selection_cycling.rs:90 (without the Enter)
        todo!()
    }

    fn pump_until(&mut self, timeout: Duration, done: impl FnMut(&Self) -> bool) -> bool {
        // lightcraft: crates/ui-egui/src/headless.rs:434
        todo!()
    }

    fn pump_idle(&mut self, timeout: Duration, busy: impl FnMut(&Self) -> bool) -> bool {
        // lightcraft: crates/ui-egui/src/headless.rs:411
        todo!()
    }
}

#[cfg(test)]
#[path = "frame_tests.rs"]
mod frame_tests;
