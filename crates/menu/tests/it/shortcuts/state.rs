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
