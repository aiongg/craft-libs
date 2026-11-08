//! The input filter: the one owner of edits to the raw input a viewport's pass reads.
//!
//! A family crate hands the filter what it learned outside the window (a key equivalent the platform's menu
//! took) and the filter puts it into the viewport's input in the hook, ahead of the stages that read keys, so it
//! reaches the application as the key press the user made. No family crate touches `RawInput` or `egui::Event`.

use std::collections::HashMap;

use craft_keymap::Chord;
use egui::{Context, Event, ViewportId};

use super::injected::{egui_modifiers, key_press, key_release};
use super::memory::Service;
use super::repaint;

/// A key equivalent the platform's menu took from the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeyEquivalent {
    /// The press of `chord`. The platform delivers the key-up itself.
    Press(Chord),
    /// A press and its release of `chord`: a click that stands for the key, so no key-up follows.
    Tap(Chord),
    /// The platform's copy command: egui's copy event.
    Copy,
    /// The platform's cut command: egui's cut event.
    Cut,
}

#[derive(Default)]
struct Pending {
    by_viewport: HashMap<ViewportId, Vec<KeyEquivalent>>,
}

/// Queues `equivalent` for the next input of `viewport` and asks for a pass of it.
pub(crate) fn add_key_equivalent(ctx: &Context, viewport: ViewportId, equivalent: KeyEquivalent) {
    Service::<Pending>::run(ctx, |pending| pending.by_viewport.entry(viewport).or_default().push(equivalent));
    repaint::request_of(ctx, viewport);
}

/// The input hook's filter step: adds the queued key equivalents of `input.viewport_id` to `input.events`.
pub(crate) fn input_hook(ctx: &Context, input: &mut egui::RawInput) {
    let queued = Service::<Pending>::run(ctx, |pending| pending.by_viewport.remove(&input.viewport_id));
    let Some(queued) = queued else { return };
    let platform = craft_keymap::egui::platform(ctx);
    for equivalent in queued {
        match equivalent {
            KeyEquivalent::Copy => input.events.push(Event::Copy),
            KeyEquivalent::Cut => input.events.push(Event::Cut),
            KeyEquivalent::Press(chord) => insert_press(input, chord, platform, false),
            KeyEquivalent::Tap(chord) => insert_press(input, chord, platform, true),
        }
    }
}

/// Puts the press of `chord` ahead of its key-up when that is already in the input, else last; `tap` adds the
/// release right after it.
fn insert_press(input: &mut egui::RawInput, chord: Chord, platform: craft_keymap::Platform, tap: bool) {
    let (key, modifiers) = (chord.key.to_egui(), egui_modifiers(chord.mods, platform));
    let at = input.events.iter().position(|e| matches!(e, Event::Key { key: up, pressed: false, .. } if *up == key)).unwrap_or(input.events.len());
    input.events.insert(at, key_press(key, modifiers));
    if tap {
        input.events.insert(at + 1, key_release(key, modifiers));
    }
}
