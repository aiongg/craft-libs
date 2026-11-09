//! Recording through the dialog: the recorder takes every key, the state decides what a chord means.

use craft_keymap::Platform;
use craft_menu::shortcuts::Outcome;
use craft_ui::testing::trial;
use egui::{Event, Key, Modifiers};

use super::fixture::Rig;

/// A chord another command would run is shown with the conflict and applied only after an answer.
#[trial]
fn recording_captures_a_chord_another_command_would_run() {
    let mut rig = Rig::tuned(Platform::Mac, |_| {});
    rig.click("Save: ⌘S");
    assert!(rig.has("Save: press keys"));
    rig.press(Modifiers::COMMAND, Key::E);
    assert!(rig.has("⌘E is used by Export…"));
    assert_eq!(rig.bindings("file.save"), ["Cmd+S"], "nothing applied before an answer");
}

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

/// While recording, Escape with any modifier is a chord; only plain Escape stops the recording.
#[trial]
fn alt_escape_is_recorded_as_a_chord() {
    let mut rig = Rig::tuned(Platform::Mac, |_| {});
    rig.click("Save: ⌘S");
    rig.press(Modifiers::ALT, Key::Escape);
    assert_eq!(rig.bindings("file.save"), ["Alt+Escape"]);
    assert!(rig.is_open(), "Alt+Escape did not close the dialog");
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

/// The conflict bar says Accept and Go to Conflict, and Return is drawn as the Apple glyph.
#[trial]
fn the_conflict_bar_says_accept_and_go_to_conflict_and_return_is_drawn_as_the_apple_glyph() {
    let mut rig = Rig::tuned(Platform::Mac, |_| {});
    rig.click("Export…: ⌘E");
    rig.press(Modifiers::COMMAND, Key::S);
    assert!(rig.has("Accept and Go to Conflict") && !rig.has("Take It and Go to Conflict"));
    let mut rig = Rig::tuned(Platform::Mac, |_| {});
    rig.click("Export…: ⌘E");
    rig.press(Modifiers::COMMAND, Key::Enter);
    assert!(rig.has("Export…: ⌘↩") && !rig.has("Export…: ⌘⏎"), "Return is ↩");
}
