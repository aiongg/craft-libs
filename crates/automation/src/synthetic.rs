//! Synthetic pointer and keyboard input: `ui.click`, `ui.move`, `ui.key`, `ui.type`.
//!
//! A request appends frames of egui events to one queue, built by
//! [`craft_ui::testing::events`] (the sequence the test harness plays too). Each pass the app's
//! raw input hook injects the next step ([`SyntheticInput::take_step`]: one frame), so a move, a
//! press, a release and a key's down and up land on separate passes as real input does. A step stays
//! in flight until a UI pass has handled it ([`SyntheticInput::pass_finished`]); a pass that
//! returned before handling input ([`SyntheticInput::pass_deferred`]) gets the same step again,
//! once. A step queued for one document or focus is withdrawn when either changed before the pass
//! consumed it, and the waiting replies fail. Screen points are egui points. Document-coordinate
//! `ui.pointer` events stay in the app: they drive its tool host, not egui.

use std::collections::VecDeque;

use craft_ui::testing::events;
use egui::{Event, Modifiers, PointerButton, Pos2, RawInput, pos2};
use serde_json::Value;

use crate::protocol::{BUTTONS, KEY_MODIFIERS, Params};

/// Most clicks one `ui.click` may ask for (`count`).
pub const MAX_CLICKS: u64 = 256;

/// The reason [`Injected::Withdrawn`] gives [`crate::control::ControlChannel::input_cancelled`].
pub const TARGET_CHANGED: &str = "the active document or focus changed while the input was queued";

/// `x` and `y` in screen points, each defaulting to 0.
fn coordinates(p: &Params<'_>) -> Result<(f32, f32), String> {
    let x = p.number_in("x", "x", None)?.unwrap_or(0.0) as f32;
    let y = p.number_in("y", "y", None)?.unwrap_or(0.0) as f32;
    Ok((x, y))
}

/// Remove one contiguous run equal to `step` from `events` (eframe's preserved synthetic input).
/// A no-op when the run is not present.
fn remove_event_run(events: &mut Vec<Event>, step: &[Event]) {
    if step.is_empty() || step.len() > events.len() {
        return;
    }
    if let Some(pos) = events.windows(step.len()).position(|w| w == step) {
        events.drain(pos..pos + step.len());
    }
}

/// A validated `ui.click`.
#[derive(Clone, Debug, PartialEq)]
pub struct Click {
    /// Target position in points.
    pub at: Pos2,
    /// The button pressed.
    pub button: PointerButton,
    /// Number of clicks (2 is a double click).
    pub count: u64,
}

impl Click {
    /// `{x?, y?, button?, count?}`, no other fields (`Params::only`). `x` and `y` are numbers,
    /// default 0. `button` is one of [`crate::protocol::BUTTONS`], default `left`; `right` and
    /// `secondary` are the secondary button, `middle` the middle one, the rest primary. `count`
    /// is an unsigned integer, default 1; over [`MAX_CLICKS`] is
    /// ``"`count` must be at most 256 (got {count})"``. Other errors are the `Params` validators'.
    pub fn from_params(params: &Value) -> Result<Self, String> {
        let p = Params(params).only(None, &["x", "y", "button", "count"])?;
        let p = Params(&p);
        let (x, y) = coordinates(&p)?;
        let button = match p.choice("button", "button", &BUTTONS)?.unwrap_or("left") {
            "right" | "secondary" => PointerButton::Secondary,
            "middle" => PointerButton::Middle,
            _ => PointerButton::Primary,
        };
        let count = p.uint_field("count")?.unwrap_or(1);
        if count > MAX_CLICKS {
            return Err(format!("`count` must be at most {MAX_CLICKS} (got {count})"));
        }
        Ok(Self { at: pos2(x, y), button, count })
    }
}

/// A validated `ui.move`: `{x?, y?}` only, numbers, default 0.
pub fn move_from_params(params: &Value) -> Result<Pos2, String> {
    let p = Params(params).only(None, &["x", "y"])?;
    let (x, y) = coordinates(&Params(&p))?;
    Ok(pos2(x, y))
}

/// A validated `ui.key`: an egui key name (`Enter`, `A`, `F5`) with modifiers.
#[derive(Clone, Debug, PartialEq)]
pub struct KeyPress {
    /// The key pressed.
    pub key: egui::Key,
    /// Modifiers held with it.
    pub modifiers: Modifiers,
}

impl KeyPress {
    /// `{key, modifiers?, shift?, alt?, command?, ctrl?}`, no other fields. The modifier flags may
    /// be top-level or grouped under `modifiers` ([`crate::protocol::KEY_MODIFIERS`]). A missing
    /// key is ``"missing `key`"``; a name `egui::Key::from_name` doesn't know is
    /// ``"unknown key `{name}`"``.
    pub fn from_params(params: &Value) -> Result<Self, String> {
        let p = Params(params).only(None, &["key", "modifiers", "shift", "alt", "command", "ctrl"])?;
        let p = Params(&p);
        let Some(name) = p.choice("key", "key", &[])? else { return Err("missing `key`".into()) };
        let key = egui::Key::from_name(name).ok_or_else(|| format!("unknown key `{name}`"))?;
        Ok(Self { key, modifiers: p.modifiers(&KEY_MODIFIERS)? })
    }
}

/// What [`SyntheticInput::inject`] did to this pass's input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Injected {
    /// Nothing queued, or a step is in flight and needs nothing this pass.
    Nothing,
    /// The next step was appended; the app records its document and focus for the next check.
    Step,
    /// The step in flight was appended again after a deferred pass.
    Again,
    /// The target changed: the step in flight was removed from the input (one contiguous run of
    /// events equal to it, if present) and the queue cleared. The app calls
    /// `ControlChannel::input_cancelled(TARGET_CHANGED)`.
    Withdrawn,
}

/// Queued synthetic events and the step in flight.
#[derive(Debug, Default)]
pub struct SyntheticInput {
    queue: VecDeque<Vec<Event>>,
    in_flight: Option<Vec<Event>>,
    again: bool,
}

impl SyntheticInput {
    /// A move to `at`, then `count` times a press and a release of `button`
    /// ([`events::click`]), one frame each, with default modifiers.
    pub fn click(&mut self, click: &Click) {
        let Click { at, button, count } = *click;
        self.queue.extend(events::click(at, button, count, Modifiers::NONE));
    }

    /// `PointerMoved(at)`.
    pub fn move_to(&mut self, at: Pos2) {
        self.queue.push_back(vec![Event::PointerMoved(at)]);
    }

    /// `key` pressed, then released ([`events::key_press`]): the modifiers announced with the
    /// press and cleared with the release, `physical_key` set to the key.
    pub fn key(&mut self, key: &KeyPress) {
        self.queue.extend(events::key_press(key.key, key.modifiers));
    }

    /// `Event::Text(text)` ([`events::text`]). The app checks `text` is given first (``"missing `text`"``).
    pub fn type_text(&mut self, text: &str) {
        self.queue.extend(events::text(text));
    }

    /// The next step: the next queued frame's events, empty when nothing is queued.
    pub fn take_step(&mut self) -> Vec<Event> {
        self.queue.pop_front().unwrap_or_default()
    }

    /// The raw input hook. With a step in flight: `target_matches` false withdraws it
    /// ([`Injected::Withdrawn`]); after [`Self::pass_deferred`] it is appended again once
    /// ([`Injected::Again`]); otherwise nothing. With none in flight, the next step is appended
    /// and is the one in flight ([`Injected::Step`]). The app computes `target_matches` by
    /// comparing the document and focus it recorded at the last `Step` with now.
    pub fn inject(&mut self, raw: &mut RawInput, target_matches: bool) -> Injected {
        if let Some(step) = &self.in_flight {
            if !target_matches {
                remove_event_run(&mut raw.events, step);
                self.cancel();
                return Injected::Withdrawn;
            }
            if self.again {
                raw.events.extend(step.iter().cloned());
                self.again = false;
                return Injected::Again;
            }
            return Injected::Nothing;
        }
        let step = self.take_step();
        if step.is_empty() {
            return Injected::Nothing;
        }
        raw.events.extend(step.iter().cloned());
        self.in_flight = Some(step);
        Injected::Step
    }

    /// A UI pass handled the step in flight: drop it and let the next step in.
    pub fn pass_finished(&mut self) {
        self.in_flight = None;
        self.again = false;
    }

    /// The pass returned before handling input (fonts not ready): egui already consumed the
    /// events, so the next [`Self::inject`] appends the step again, once.
    pub fn pass_deferred(&mut self) {
        self.again = self.in_flight.is_some();
    }

    /// Drop the queue and the step in flight; true when anything was pending (the app calls
    /// `ControlChannel::cancel_input`, which also fails waiters when nothing was queued).
    pub fn cancel(&mut self) -> bool {
        let pending = self.in_flight.take().is_some() || !self.queue.is_empty();
        self.queue.clear();
        self.again = false;
        pending
    }

    /// Nothing queued and nothing in flight: the app's `AfterInput` waiters may be answered
    /// (`ControlChannel::input_processed`). While false the app keeps requesting repaints.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty() && self.in_flight.is_none()
    }
}

#[cfg(test)]
#[path = "synthetic_tests.rs"]
mod synthetic_tests;
