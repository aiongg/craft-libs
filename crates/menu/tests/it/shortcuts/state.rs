//! What the dialog does with recorded chords and set actions over several passes, read from the keymap, the sets and the
//! outcomes.

use craft_keymap::{ContextRule, Flavor, ForeignEntry, ForeignKey, ForeignSet, ImportOptions, Overrides, Platform, Profiles};
use craft_menu::shortcuts::Outcome;
use craft_ui::testing::trial;
use egui::{Key, Modifiers};

use super::fixture::{Rig, keymap, options};

fn rig_with(options: craft_menu::shortcuts::Options) -> Rig {
    Rig::new(keymap(Platform::Mac), options, None)
}

/// Tab and Shift+Escape are recorded as chords; Escape alone ends the recording and nothing else; Cancel then restores
/// the opening keymap and Changed follows for the app's menus.
#[trial]
fn recording_takes_tab_and_shift_escape_and_escape_cancels_only_the_recording() {
    let mut rig = Rig::mac();
    for (target, key, modifiers) in
        [("Bring to Front: None", Key::Tab, Modifiers::NONE), ("Add a shortcut to Bring to Front", Key::Escape, Modifiers::SHIFT)]
    {
        rig.click(target);
        rig.press(modifiers, key);
    }
    assert_eq!(rig.bindings("object.front"), ["Tab", "Shift+Escape"]);
    rig.click("Bring to Front");
    rig.press(Modifiers::NONE, Key::Escape);
    assert_eq!(rig.bindings("object.front").len(), 2, "Escape only ended the recording");
    assert!(rig.shared.borrow().state.is_open());
    rig.press(Modifiers::NONE, Key::Escape);
    assert!(rig.has_outcome(|o| matches!(o, Outcome::Cancel)), "a second Escape cancels the dialog");
    assert!(rig.bindings("object.front").is_empty(), "Cancel restores the opening keymap");
}

/// With double presses on, a single press waits for a second; two within the window record "K K", two apart record "K".
#[trial]
fn double_press_records_k_k_within_the_window() {
    let mut rig = rig_with(options().with(|o| o.double_press = true));
    rig.click("Bring to Front: None");
    rig.press(Modifiers::NONE, Key::K);
    assert!(rig.shared.borrow().state.is_recording(), "a single waits for a second press: {:?}", rig.bindings("object.front"));
    rig.fixture.advance(0.3);
    rig.press(Modifiers::NONE, Key::K);
    assert!(!rig.shared.borrow().state.is_recording());
    assert_eq!(rig.bindings("object.front"), ["K K"]);

    rig.click("Add a shortcut to Bold");
    rig.press(Modifiers::NONE, Key::K);
    rig.fixture.advance(0.9);
    rig.fixture.idle(2);
    rig.press(Modifiers::NONE, Key::K);
    assert_eq!(rig.bindings("type.bold"), ["Text: Cmd+B", "K"]);
}

/// Backspace without modifiers removes the binding being replaced when the option is on.
#[trial]
fn backspace_clears_the_binding_being_replaced() {
    let mut rig = rig_with(options().with(|o| o.backspace_clears = true));
    rig.click("Save: ⌘S");
    rig.press(Modifiers::NONE, Key::Backspace);
    assert!(rig.bindings("file.save").is_empty());
}

/// Deleting the last set of an app without built-in sets loads no overrides, and OK stores no copy of the deleted set.
#[trial]
fn deleting_the_last_set_without_builtins_loads_no_overrides() {
    let mut mine = Overrides::new();
    mine.insert("object.front", vec!["F2".into()]);
    let mut profiles = Profiles::default();
    profiles.save_as("Mine", mine.clone(), &[]).unwrap_or_else(|e| panic!("{e:?}"));
    let mut km = keymap(Platform::Mac);
    km.set_overrides(mine);
    let mut rig = Rig::with_builtins(km, options(), Some((profiles, Vec::new())));
    rig.click("Delete");
    assert!(rig.has("Delete the set “Mine”?"));
    rig.answer("Delete");
    let shared = rig.shared.borrow();
    assert!(shared.sets.as_ref().is_some_and(|(p, _)| p.sets().is_empty()) && shared.km.overrides().is_empty());
    drop(shared);
    rig.click("OK");
    assert!(rig.shared.borrow().sets.as_ref().is_some_and(|(p, _)| p.sets().is_empty()), "OK stores no copy of the deleted set");
}

fn foreign() -> (ForeignSet, ImportOptions) {
    let read = |text: &str| ForeignKey::read(Some("Default"), text, Flavor::Windows);
    let set = ForeignSet::new(vec![
        ForeignEntry::new(Some("File"), ["Save"]).with_key(read("Ctrl+Alt+S")).at_line(3),
        ForeignEntry::new(Some("File"), ["Frobnicate"]).with_key(read("Ctrl+F")).at_line(4),
    ])
    .with_name("Theirs");
    (set, ImportOptions { contexts: vec![("Default".into(), ContextRule::Home)], ..ImportOptions::default() })
}

/// Don't Save before an import drops the edits; the reviewed import then makes a new set from the file alone and leaves
/// the stored set as it was.
#[trial]
fn dont_save_before_import_drops_the_edits() {
    let mut profiles = Profiles::default();
    profiles.save_as("Mine", Overrides::new(), &super::fixture::builtins()).unwrap_or_else(|e| panic!("{e:?}"));
    let mut rig = Rig::mac_with_sets(profiles);
    rig.click("Bring to Front: None");
    rig.press(Modifiers::NONE, Key::F2);
    rig.action("Import…");
    rig.click("Don't Save");
    assert!(rig.has_outcome(|o| matches!(o, Outcome::Import { .. })));
    assert!(rig.shared.borrow().km.overrides().is_empty());
    let (set, options) = foreign();
    rig.review_import(set, options);
    rig.click("Import as New Set");
    assert_eq!(rig.active_set().as_deref(), Some("Theirs"));
    let shared = rig.shared.borrow();
    assert_eq!(shared.sets.as_ref().and_then(|(p, b)| p.get("Theirs", b)).and_then(|o| o.get("object.front")), None);
    assert!(shared.sets.as_ref().and_then(|(p, b)| p.get("Mine", b)).is_some_and(Overrides::is_empty));
}

/// The review changes nothing until Import as New Set; the new set is active with the file's shortcuts, and Done returns
/// to the table.
#[trial]
fn foreign_import_review_makes_a_new_set() {
    let mut rig = Rig::mac_with_sets(Profiles::default());
    let (set, options) = foreign();
    rig.review_import(set, options);
    assert!(rig.has("Import “Theirs”"));
    assert!(rig.shared.borrow().km.overrides().is_empty(), "review changes nothing");
    rig.click("Import as New Set");
    assert_eq!(rig.active_set().as_deref(), Some("Theirs"));
    assert_eq!(rig.bindings("file.save"), ["Cmd+Alt+S"]);
    rig.click("Done");
    assert!(rig.shared.borrow().state.is_open() && rig.has("Commands"));
}
