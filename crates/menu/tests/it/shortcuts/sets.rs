//! The set row: switching, the New Set form, prompts that hold the dialog, OK and Cancel with sets, export and import.

use craft_keymap::{Overrides, Profiles};
use craft_menu::shortcuts::Outcome;
use craft_ui::contract::Role;
use craft_ui::testing::trial;
use egui::{Key, Modifiers};

use super::fixture::{Rig, builtins, keymap, options};

fn profiles_with(names: &[&str]) -> Profiles {
    let mut profiles = Profiles::default();
    for name in names {
        profiles.save_as(name, Overrides::new(), &builtins()).unwrap_or_else(|e| panic!("{name}: {e:?}"));
    }
    profiles
}

fn new_set(rig: &mut Rig, name: &str) {
    rig.click("New Set…");
    let field = rig.named("Name:", Role::TEXT_INPUT);
    rig.fixture.click(&field);
    rig.fixture.type_text(name);
    rig.fixture.idle(2);
    rig.click("Create");
}

/// A taken name, in any case or with spaces around it, is refused for New Set and Rename, and the set stays.
#[trial]
fn new_set_refuses_a_taken_name_and_keeps_the_set() {
    let mut rig = Rig::mac_with_sets(profiles_with(&["Mine"]));
    for name in ["Mine", " mine ", "fixture defaults"] {
        new_set(&mut rig, name);
        assert!(rig.has("A set with this name exists"), "{name:?}");
        assert_eq!(rig.active_set().as_deref(), Some("Fixture Defaults"));
        assert_eq!(rig.shared.borrow().sets.as_ref().map(|(p, _)| p.sets().len()), Some(1));
        rig.click("Cancel");
    }
}

/// Cancel restores the keymap and the sets as they were at open, also after a set was made and deleted.
#[trial]
fn cancel_restores_the_keymap_and_the_sets() {
    let before = profiles_with(&["Mine"]);
    let mut rig = Rig::mac_with_sets(before.clone());
    rig.click("Save: ⌘S");
    rig.press(Modifiers::NONE, Key::F2);
    new_set(&mut rig, "Other");
    assert_eq!(rig.active_set().as_deref(), Some("Other"));
    assert_eq!(rig.bindings("file.save"), ["F2"], "based on the current shortcuts");
    rig.click("Delete");
    assert!(rig.has("Delete the set “Other”?"));
    rig.click("Cancel");
    assert!(rig.has_outcome(|o| matches!(o, Outcome::Cancel)));
    let shared = rig.shared.borrow();
    assert_eq!(shared.sets.as_ref().map(|(p, _)| p), Some(&before));
    assert!(shared.km.overrides().is_empty());
}

/// While a prompt is open the set row, the table and the edit actions hold, and answering it frees them.
#[trial]
fn an_open_prompt_holds_the_set_row_table_and_edit_actions() {
    let mut rig = Rig::mac_with_sets(Profiles::default());
    rig.click("Save: ⌘S");
    rig.press(Modifiers::NONE, Key::F2);
    new_set(&mut rig, "Other");
    rig.click("Delete");
    assert!(rig.has("Delete the set “Other”?"));
    rig.click("Save: F2");
    assert!(!rig.has("Save: press keys"), "the table holds");
    rig.click("OK");
    assert!(!rig.has_outcome(|o| matches!(o, Outcome::Ok)), "OK waits for the prompt");
    rig.click("Cancel");
    assert!(rig.shared.borrow().state.is_open());
    rig.click("OK");
    assert!(rig.has_outcome(|o| matches!(o, Outcome::Ok)));
}

/// OK stores the edits of a built-in set as a set named Custom.
#[trial]
fn ok_stores_edits_of_a_builtin_as_a_copy() {
    let mut rig = Rig::mac_with_sets(Profiles::default());
    rig.click("Save: ⌘S");
    rig.press(Modifiers::NONE, Key::F2);
    rig.click("OK");
    assert!(rig.has_outcome(|o| matches!(o, Outcome::Ok)));
    let shared = rig.shared.borrow();
    let profiles = &shared.sets.as_ref().map(|(p, _)| p).unwrap_or_else(|| panic!("sets"));
    assert_eq!(profiles.active(), "Custom");
    assert_eq!(profiles.sets().get("Custom").and_then(|o| o.get("file.save")), Some(&["F2".to_string()][..]));
}

/// Switching away from a modified set asks to save; Don't Save loads the other set with a fresh undo history and
/// leaves the stored set as it was.
#[trial]
fn switching_a_modified_set_asks_to_save() {
    let mut rig = Rig::mac_with_sets(profiles_with(&["Mine"]));
    rig.click("Save: ⌘S");
    rig.press(Modifiers::NONE, Key::F2);
    let select = rig.named("Set:", Role::COMBO_BOX);
    rig.fixture.click(&select);
    rig.fixture.idle(3);
    rig.click("Fixture Defaults");
    assert!(rig.has("Save changes to “Mine”?"));
    rig.click("Don't Save");
    assert_eq!(rig.active_set().as_deref(), Some("Fixture Defaults"));
    assert!(rig.shared.borrow().km.overrides().is_empty());
    let shared = rig.shared.borrow();
    assert!(shared.sets.as_ref().and_then(|(p, b)| p.get("Mine", b)).is_some_and(Overrides::is_empty), "Don't Save keeps the stored set");
}

/// Import asks the app for a file; the Save changes prompt comes first, and Changed follows once the edits are dropped
/// or stored.
#[trial]
fn save_prompt_before_import_reports_changed_after_import() {
    for answer in ["Don't Save", "Save"] {
        let mut rig = Rig::mac_with_sets(profiles_with(&["Mine"]));
        rig.click("Save: ⌘S");
        rig.press(Modifiers::NONE, Key::F2);
        rig.click("Import…");
        rig.click(answer);
        let outcomes = rig.outcomes();
        let import = outcomes.iter().position(|o| matches!(o, Outcome::Import { .. }));
        assert!(import.is_some(), "{answer}");
        assert!(outcomes[import.unwrap_or_default()..].iter().any(|o| matches!(o, Outcome::Changed)), "{answer}");
    }
}

/// Import without edits goes straight to the app; Reset All asks first, and Undo brings the reset shortcuts back.
#[trial]
fn import_request_and_reset_all() {
    let mut rig = Rig::mac_with_sets(Profiles::default());
    rig.click("Import…");
    assert!(rig.has_outcome(|o| matches!(o, Outcome::Import { formats, .. } if formats == &["craft-keymap"])));
    rig.click("Save: ⌘S");
    rig.press(Modifiers::NONE, Key::F2);
    rig.click("Reset All…");
    assert!(rig.has("Restore every default shortcut in this set?"));
    rig.click("Reset All");
    assert!(rig.shared.borrow().km.overrides().is_empty());
    rig.click("Undo");
    assert_eq!(rig.bindings("file.save"), ["F2"]);
}

/// A set exported from one dialog imports into another as a new set with the same shortcuts; a file that is not a set
/// is refused; without sets the import replaces the keymap's overrides as one undo step.
#[trial]
fn set_export_import_round_trip() {
    let mut rig = Rig::mac_with_sets(Profiles::default());
    new_set(&mut rig, "Name");
    rig.click("Save: ⌘S");
    rig.press(Modifiers::NONE, Key::F2);
    rig.click("Export…");
    let (name, json) = rig
        .outcomes()
        .into_iter()
        .find_map(|o| if let Outcome::Export { name, json, .. } = o { Some((name, json)) } else { None })
        .unwrap_or_else(|| panic!("Outcome::Export"));
    assert_eq!(name, "Name");
    let edited = rig.shared.borrow().km.overrides().clone();
    let profiles = rig.shared.borrow().sets.as_ref().map(|(p, _)| p.clone());

    let mut fresh = Rig::new(keymap(craft_keymap::Platform::Mac), options(), profiles);
    assert_eq!(fresh.import_json(&json), Ok("Name 2".into()));
    assert_eq!(fresh.active_set().as_deref(), Some("Name 2"));
    assert_eq!(fresh.shared.borrow().km.overrides(), &edited);
    assert!(fresh.import_json("{}").is_err());
    assert!(fresh.has("This is not a shortcut set file") || fresh.has("This is not a craft-keymap file"));

    let mut plain = Rig::mac();
    assert!(plain.import_json(&json).is_ok());
    assert_eq!(plain.shared.borrow().km.overrides(), &edited);
    plain.click("Undo");
    assert!(plain.shared.borrow().km.overrides().is_empty());
}

/// On a narrow dialog the set actions that do not fit move into the overflow menu, and a pick there runs the action.
#[trial]
fn narrow_set_row_moves_actions_into_more() {
    let mut rig = Rig::mac_with_sets(Profiles::default());
    rig.fixture.room(egui::vec2(560.0, 600.0));
    rig.fixture.settle();
    assert!(!rig.has("Summarize…"), "Summarize… is in More: {:?}", rig.names_with(Role::BUTTON, "…"));
    rig.click("More");
    rig.click("Summarize…");
    assert!(rig.has_outcome(|o| matches!(o, Outcome::Summary { name, .. } if name == "Fixture Defaults")));
}
