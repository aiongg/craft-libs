//! The event sequences of synthetic input, one frame's events per entry.
//!
//! A platform delivers a pointer move, a press, a release and a key's down and up in separate
//! frames, with the modifier state announced before the first and cleared after the last. These
//! builders are the one definition of that sequence: `HarnessExt` plays it
//! on a kittest harness and `craft_automation::synthetic::SyntheticInput` plays it into a running
//! app. Keys carry their `physical_key`, which the shortcut dispatcher's position fallback reads.

use egui::{Event, Key, Modifiers, PointerButton, Pos2};

/// The events egui receives in one frame.
pub type Frame = Vec<Event>;

/// Pointer moves a drag makes between press and release.
// designcraft: crates/ui-egui/src/panel_docking/tests.rs:79
pub const DRAG_STEPS: usize = 8;

fn modifiers_changed(modifiers: Modifiers) -> Option<Event> {
    (modifiers != Modifiers::NONE).then_some(Event::ModifiersChanged(modifiers))
}

fn with_modifiers(modifiers: Modifiers, events: impl IntoIterator<Item = Event>) -> Frame {
    modifiers_changed(modifiers).into_iter().chain(events).collect()
}

fn modifiers_cleared(modifiers: Modifiers) -> Option<Frame> {
    modifiers_changed(modifiers).map(|_| vec![Event::ModifiersChanged(Modifiers::NONE)])
}

fn button(pos: Pos2, button: PointerButton, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton { pos, button, pressed, modifiers }
}

/// A move to `pos` (with `modifiers` announced first), then `count` times a press and a release
/// of `button`, one frame each; `modifiers` are cleared in a last frame when they were set.
pub fn click(pos: Pos2, button_kind: PointerButton, count: u64, modifiers: Modifiers) -> Vec<Frame> {
    let mut frames = vec![with_modifiers(modifiers, [Event::PointerMoved(pos)])];
    for _ in 0..count {
        frames.push(vec![button(pos, button_kind, true, modifiers)]);
        frames.push(vec![button(pos, button_kind, false, modifiers)]);
    }
    frames.extend(modifiers_cleared(modifiers));
    frames
}

/// A primary drag: a move to `from`, the press, [`DRAG_STEPS`] evenly spaced moves, and the
/// release at `to` together with the cleared modifiers.
pub fn drag(from: Pos2, to: Pos2, modifiers: Modifiers) -> Vec<Frame> {
    let primary = PointerButton::Primary;
    let mut frames = vec![with_modifiers(modifiers, [Event::PointerMoved(from)]), vec![button(from, primary, true, modifiers)]];
    for step in 1..=DRAG_STEPS {
        frames.push(vec![Event::PointerMoved(from + (to - from) * (step as f32 / DRAG_STEPS as f32))]);
    }
    let mut release = vec![button(to, primary, false, modifiers)];
    release.extend(modifiers_cleared(modifiers).into_iter().flatten());
    frames.push(release);
    frames
}

/// `key` down with `modifiers` announced in one frame, up (and the modifiers cleared) in the next.
pub fn key_press(key: Key, modifiers: Modifiers) -> Vec<Frame> {
    let event = |pressed| Event::Key { key, physical_key: Some(key), pressed, repeat: false, modifiers };
    let mut up = vec![event(false)];
    up.extend(modifiers_cleared(modifiers).into_iter().flatten());
    vec![with_modifiers(modifiers, [event(true)]), up]
}

/// `text` as one `Event::Text` in one frame.
pub fn text(text: &str) -> Vec<Frame> {
    vec![vec![Event::Text(text.to_owned())]]
}
