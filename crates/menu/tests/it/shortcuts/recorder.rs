//! Recording through the dialog: the recorder takes every key, the state decides what a chord means.

use craft_keymap::Platform;
use craft_menu::shortcuts::Outcome;
use craft_ui::testing::trial;
use egui::{Event, Key, Modifiers};

use super::fixture::Rig;

/// While recording, Tab, an arrow and Shift+Escape are chords, not focus moves or a close.
#[trial]
fn recording_takes_tab_arrows_and_shift_escape() {
    let mut rig = Rig::tuned(Platform::Mac, |_| {});
    let mut current = "Save: ⌘S".to_owned();
    for (key, modifiers, expected, shown) in [
        (Key::Tab, Modifiers::NONE, "Tab", "⇥"),
        (Key::ArrowDown, Modifiers::NONE, "Down", "↓"),
        (Key::Escape, Modifiers::SHIFT, "Shift+Escape", "⇧⎋"),
    ] {
        rig.click(&current);
        rig.press(modifiers, key);
        assert_eq!(rig.bindings("file.save"), [expected]);
        current = format!("Save: {shown}");
        assert!(rig.has(&current));
    }
    assert!(rig.is_open(), "Shift+Escape did not close the dialog");
}

/// Escape stops the recording; a second Escape cancels the dialog.
#[trial]
fn escape_cancels_recording_only() {
    let mut rig = Rig::tuned(Platform::Mac, |_| {});
    rig.click("Save: ⌘S");
    rig.press(Modifiers::NONE, Key::Escape);
    assert!(rig.is_open() && !rig.recording(), "open {} recording {}", rig.is_open(), rig.recording());
    assert!(rig.outcomes().is_empty());
    assert_eq!(rig.bindings("file.save"), ["Cmd+S"]);
    rig.press(Modifiers::NONE, Key::Escape);
    assert!(rig.has_outcome(|outcome| matches!(outcome, Outcome::Cancel)));
}

/// A Copy event, which a backend sends in place of the press, records Cmd+C once.
#[trial]
fn copy_event_records_cmd_c() {
    let mut rig = Rig::tuned(Platform::Mac, |_| {});
    rig.click("Save: ⌘S");
    rig.fixture.frame(vec![Event::Copy]).settle();
    assert_eq!(rig.bindings("file.save"), ["Cmd+C"]);
}

/// Two presses of K within the double-press time record `K K`; a slow second press records `K`.
#[trial]
fn double_press_records_k_k() {
    let mut rig = Rig::tuned(Platform::Mac, |options| options.double_press = true);
    rig.click("Save: ⌘S");
    rig.press(Modifiers::NONE, Key::K);
    assert!(rig.recording() && rig.has("Save: press keys"), "a single press waits for the second: {:?}", rig.bindings("file.save"));
    rig.fixture.advance(0.3);
    rig.press(Modifiers::NONE, Key::K);
    assert_eq!(rig.bindings("file.save"), ["K K"]);

    rig.click("Save: K K");
    rig.press(Modifiers::NONE, Key::K);
    rig.fixture.advance(0.9);
    rig.press(Modifiers::NONE, Key::K);
    assert_eq!(rig.bindings("file.save"), ["K"]);
}
