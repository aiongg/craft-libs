//! The table's model through the dialog: what each Show area, filter and search lists, how groups sort and
//! collapse, and which rows count as conflicts.

use craft_keymap::{Bindable, Binding, CommandRef, Keymap, Kind, Platform, Policy, Scope, Scopes, Trigger};
use craft_menu::shortcuts::{FixedRow, Options};
use craft_ui::testing::trial;

use super::fixture::{self, Rig};

#[trial]
fn areas_follow_paths_kind_and_scope() {
    let mut rig = Rig::mac();
    rig.open("Show:");
    for area in ["All Commands", "File Menu", "Object Menu", "Tools", "Timeline Panel", "Other"] {
        assert!(rig.has(area), "{area} is a Show entry");
    }
    rig.click("Object Menu");
    assert!(rig.has("Bring to Front: None"), "the Arrange submenu is a group, and the command has no shortcut");
    assert!(!rig.has("Save: ⌘S"));
    rig.choose("Show:", "Tools");
    assert!(rig.has("Marquee Tool (cycles): M") || rig.has("Marquee Tool (cycles)"), "two tools of a group share M");
    rig.choose("Show:", "Timeline Panel");
    assert!(rig.has("Loop: L"));
    rig.choose("Show:", "Other");
    assert!(rig.has("Bold: ⌘B (Text)"), "a binding in another scope shows the scope");
}

#[trial]
fn search_by_name_translation_or_chord() {
    let mut rig = Rig::mac();
    let rows = |rig: &Rig| ["Save: ⌘S", "Save As…: ⇧⌘S", "Export…: ⌘E", "Bold: ⌘B (Text)"].map(|row| rig.has(row));
    rig.search("save");
    assert_eq!(rows(&rig), [true, true, false, false], "by name");
    rig.search("");
    rig.search("cmd+s");
    assert_eq!(rows(&rig), [true, false, false, false], "a spelled chord matches that chord only");
    rig.search("");
    rig.search("cmd+b");
    assert_eq!(rows(&rig), [false, false, false, true], "a trigger matches in any scope");
    rig.search("");
    rig.search("zzz");
    assert!(rig.has("No matching commands"));
}

#[trial]
fn filters_and_conflict_counts() {
    let mut rig = Rig::new(fixture::keymap(Platform::Windows), fixture::options(), None);
    rig.shared.borrow_mut().km.assign(&CommandRef::from("file.export"), Binding::parse("Cmd+S").expect("a chord"), Policy::Keep).expect("assigns");
    rig.pass();
    rig.choose("Filter:", "Changed");
    assert!(rig.has("Export…: Ctrl+E") && !rig.has("Save: Ctrl+S"), "only commands with an override");
    rig.choose("Filter:", "Conflicts");
    assert!(rig.has("Save: Ctrl+S") && rig.has("Export…: Ctrl+E"), "both holders of Ctrl+S");
    rig.choose("Filter:", "With Shortcut");
    assert!(!rig.has("Bring to Front: None"));
}

#[trial]
fn hidden_entries_are_not_listed_but_still_conflict() {
    let mut options = fixture::options();
    options.visible = Some(|b| b.command.id != "file.export");
    let mut rig = Rig::new(fixture::keymap(Platform::Mac), options, None);
    assert!(!rig.has("Export…: ⌘E"), "a hidden command is not listed");
    rig.shared.borrow_mut().km.assign(&CommandRef::from("file.save"), Binding::parse("Cmd+E").expect("a chord"), Policy::Keep).expect("assigns");
    rig.pass();
    rig.choose("Filter:", "Conflicts");
    assert!(rig.has("Save: ⌘S") && rig.has("Save: ⌘E"), "the hidden command still clashes with Save");
}

#[trial]
fn commands_sharing_a_default_key_are_no_conflict() {
    let mut rig = Rig::new(fixture::keymap(Platform::Mac), fixture::options(), None);
    rig.choose("Filter:", "Conflicts");
    assert!(!rig.has("Marquee Tool: M"), "tools of one group sharing a key cycle; they do not clash");
}

#[trial]
fn fixed_rows_strike_through_when_a_command_takes_their_chord() {
    let chord = |text: &str| Trigger::parse(text).unwrap_or_else(|e| panic!("{text}: {e}"));
    let mut options = fixture::options();
    options.fixed = vec![FixedRow::new("Rating", "Rate 1", "1").chord(chord("1")), FixedRow::new("Rating", "Loop", "L").chord(chord("L"))];
    let mut rig = Rig::new(fixture::keymap(Platform::Mac), options, None);
    rig.choose("Show:", "Fixed Keys");
    assert!(rig.has("Loop: L (Now used by Loop)"), "a fixed row whose chord a command took names the command");
    assert!(rig.has("Rate 1: 1"), "a free fixed row is plain");
}

