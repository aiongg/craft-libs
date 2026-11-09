//! The dialog's frame: recording into a row, the conflict bar, undo, Cancel and OK, and the messages under the table.

use craft_keymap::{Active, Bindable, Binding, Chord, ConflictKind, Keymap, Kind, Lookup, Platform, Scope, Scopes};
use craft_menu::shortcuts::{ConflictMode, Outcome};
use craft_ui::testing::trial;
use egui::{Key, Modifiers};

use super::fixture::{Rig, keymap, options};
use craft_ui::contract::Role;

/// A chord pressed while a row records reaches the recorder first, and the row's bindings stay as they were until the
/// conflict is answered.
#[trial]
fn recording_takes_a_chord_another_command_would_run() {
    let mut rig = Rig::mac();
    rig.click("Save: ⌘S");
    assert!(rig.has("Save: press keys"));
    rig.press(Modifiers::MAC_CMD, Key::E);
    assert!(rig.has("⌘E is used by Export…"));
    assert_eq!(rig.bindings("file.save"), ["Cmd+S"], "nothing changes before the answer");
    assert!(!rig.shared.borrow().state.is_recording());
}

/// Take It moves the chord, the message offers Undo, and Undo and Redo (also Cmd+Z) move it back and forth.
#[trial]
fn conflict_take_it_steals_and_undo_restores() {
    let mut rig = Rig::mac();
    rig.click("Export…: ⌘E");
    rig.press(Modifiers::MAC_CMD, Key::S);
    rig.click("Take It");
    assert_eq!(rig.bindings("file.save"), Vec::<String>::new());
    assert_eq!(rig.bindings("file.export"), ["Cmd+S"]);
    assert!(rig.has("⌘S was removed from Save"));
    rig.click("Undo");
    assert_eq!(rig.bindings("file.save"), ["Cmd+S"]);
    assert_eq!(rig.bindings("file.export"), ["Cmd+E"]);
    rig.click("Redo");
    assert_eq!(rig.bindings("file.export"), ["Cmd+S"]);
    rig.press(Modifiers::MAC_CMD, Key::Z);
    assert_eq!(rig.bindings("file.export"), ["Cmd+E"], "Cmd+Z undoes when no text field has focus");
}

/// Keep Both lists the clash under the Conflicts filter; undoing the changes changes nothing.
#[trial]
fn conflict_keep_both_lists_the_clash_and_cancel_changes_nothing() {
    let mut rig = Rig::mac();
    rig.click("Export…: ⌘E");
    rig.press(Modifiers::MAC_CMD, Key::S);
    rig.click("Undo Changes");
    assert_eq!(rig.bindings("file.export"), ["Cmd+E"]);
    rig.click("Export…: ⌘E");
    rig.press(Modifiers::MAC_CMD, Key::S);
    rig.click("Keep Both");
    let clashes = rig.shared.borrow().km.conflicts().into_iter().filter(|c| c.kind == ConflictKind::Clash).count();
    assert_eq!(clashes, 1);
    assert!(rig.has("Also used by Save"));
    let link = rig.find("1 shortcut is assigned more than once").rect;
    rig.click("1 shortcut is assigned more than once");
    assert!(rig.has("Save: ⌘S") && rig.has("Export…: ⌘S"));
    assert!(
        !rig.has("Save As…: ⇧⌘S"),
        "the Conflicts filter shows only the clash: link {link:?}, link still there {}, filter named Conflicts {}, rows {:?}",
        rig.has("1 shortcut is assigned more than once"),
        rig.has("Conflicts"),
        rig.names_with(Role::ROW, "")
    );
}

/// Assign in the detail pane takes a held shortcut without the bar and leaves the command's other shortcuts.
#[trial]
fn assign_takes_a_held_shortcut_and_keeps_the_others() {
    let key = |text: &str| Binding::parse(text).unwrap_or_else(|e| panic!("{e}"));
    let catalog = vec![
        Bindable::new("file.print", "Print…", Scope::app()).with_default(key("Cmd+P")),
        Bindable::new("type.paragraphStyles", "Paragraph Styles", Scope::app()).with_default(key("Cmd+F11")),
    ];
    let mut rig = Rig::new(Keymap::new(catalog, Scopes::new(), Platform::Mac), options(), None);
    rig.click("Paragraph Styles");
    rig.click("New shortcut: None");
    rig.press(Modifiers::MAC_CMD, Key::P);
    assert!(rig.has("Currently assigned to:"));
    assert!(rig.has("Print…"), "the holder is listed under the field");
    rig.click("Assign");
    assert!(rig.has("⌘P was removed from Print…"));
    rig.click("OK");
    assert!(rig.has_outcome(|o| matches!(o, Outcome::Ok)));
    let shared = rig.shared.borrow();
    let run = |text: &str| shared.km.lookup(&Chord::parse(text).unwrap_or_else(|e| panic!("{e}")), false, &Active::NONE);
    assert_eq!(run("Cmd+P"), Lookup::Run("type.paragraphStyles".into()));
    assert_eq!(run("Cmd+F11"), Lookup::Run("type.paragraphStyles".into()));
    drop(shared);
    assert!(rig.bindings("file.print").is_empty());
}

/// A reserved chord and the app's own rule refuse with a message and change nothing.
#[trial]
fn reserved_chord_and_app_rule_refuse() {
    let reserved = Chord::parse("Cmd+Tab").unwrap_or_else(|e| panic!("{e}"));
    let options = options().with(|o| {
        o.accept = Some(Box::new(|bindable, binding| {
            if matches!(bindable.kind, Kind::Tool { .. }) && !binding.trigger.chord.mods.is_none() {
                Err("Tools take keys without modifiers".into())
            } else {
                Ok(())
            }
        }));
    });
    let mut rig = Rig::new(keymap(Platform::Mac).with_reserved(vec![reserved]), options, None);
    rig.click("Save: ⌘S");
    rig.press(Modifiers::MAC_CMD, Key::Tab);
    assert!(rig.has("⌘⇥ is reserved by the system"));
    assert_eq!(rig.bindings("file.save"), ["Cmd+S"]);
    rig.click("Marquee Tool: M");
    rig.press(Modifiers::MAC_CMD, Key::K);
    assert!(rig.has("Tools take keys without modifiers"));
    assert_eq!(rig.bindings("tool.marquee"), ["M"]);
}

/// Cancel puts the keymap back, and Changed follows because it undid an edit.
#[trial]
fn cancel_restores_the_keymap_and_reports_changed_after() {
    let mut rig = Rig::mac();
    rig.click("Save: ⌘S");
    rig.press(Modifiers::NONE, Key::F2);
    assert_eq!(rig.bindings("file.save"), ["F2"]);
    rig.click("Cancel");
    assert_eq!(rig.bindings("file.save"), ["Cmd+S"]);
    let after: Vec<bool> = rig.outcomes().iter().skip_while(|o| !matches!(o, Outcome::Cancel)).map(|o| matches!(o, Outcome::Changed)).collect();
    assert_eq!(after, [false, true], "Changed follows Cancel, which undid an edit");
    assert!(!rig.shared.borrow().state.is_open());
}

