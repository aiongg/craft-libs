//! The dialog body through its public surface: keys in the table, recording from a chip, the detail pane, the
//! typed entry, conflicts and the read-only dialog.

use craft_keymap::Platform;
use craft_menu::shortcuts::ConflictMode;
use craft_ui::testing::trial;
use egui::{Key, Modifiers};

use super::fixture;
use super::rows::Rig;

fn selected(rig: &Rig, id: &str) -> bool {
    rig.state.snapshot(&rig.km).is_some_and(|snapshot| snapshot.selected.is_some_and(|command| command.id == id))
}

#[trial]
fn table_keys_move_record_and_delete() {
    let mut rig = Rig::mac();
    rig.click("Save");
    rig.press(Key::ArrowDown, Modifiers::NONE);
    assert!(selected(&rig, "file.saveAs"), "Down moves the selection");
    rig.press(Key::Delete, Modifiers::NONE);
    assert!(rig.bindings("file.saveAs").is_empty(), "Delete removes the current shortcut");
    rig.press(Key::Enter, Modifiers::NONE);
    assert!(rig.state.is_recording(), "Enter records into the row");
    rig.press(Key::F7, Modifiers::NONE);
    assert_eq!(rig.bindings("file.saveAs"), ["F7"]);
}

#[trial]
fn group_headers_take_focus_and_toggle_with_enter() {
    let mut rig = Rig::mac();
    rig.click("File");
    assert!(!rig.has("Export…: ⌘E"), "a click on the header closes the group");
    rig.press(Key::Enter, Modifiers::NONE);
    assert!(rig.has("Export…: ⌘E"), "Enter on the header opens it again");
}

#[trial]
fn one_pointer_click_on_a_chip_of_another_row_records() {
    let mut rig = Rig::mac();
    rig.click("Save");
    rig.click("Export…: ⌘E");
    assert!(rig.has("Export…: press keys"), "the chip records at once");
}

#[trial]
fn go_to_conflict_selects_and_reveals_the_holder() {
    let mut rig = Rig::mac();
    rig.choose("Show:", "Tools");
    rig.search("marquee");
    assert!(!rig.has("Save: ⌘S"));
    rig.click("Marquee Tool: M");
    rig.press(Key::S, Modifiers::COMMAND);
    rig.click("Accept and Go to Conflict");
    assert_eq!(rig.bindings("tool.marquee"), ["Cmd+S"]);
    assert!(selected(&rig, "file.save"), "the holder is selected");
    assert!(rig.has("Export…: ⌘E"), "the area and the search are cleared to show it");
}

#[trial]
fn the_new_shortcut_field_waits_for_assign() {
    let mut options = fixture::options();
    options.conflicts = ConflictMode::Keep;
    let mut rig = Rig::new(fixture::keymap(Platform::Mac), options);
    rig.click("Save");
    rig.click("New shortcut: None");
    rig.press(Key::E, Modifiers::COMMAND);
    assert!(rig.has("Currently assigned to:"));
    assert_eq!(rig.bindings("file.save"), ["Cmd+S"], "nothing changes before Assign");
    rig.click("Assign");
    assert!(rig.has("Also used by Export…"), "Keep mode keeps both");
    assert_eq!(rig.bindings("file.save"), ["Cmd+S", "Cmd+E"]);
}

#[trial]
fn typed_entry_parses_and_shows_errors() {
    let mut rig = Rig::mac();
    rig.click("Save");
    rig.click("Type…");
    rig.f.type_text("Cmd+Nope");
    rig.press(Key::Enter, Modifiers::NONE);
    assert!(rig.has("⚠ unknown key Nope"));
    rig.press(Key::A, Modifiers::COMMAND);
    rig.f.type_text("Cmd+Shift+K");
    rig.press(Key::Enter, Modifiers::NONE);
    assert!(!rig.has("⚠ unknown key Nope"));
    rig.click("Assign");
    assert_eq!(rig.bindings("file.save"), ["Cmd+S", "Cmd+Shift+K"]);
}

#[trial]
fn read_only_has_no_edit_controls() {
    let mut options = fixture::options();
    options.editable = false;
    let mut rig = Rig::new(fixture::keymap(Platform::Mac), options);
    rig.click("Save");
    rig.click("Save: ⌘S");
    assert!(!rig.has("Save: press keys"), "no recording");
    for label in [
        "Add a shortcut to Save",
        "Restore the default for Save",
        "Remove ⌘S from Save",
        "New shortcut:",
        "Context:",
        "Assign",
        "Use Default",
        "Clear",
    ] {
        assert!(!rig.has(label), "{label} is shown");
    }
    rig.press(Key::Z, Modifiers::COMMAND);
    assert_eq!(rig.bindings("file.save"), ["Cmd+S"]);
}

#[trial]
fn display_follows_the_keymap_platform() {
    let mac = Rig::mac();
    assert!(mac.has("Save As…: ⇧⌘S"));
    let windows = Rig::new(fixture::keymap(Platform::Windows), fixture::options());
    assert!(windows.has("Save As…: Ctrl+Shift+S"));
}

#[trial]
fn tab_leaves_the_new_shortcut_field_after_recording() {
    let mut rig = Rig::mac();
    rig.click("Save");
    rig.click("New shortcut: None");
    rig.press(Key::F8, Modifiers::NONE);
    assert!(!rig.state.is_recording(), "a recorded chord ends the recording");
    rig.press(Key::Tab, Modifiers::NONE);
    assert!(!rig.state.is_recording(), "Tab moves on instead of recording");
}

#[trial]
fn tab_leaves_search_by_key_after_recording() {
    let mut rig = Rig::mac();
    rig.click("Search by key");
    assert!(rig.state.is_recording());
    rig.press(Key::S, Modifiers::COMMAND);
    assert!(!rig.state.is_recording(), "the chord becomes the query and the recording ends");
    rig.press(Key::Tab, Modifiers::NONE);
    assert!(!rig.state.is_recording(), "Tab moves on instead of recording");
}
