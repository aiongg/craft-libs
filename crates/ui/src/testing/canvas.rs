//! Pointer and key input for a tool under test: the sequence a platform sends a canvas, played on
//! any kittest harness whose UI shows a [`ToolHost`](crate::canvas::ToolHost). Available with the
//! `screen-testing` feature (desktop) and in craft-ui's own tests.
//!
//! Every function works on `&mut Harness<'_, S>` for any state `S`, so a test keeps its own
//! state and UI. Each sends real `Event::PointerMoved`, `Event::PointerButton`,
//! `Event::ModifiersChanged` and `Event::Key` events, one frame per step, so the host reads
//! `PointerState` the way it does in an app (a press, drags and a release arrive in separate
//! passes). The pointer reaches a tool as [`ToolPointer`](crate::canvas::ToolPointer)s in
//! document units through the canvas's view; nothing is injected through
//! [`ToolHost::inject`](crate::canvas::ToolHost::inject), so tests of `takes`, click runs and
//! key handling exercise the OS path.
//!
//! Modifiers: every call that takes `modifiers` announces exactly that state in the frame of its
//! first event, so a call with [`Modifiers::NONE`] also releases keys an earlier call held.
//! [`release`] and [`key`] clear them again when they finish; [`press`] keeps them held for the
//! drag that follows.
//!
//! ```ignore
//! use craft_ui::testing::canvas;
//!
//! canvas::press(&mut h, pos2(100.0, 100.0), Modifiers::NONE);
//! canvas::move_to(&mut h, pos2(140.0, 120.0));
//! canvas::release(&mut h, pos2(140.0, 120.0), Modifiers::NONE);
//! canvas::drag(&mut h, pos2(10.0, 10.0), pos2(90.0, 50.0), Modifiers::ALT);
//! canvas::key(&mut h, Key::Escape, Modifiers::NONE);
//! ```

use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;

use super::events::{DRAG_STEPS, Frame};

/// Run one frame with `events`.
fn frame<S>(h: &mut Harness<'_, S>, events: Frame) {
    for event in events {
        h.event(event);
    }
    h.step();
}

fn button(at: Pos2, button: PointerButton, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton { pos: at, button, pressed, modifiers }
}

/// Move the pointer to `at` (screen points) and run a frame. Hovers the canvas when it is over it.
pub fn move_to<S>(h: &mut Harness<'_, S>, at: Pos2) {
    frame(h, vec![Event::PointerMoved(at)]);
}

/// Announce `modifiers` as the keys now held and run a frame (a key going up mid-gesture).
pub fn set_modifiers<S>(h: &mut Harness<'_, S>, modifiers: Modifiers) {
    frame(h, vec![Event::ModifiersChanged(modifiers)]);
}

/// Move to `at`, then press the primary button with `modifiers` held, one frame each.
pub fn press<S>(h: &mut Harness<'_, S>, at: Pos2, modifiers: Modifiers) {
    press_button(h, at, PointerButton::Primary, modifiers);
}

/// [`press`] with another button, for tools that take a secondary press ([`takes`](crate::canvas::CanvasTool::takes)).
pub fn press_button<S>(h: &mut Harness<'_, S>, at: Pos2, which: PointerButton, modifiers: Modifiers) {
    move_to(h, at);
    frame(h, vec![Event::ModifiersChanged(modifiers), button(at, which, true, modifiers)]);
}

/// Release the primary button at `at` and clear the modifiers, in one frame.
pub fn release<S>(h: &mut Harness<'_, S>, at: Pos2, modifiers: Modifiers) {
    release_button(h, at, PointerButton::Primary, modifiers);
}

/// [`release`] for another button.
pub fn release_button<S>(h: &mut Harness<'_, S>, at: Pos2, which: PointerButton, modifiers: Modifiers) {
    frame(h, vec![button(at, which, false, modifiers), Event::ModifiersChanged(Modifiers::NONE)]);
}

/// A primary drag: [`press`] at `from`, [`DRAG_STEPS`] evenly spaced moves, [`release`] at `to`.
/// Then 1 more frame, so a tool that reacts one pass after the release has run.
pub fn drag<S>(h: &mut Harness<'_, S>, from: Pos2, to: Pos2, modifiers: Modifiers) {
    press(h, from, modifiers);
    for step in 1..=DRAG_STEPS {
        move_to(h, from + (to - from) * (step as f32 / DRAG_STEPS as f32));
    }
    release(h, to, modifiers);
    h.step();
}

/// `key` down with `modifiers` held in one frame, up with the modifiers cleared in the next. The
/// canvas must have the keyboard (`Response::request_focus` in the UI).
pub fn key<S>(h: &mut Harness<'_, S>, key: Key, modifiers: Modifiers) {
    let event = |pressed| Event::Key { key, physical_key: Some(key), pressed, repeat: false, modifiers };
    frame(h, vec![Event::ModifiersChanged(modifiers), event(true)]);
    frame(h, vec![event(false), Event::ModifiersChanged(Modifiers::NONE)]);
}

/// The pointer's button goes up while the host is not looking: a pass in which nothing is shown
/// receives the release, then the harness runs a frame. The host finds a gesture whose button is
/// neither down nor released in its pass, and cancels it with
/// [`CancelReason::FocusLost`](crate::edit::CancelReason::FocusLost). egui reports no other way
/// for a button to end without a release.
pub fn lose_pointer<S>(h: &mut Harness<'_, S>, at: Pos2, which: PointerButton) {
    let events = vec![button(at, which, false, Modifiers::NONE)];
    let mut out = h.ctx.run_ui(egui::RawInput { events, ..egui::RawInput::default() }, |_| {});
    out.textures_delta.clear();
    h.step();
}
