//! The table's model through the dialog: what each Show area, filter and search lists, how groups sort and
//! collapse, and which rows count as conflicts.

use craft_keymap::{Bindable, Binding, CommandRef, Keymap, Kind, Platform, Policy, Scope, Scopes};
use craft_menu::shortcuts::{DialogState, Options, ShortcutsOutput};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};

use super::fixture;

/// A dialog in a fixture: the keymap and state it edits, and the options it shows.
pub(super) struct Rig {
    pub f: Fixture,
    pub km: Keymap,
    pub state: DialogState,
    pub options: Options,
}

impl Rig {
    pub fn new(km: Keymap, options: Options) -> Rig {
        let mut rig = Rig { f: Fixture::new(|_| {}), km, state: DialogState::new(), options };
        rig.pass();
        rig
    }

    pub fn mac() -> Rig {
        Rig::new(fixture::keymap(Platform::Mac), fixture::options())
    }

    /// One pass of the dialog and the frames its answer needs.
    pub fn pass(&mut self) -> ShortcutsOutput {
        let out = fixture::show(&mut self.f, &mut self.km, &mut self.state, &self.options);
        self.f.idle(2);
        out
    }

    pub fn has(&self, label: &str) -> bool {
        self.f.by_label(label).is_some()
    }

    pub fn click(&mut self, label: &str) {
        let target = self.f.by_label(label).unwrap_or_else(|| panic!("{label} is not shown"));
        self.f.click(&target);
        self.pass();
    }

    pub fn press(&mut self, key: Key, modifiers: Modifiers) {
        self.f.key(key, modifiers);
        self.pass();
    }

    pub fn search(&mut self, text: &str) {
        self.click("Search:");
        self.f.type_text(text);
        self.pass();
    }

    /// Picks `entry` in the select named `select`.
    pub fn choose(&mut self, select: &str, entry: &str) {
        self.click(select);
        self.click(entry);
    }

    pub fn bindings(&self, id: &str) -> Vec<String> {
        self.km.bindings(&CommandRef::from(id)).iter().map(Binding::to_string).collect()
    }
}

#[trial]
fn areas_follow_paths_kind_and_scope() {
    let mut rig = Rig::mac();
    rig.click("Show:");
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
    let mut rig = Rig::new(fixture::keymap(Platform::Windows), fixture::options());
    rig.km.assign(&CommandRef::from("file.export"), Binding::parse("Cmd+S").expect("a chord"), Policy::Keep).expect("assigns");
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
    let mut rig = Rig::new(fixture::keymap(Platform::Mac), options);
    assert!(!rig.has("Export…: ⌘E"), "a hidden command is not listed");
    rig.km.assign(&CommandRef::from("file.save"), Binding::parse("Cmd+E").expect("a chord"), Policy::Keep).expect("assigns");
    rig.pass();
    rig.choose("Filter:", "Conflicts");
    assert!(rig.has("Save: ⌘S, ⌘E"), "the hidden command still clashes with Save");
}

#[trial]
fn commands_sharing_a_default_key_are_no_conflict() {
    let mut rig = Rig::new(fixture::keymap(Platform::Mac), fixture::options());
    rig.choose("Filter:", "Conflicts");
    assert!(!rig.has("Marquee Tool: M"), "tools of one group sharing a key cycle; they do not clash");
}

#[trial]
fn sort_by_label_and_collapsed_groups() {
    let mut options = fixture::options();
    options.sort_by_label = true;
    let mut rig = Rig::new(fixture::keymap(Platform::Mac), options);
    rig.choose("Show:", "File Menu");
    let at = |rig: &Rig, label: &str| rig.f.by_label(label).map(|t| t.center().y).expect("shown");
    assert!(at(&rig, "Export…: ⌘E") < at(&rig, "Save: ⌘S") && at(&rig, "Save: ⌘S") < at(&rig, "Save As…: ⇧⌘S"));
    rig.choose("Show:", "All Commands");
    rig.click("File");
    assert!(!rig.has("Save: ⌘S"), "a closed group lists no rows");
    assert!(rig.has("Object"), "the other groups stay");
}

#[trial]
fn fixed_rows_strike_through_when_a_command_takes_their_chord() {
    let mut rig = Rig::mac();
    rig.choose("Show:", "Fixed Keys");
    assert!(rig.has("Loop: L (Now used by Loop)"), "a fixed row whose chord a command took names the command");
    assert!(rig.has("Rate 1: 1"), "a free fixed row is plain");
}

#[trial]
fn import_paths_make_no_areas_or_groups() {
    let pen = Bindable::new("tool.pen", "Pen Tool", Scope::app())
        .with_kind(Kind::Tool { group: None })
        .with_import_path(vec!["Tools".into(), "Pen Tool".into()])
        .with_default(Binding::parse("P").expect("a key"));
    let mut rig = Rig::new(Keymap::new(vec![pen], Scopes::new(), Platform::Mac), Options::default());
    rig.click("Show:");
    assert!(rig.has("Tools") && !rig.has("File Menu"), "an import path makes no menu area");
    assert!(rig.has("Pen Tool: P"));
}
