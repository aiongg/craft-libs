//! Recording through the dialog: the recorder takes every key, the state decides what a chord means.
//!
//! Written against keymap-c's fixture (`fixture::keymap`, `fixture::options`) and the dialog's public surface.

use std::cell::RefCell;
use std::rc::Rc;

use craft_host::target::Os;
use craft_keymap::{CommandRef, ForeignSet, ImportOptions, Keymap, Platform};
use craft_menu::shortcuts::{DialogState, Options, Outcome, ShortcutsDialog};
use craft_ui::testing::{Fixture, trial};
use egui::{Event, Key, Modifiers};

use super::fixture;

/// The dialog and what it works on, shared with the fixture's closure.
pub(crate) struct Shared {
    pub keymap: Keymap,
    pub state: DialogState,
    pub options: Options,
    pub outcome: Option<Outcome>,
    /// A foreign set to put under review on the next pass.
    pub review: Option<(ForeignSet, ImportOptions)>,
}

/// A dialog on a keymap, driven pass by pass.
pub(crate) struct Rig {
    pub fixture: Fixture,
    pub shared: Rc<RefCell<Shared>>,
}

impl Rig {
    pub(crate) fn new(platform: Platform, tune: impl FnOnce(&mut Options)) -> Rig {
        let mut options = fixture::options();
        tune(&mut options);
        let shared =
            Rc::new(RefCell::new(Shared { keymap: fixture::keymap(platform), state: DialogState::new(), options, outcome: None, review: None }));
        let inner = Rc::clone(&shared);
        let mut fixture = Fixture::new(move |ui| {
            let mut shared = inner.borrow_mut();
            let Shared { keymap, state, options, outcome, review } = &mut *shared;
            if let Some((set, import)) = review.take() {
                ShortcutsDialog::new(keymap).options(options).review_import(ui, state, set, import);
            }
            let shown = ShortcutsDialog::new(keymap).options(options).show(ui, state);
            if let Some(done) = shown.outcome {
                *outcome = Some(done);
            }
        });
        fixture.os(if platform.is_mac() { Os::Mac } else { Os::Windows }).settle();
        Rig { fixture, shared }
    }

    pub(crate) fn has(&self, label: &str) -> bool {
        self.fixture.by_label(label).is_some()
    }

    pub(crate) fn click(&mut self, label: &str) {
        let target = self.fixture.by_label(label).unwrap_or_else(|| panic!("no node labelled {label}"));
        self.fixture.click(&target).settle();
    }

    pub(crate) fn press(&mut self, key: Key, modifiers: Modifiers) {
        self.fixture.key(key, modifiers).settle();
    }

    pub(crate) fn bindings(&self, id: &str) -> Vec<String> {
        self.shared.borrow().keymap.bindings(&CommandRef::from(id)).iter().map(ToString::to_string).collect()
    }
}

/// A chord another command would run is shown with the conflict and applied only after an answer.
#[trial]
fn recording_captures_a_chord_another_command_would_run() {
    let mut rig = Rig::new(Platform::Mac, |_| {});
    rig.click("Save: ⌘S");
    assert!(rig.has("Save: press keys"));
    rig.press(Key::E, Modifiers::COMMAND);
    assert!(rig.has("⚠ ⌘E is used by Export…"));
    assert_eq!(rig.bindings("file.save"), ["Cmd+S"], "nothing applied before an answer");
}

/// While recording, Tab, an arrow and Shift+Escape are chords, not focus moves or a close.
#[trial]
fn recording_takes_tab_arrows_and_shift_escape() {
    let mut rig = Rig::new(Platform::Mac, |_| {});
    let mut current = "Save: ⌘S".to_owned();
    for (key, modifiers, expected, shown) in [
        (Key::Tab, Modifiers::NONE, "Tab", "⇥"),
        (Key::ArrowDown, Modifiers::NONE, "Down", "↓"),
        (Key::Escape, Modifiers::SHIFT, "Shift+Escape", "⇧⎋"),
    ] {
        rig.click(&current);
        rig.press(key, modifiers);
        assert_eq!(rig.bindings("file.save"), [expected]);
        current = format!("Save: {shown}");
        assert!(rig.has(&current));
    }
    assert!(rig.shared.borrow().state.is_open(), "Shift+Escape did not close the dialog");
}

/// Escape stops the recording; a second Escape cancels the dialog.
#[trial]
fn escape_cancels_recording_only() {
    let mut rig = Rig::new(Platform::Mac, |_| {});
    rig.click("Save: ⌘S");
    rig.press(Key::Escape, Modifiers::NONE);
    assert!(rig.shared.borrow().state.is_open() && !rig.shared.borrow().state.is_recording());
    assert!(rig.shared.borrow().outcome.is_none());
    assert_eq!(rig.bindings("file.save"), ["Cmd+S"]);
    rig.press(Key::Escape, Modifiers::NONE);
    assert!(matches!(rig.shared.borrow().outcome, Some(Outcome::Cancel)));
}

/// A Copy event, which a backend sends in place of the press, records Cmd+C once.
#[trial]
fn copy_event_records_cmd_c() {
    let mut rig = Rig::new(Platform::Mac, |_| {});
    rig.click("Save: ⌘S");
    rig.fixture.frame(vec![Event::Copy]).settle();
    assert_eq!(rig.bindings("file.save"), ["Cmd+C"]);
}

/// Two presses of K within the double-press time record `K K`; a slow second press records `K`.
#[trial]
fn double_press_records_k_k() {
    let mut rig = Rig::new(Platform::Mac, |options| options.double_press = true);
    rig.click("Save: ⌘S");
    rig.press(Key::K, Modifiers::NONE);
    assert!(rig.has("Save: K (press again for a double press)"));
    rig.fixture.advance(0.3);
    rig.press(Key::K, Modifiers::NONE);
    assert_eq!(rig.bindings("file.save"), ["K K"]);

    rig.click("Save: K K");
    rig.press(Key::K, Modifiers::NONE);
    rig.fixture.advance(0.9);
    rig.press(Key::K, Modifiers::NONE);
    assert_eq!(rig.bindings("file.save"), ["K"]);
}

/// The conflict bar says Accept and Go to Conflict, and Return is drawn as the Apple glyph.
#[trial]
fn the_conflict_bar_says_accept_and_go_to_conflict_and_return_is_drawn_as_the_apple_glyph() {
    let mut rig = Rig::new(Platform::Mac, |_| {});
    rig.click("Export…: ⌘E");
    rig.press(Key::S, Modifiers::COMMAND);
    assert!(rig.has("Accept and Go to Conflict") && !rig.has("Take It and Go to Conflict"));
    let mut rig = Rig::new(Platform::Mac, |_| {});
    rig.click("Export…: ⌘E");
    rig.press(Key::Enter, Modifiers::COMMAND);
    assert!(rig.has("Export…: ⌘↩") && !rig.has("Export…: ⌘⏎"), "Return is ↩");
}
