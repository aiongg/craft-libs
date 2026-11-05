//! Tests of the blend mode keys.

use egui::{Context, Event, Id, Key, Modifiers, RawInput};

use super::blend_keys::{BlendKeyEntry, BlendKeyPick, BlendKeys, BlendShortcut};
use craft_ui::focus::key_owner::OwnerGate;

const SHIFT: Modifiers = Modifiers { shift: true, ..Modifiers::NONE };
const SHIFT_ALT: Modifiers = Modifiers { shift: true, alt: true, ..Modifiers::NONE };

type Pick = Option<BlendKeyPick<&'static str>>;

fn key_event(key: Key, physical_key: Option<Key>, modifiers: Modifiers) -> Event {
    Event::Key { key, physical_key, pressed: true, repeat: false, modifiers }
}

/// A short Blend Mode menu, by name.
fn modes() -> Vec<BlendKeyEntry<&'static str>> {
    vec![
        BlendKeyEntry::new("normal", BlendShortcut::Normal),
        BlendKeyEntry::new("multiply", BlendShortcut::Multiply),
        BlendKeyEntry::new("screen", BlendShortcut::Screen),
        BlendKeyEntry::new("pin", BlendShortcut::PinLight),
        BlendKeyEntry::unlettered("subtract"),
    ]
}

/// One pass on `ctx` that reads `events` before `after` draws; returns the pick and the number of
/// key presses still in the input.
fn read_in(
    ctx: &Context,
    events: Vec<Event>,
    current: Option<&'static str>,
    gate: OwnerGate,
    entries: &[BlendKeyEntry<&'static str>],
    after: impl Fn(&mut egui::Ui),
) -> (Pick, usize) {
    let mut out = (None, 0);
    let raw = RawInput { events, ..RawInput::default() };
    ctx.run_ui(raw, |ui| {
        let keys = BlendKeys::new(entries).gate(gate);
        let pick = match current.as_ref() {
            Some(c) => keys.current(c).consume(ui.ctx()),
            None => keys.consume(ui.ctx()),
        };
        let left = ui.ctx().input(|i| i.events.iter().filter(|e| matches!(e, Event::Key { .. })).count());
        out = (pick, left);
        after(ui);
    })
    .drop_without_applying_deltas();
    out
}

fn read(events: Vec<Event>, current: Option<&'static str>, entries: &[BlendKeyEntry<&'static str>]) -> (Pick, usize) {
    read_in(&Context::default(), events, current, OwnerGate::default(), entries, |_| {})
}

// ⇧+ from the last entry wraps to the first and ⇧− from the first to the last; an unlisted
// current steps from the top (+) or the bottom (−); ⇧= counts as ⇧+.
#[test]
fn shift_plus_and_minus_step_and_wrap() {
    let entries = modes();
    let plus = || vec![key_event(Key::Plus, None, SHIFT)];
    let minus = || vec![key_event(Key::Minus, None, SHIFT)];
    assert_eq!(read(plus(), Some("subtract"), &entries).0, Some(BlendKeyPick::Stepped("normal")));
    assert_eq!(read(minus(), Some("normal"), &entries).0, Some(BlendKeyPick::Stepped("subtract")));
    assert_eq!(read(plus(), Some("unlisted"), &entries).0, Some(BlendKeyPick::Stepped("normal")));
    assert_eq!(read(minus(), None, &entries).0, Some(BlendKeyPick::Stepped("subtract")));
    assert_eq!(read(vec![key_event(Key::Equals, None, SHIFT)], Some("normal"), &entries).0, Some(BlendKeyPick::Stepped("multiply")));
}

// ⇧⌥M picks the Multiply entry as `BlendKeyPick::Lettered`; the event is gone from the input.
#[test]
fn shift_alt_letter_picks_the_listed_mode() {
    let (pick, left) = read(vec![key_event(Key::M, None, SHIFT_ALT)], Some("normal"), &modes());
    assert_eq!(pick, Some(BlendKeyPick::Lettered("multiply")));
    assert_eq!(pick.map(BlendKeyPick::into_key), Some("multiply"));
    assert_eq!(left, 0);
}

// Every `BlendShortcut` maps to its own letter and back (`letter`, `from_key`), 26 distinct keys.
#[test]
fn every_shortcut_has_its_own_letter() {
    let mut letters = std::collections::HashSet::new();
    for shortcut in BlendShortcut::ALL {
        assert_eq!(BlendShortcut::from_key(shortcut.letter()), Some(shortcut));
        letters.insert(shortcut.letter());
    }
    assert_eq!(letters.len(), 26);
    assert_eq!(BlendShortcut::from_key(Key::Num1), None);
}

// ⇧⌥P with no Pass Through entry: consumed (later readers don't see it) and no pick; ⇧⌥1 is not
// a table letter and stays in the input.
#[test]
fn a_letter_for_a_missing_mode_is_consumed_and_does_nothing() {
    let entries = modes();
    assert_eq!(read(vec![key_event(Key::P, None, SHIFT_ALT)], Some("normal"), &entries), (None, 0));
    assert_eq!(read(vec![key_event(Key::Num1, None, SHIFT_ALT)], Some("normal"), &entries), (None, 1));
}

// key Â with physical key M picks Multiply; key Z with physical key Y picks Pin Light (the
// layout's letter wins).
#[test]
fn physical_key_is_used_when_the_layout_letter_is_not_a_mode() {
    let entries = modes();
    let mac = key_event(Key::Num1, Some(Key::M), SHIFT_ALT);
    assert_eq!(read(vec![mac], None, &entries).0, Some(BlendKeyPick::Lettered("multiply")));
    let german = key_event(Key::Z, Some(Key::Y), SHIFT_ALT);
    assert_eq!(read(vec![german], None, &entries).0, Some(BlendKeyPick::Lettered("pin")));
}

// With a focused text field (and with an open menu) the presses stay in the input and nothing is
// picked; with ⌘ or Ctrl held nothing is read either.
#[test]
fn keys_are_ignored_when_a_text_field_has_focus() {
    let entries = modes();
    let ctx = Context::default();
    let id = Id::unique("field");
    let field = |ui: &mut egui::Ui| {
        let mut text = String::new();
        ui.add(egui::TextEdit::singleline(&mut text).id(id)).request_focus();
    };
    for _ in 0..2 {
        read_in(&ctx, vec![], None, OwnerGate::default(), &entries, field);
    }
    assert!(ctx.text_edit_focused());
    let m = || vec![key_event(Key::M, None, SHIFT_ALT)];
    assert_eq!(read_in(&ctx, m(), None, OwnerGate::default(), &entries, field), (None, 1));

    // A dialog the app draws itself takes the keyboard the same way.
    let dialog = OwnerGate { dialog: true, ..OwnerGate::default() };
    assert_eq!(read_in(&Context::default(), m(), None, dialog, &entries, |_| {}), (None, 1));

    for modifiers in [Modifiers { command: true, ..SHIFT_ALT }, Modifiers { ctrl: true, ..SHIFT_ALT }] {
        assert_eq!(read(vec![key_event(Key::M, None, modifiers)], None, &entries), (None, 1));
    }
}

// ⇧⌥Q with a `BlendKeyEntry::behind` entry picks it; without one it is consumed and picks nothing.
#[test]
fn behind_is_consumed_and_reported() {
    let q = || vec![key_event(Key::Q, None, SHIFT_ALT)];
    assert_eq!(read(q(), None, &modes()), (None, 0));
    let mut entries = modes();
    entries.insert(1, BlendKeyEntry::behind("behind"));
    assert_eq!(read(q(), None, &entries), (Some(BlendKeyPick::Lettered("behind")), 0));
}

// Two ⇧+ presses in one pass step twice from the current entry and report one pick.
#[test]
fn presses_in_one_pass_combine_into_one_pick() {
    let events = vec![key_event(Key::Plus, None, SHIFT), key_event(Key::Plus, None, SHIFT)];
    assert_eq!(read(events, Some("normal"), &modes()), (Some(BlendKeyPick::Stepped("screen")), 0));
}
