//! egui events to rt platform events: pointer events while egui says the panel is hovered or it
//! holds a press, wheel steps while hovered, and keys, text, paste and IME while the panel has
//! egui's key focus.

use std::ops::Range;

use craft_keymap::chord::{Key, Mods, Platform};
use craft_ui_rt::{
    KeyInput, PlatformEvent, PointerButton, PointerEvent, PointerKind, PointerPhase, Pos2, WheelPhase, WheelSource, WheelUnit, WindowEvent, WindowId,
};
use egui::{Event, InputState, MouseWheelSource, MouseWheelUnit, TouchPhase};

/// The mouse's pointer id, as the winit platform numbers it.
const MOUSE: u64 = 0;

/// Whether pointer events reach the rt window, kept across passes.
#[derive(Default)]
pub(crate) struct PointerGate {
    /// A press went down in the panel and is not released yet: the panel gets the pointer
    /// wherever it goes (egui's press capture keeps the drag on the panel's widget).
    pressed: bool,
    /// The rt window saw the pointer inside the panel since its last Leave.
    inside: bool,
}

/// What [`forward`] passes on in one pass.
pub(crate) struct Gate {
    pub window: WindowId,
    /// egui's hover answer for the panel: layers above it and other widgets' drags respected.
    pub hovered: bool,
    /// The panel has egui's key focus.
    pub focused: bool,
    pub platform: Platform,
}

/// What [`forward`] sent, for the bridge's work after the step.
#[derive(Default)]
pub(crate) struct Forwarded {
    pub events: u32,
    /// The key events sent, with their index in egui's event list, in order.
    pub keys: Vec<(usize, KeyInput)>,
    /// A wheel step went to the panel: no parent scrolls by it.
    pub wheel: bool,
}

impl PointerGate {
    /// Whether the rt window gets pointer events this pass.
    pub fn open(&self, hovered: bool) -> bool {
        hovered || self.pressed
    }

    /// The rt window holds a press: egui keeps the pointer on the panel.
    pub fn captured(&self) -> bool {
        self.pressed
    }
}

/// Moves this pass's egui events for the panel into `out`, in order, with their own positions.
pub(crate) fn forward(i: &InputState, gate: &Gate, pointer: &mut PointerGate, out: &mut Vec<PlatformEvent>) -> Forwarded {
    let mut sent = Forwarded::default();
    let mods = Mods::from_egui(&i.modifiers, gate.platform);
    let window = gate.window;
    let push = |out: &mut Vec<PlatformEvent>, ev: WindowEvent| {
        out.push(PlatformEvent::Window(window, ev));
    };
    let pointer_event = |phase: PointerPhase, button: PointerButton, pos: Pos2, mods: Mods| {
        WindowEvent::Pointer(PointerEvent::new(MOUSE, PointerKind::Mouse, phase, button, pos, mods))
    };
    for (index, event) in i.events.iter().enumerate() {
        let before = out.len();
        match event {
            Event::PointerMoved(pos) if pointer.open(gate.hovered) => {
                pointer.inside = true;
                push(out, pointer_event(PointerPhase::Move, PointerButton::Primary, *pos, mods));
            }
            Event::PointerButton { pos, button, pressed, modifiers } => {
                let button = button_of(*button);
                let mods = Mods::from_egui(modifiers, gate.platform);
                if *pressed && gate.hovered {
                    pointer.pressed = true;
                    pointer.inside = true;
                    push(out, pointer_event(PointerPhase::Down, button, *pos, mods));
                } else if !*pressed && pointer.pressed {
                    pointer.pressed = false;
                    push(out, pointer_event(PointerPhase::Up, button, *pos, mods));
                }
            }
            Event::PointerGone if pointer.inside => {
                pointer.inside = false;
                if pointer.pressed {
                    pointer.pressed = false;
                    push(out, pointer_event(PointerPhase::Cancel, PointerButton::Primary, i.pointer.latest_pos().unwrap_or_default(), mods));
                }
                push(out, pointer_event(PointerPhase::Leave, PointerButton::Primary, Pos2::ZERO, mods));
            }
            Event::MouseWheel { unit, delta, phase, source, modifiers } if gate.hovered => {
                let Some(pos) = i.pointer.latest_pos() else { continue };
                sent.wheel = true;
                push(
                    out,
                    WindowEvent::Wheel {
                        pos,
                        delta: *delta,
                        unit: unit_of(*unit),
                        phase: phase_of(*phase),
                        source: source_of(*source),
                        mods: Mods::from_egui(modifiers, gate.platform),
                    },
                );
            }
            Event::Key { key, pressed, repeat, modifiers, .. } if gate.focused => {
                let Some(key) = Key::from_egui(*key) else { continue };
                let k = KeyInput { key, mods: Mods::from_egui(modifiers, gate.platform), down: *pressed, repeat: *repeat };
                sent.keys.push((index, k));
                push(out, WindowEvent::Key { key: k.key, mods: k.mods, down: k.down, repeat: k.repeat });
            }
            Event::Text(text) if gate.focused => push(out, WindowEvent::Text(text.clone())),
            Event::Paste(text) if gate.focused => push(out, WindowEvent::Paste(text.clone())),
            Event::Ime(egui::ImeEvent::Preedit { text, active_range_chars }) if gate.focused => {
                let cursor = active_range_chars.clone().map(|r| byte_range(text, r));
                push(out, WindowEvent::ImePreedit { text: text.clone(), cursor });
            }
            Event::Ime(egui::ImeEvent::Commit(text)) if gate.focused => push(out, WindowEvent::ImeCommit(text.clone())),
            _ => {}
        }
        sent.events += u32::try_from(out.len() - before).unwrap_or(0);
    }
    if !gate.hovered && !pointer.pressed && pointer.inside {
        // The pointer left the panel (or another layer or widget took it) since the last pass.
        pointer.inside = false;
        push(out, pointer_event(PointerPhase::Leave, PointerButton::Primary, Pos2::ZERO, mods));
        sent.events += 1;
    }
    sent
}

/// The indices of the keys in `sent` that the rt window took: every one but those it handed back
/// as unhandled, matched in order.
pub(crate) fn claimed(sent: &[(usize, KeyInput)], unhandled: &[KeyInput]) -> Vec<usize> {
    let mut back = unhandled.iter().peekable();
    let mut taken = Vec::new();
    for (index, k) in sent {
        if back.peek().is_some_and(|u| *u == k) {
            back.next();
        } else {
            taken.push(*index);
        }
    }
    taken
}

fn byte_range(text: &str, chars: Range<usize>) -> Range<usize> {
    let byte = |c: usize| text.char_indices().nth(c).map_or(text.len(), |(b, _)| b);
    byte(chars.start)..byte(chars.end)
}

fn button_of(button: egui::PointerButton) -> PointerButton {
    match button {
        egui::PointerButton::Primary => PointerButton::Primary,
        egui::PointerButton::Secondary => PointerButton::Secondary,
        egui::PointerButton::Middle => PointerButton::Middle,
        egui::PointerButton::Extra1 => PointerButton::Extra1,
        egui::PointerButton::Extra2 => PointerButton::Extra2,
    }
}

fn unit_of(unit: MouseWheelUnit) -> WheelUnit {
    match unit {
        MouseWheelUnit::Point => WheelUnit::Point,
        MouseWheelUnit::Line => WheelUnit::Line,
        MouseWheelUnit::Page => WheelUnit::Page,
    }
}

fn phase_of(phase: TouchPhase) -> WheelPhase {
    match phase {
        TouchPhase::Start => WheelPhase::Start,
        TouchPhase::Move => WheelPhase::Move,
        TouchPhase::End => WheelPhase::End,
        TouchPhase::Cancel => WheelPhase::Cancel,
    }
}

fn source_of(source: MouseWheelSource) -> WheelSource {
    match source {
        MouseWheelSource::Unknown => WheelSource::Unknown,
        MouseWheelSource::Wheel => WheelSource::Wheel,
        MouseWheelSource::Trackpad => WheelSource::Trackpad,
        MouseWheelSource::Momentum => WheelSource::Momentum,
    }
}
