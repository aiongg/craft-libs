//! The dialog body through its public surface: keys in the table, recording from a chip, the detail pane, the
//! typed entry, conflicts and the read-only dialog.

use craft_keymap::Platform;
use craft_ui::testing::trial;
use egui::{Key, Modifiers};

use super::fixture::{self, Rig};

fn selected(rig: &Rig, id: &str) -> bool {
    let shared = rig.shared.borrow();
    shared.state.snapshot(&shared.km).is_some_and(|snapshot| snapshot.selected.is_some_and(|command| command.id == id))
}

#[trial]
fn group_headers_take_focus_and_toggle_with_enter() {
    let mut rig = Rig::mac();
    rig.click("File");
    assert!(!rig.has("Export…: ⌘E"), "a click on the header closes the group");
    rig.press(Modifiers::NONE, Key::Enter);
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
    rig.press(Modifiers::COMMAND, Key::S);
    rig.click("Accept and Go to Conflict");
    assert_eq!(rig.bindings("tool.marquee"), ["Cmd+S"]);
    assert!(selected(&rig, "file.save"), "the holder is selected");
    assert!(rig.has("Export…: ⌘E"), "the area and the search are cleared to show it");
}

#[trial]
fn typed_entry_parses_and_shows_errors() {
    let mut rig = Rig::mac();
    rig.click("Save");
    rig.click("Type…");
    rig.fixture.type_text("Cmd+Nope");
    rig.press(Modifiers::NONE, Key::Enter);
    assert!(rig.has("unknown key Nope"));
    rig.press(Modifiers::COMMAND, Key::A);
    rig.fixture.type_text("Cmd+Shift+K");
    rig.press(Modifiers::NONE, Key::Enter);
    assert!(!rig.has("unknown key Nope"));
    rig.click("Assign");
    assert_eq!(rig.bindings("file.save"), ["Cmd+S", "Cmd+Shift+K"]);
}

#[trial]
fn read_only_has_no_edit_controls() {
    let mut options = fixture::options();
    options.editable = false;
    let mut rig = Rig::new(fixture::keymap(Platform::Mac), options, None);
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
    rig.press(Modifiers::COMMAND, Key::Z);
    assert_eq!(rig.bindings("file.save"), ["Cmd+S"]);
}

#[trial]
fn tab_leaves_the_new_shortcut_field_after_recording() {
    let mut rig = Rig::mac();
    rig.click("Save");
    rig.click("New shortcut: None");
    rig.press(Modifiers::NONE, Key::F8);
    assert!(!rig.recording(), "a recorded chord ends the recording");
    rig.press(Modifiers::NONE, Key::Tab);
    assert!(!rig.recording(), "Tab moves on instead of recording");
}
