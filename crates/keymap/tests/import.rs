//! Import of synthetic shortcut sets written for these tests.

use std::collections::BTreeMap;

use craft_keymap::chord::foreign::ReadNote;
use craft_keymap::chord::foreign::{Flavor, ForeignError};
use craft_keymap::import::{
    Failed, FailedChange, MAX_IMPORT_ENTRIES, MAX_IMPORT_TEXT, MAX_KEYS_PER_ENTRY, MAX_SUGGESTIONS, NoChangeReason, SkipReason, UnmatchedReason,
};
use craft_keymap::{
    Bindable, Binding, CommandRef, ContextRule, ForeignEntry, ForeignKey, ForeignSet, ImportOptions, ImportReport, Keymap, Kind, MAX_COMMANDS,
    Overrides, Platform, Scope, Scopes, Start, plan, reserved,
};
use serde_json::json;

fn b(text: &str) -> Binding {
    Binding::parse(text).unwrap_or_else(|e| panic!("{text}: {e}"))
}

fn bindings(texts: &[&str]) -> Vec<Binding> {
    texts.iter().map(|t| b(t)).collect()
}

fn command(id: impl Into<CommandRef>, label: &str, path: &[&str], keys: &[&str]) -> Bindable {
    let mut out = Bindable::new(id, label, Scope::app());
    if !path.is_empty() {
        out = out.with_path(path.iter().map(|s| s.to_string()).collect());
    }
    keys.iter().fold(out, |e, k| e.with_default(b(k)))
}

fn panel(name: &str) -> CommandRef {
    CommandRef::with_params("window.panel", json!({ "panel": name }))
}

fn catalog() -> Vec<Bindable> {
    vec![
        command("file.new", "Document…", &["File", "New", "Document…"], &["Cmd+N"]),
        command("dialog.newDocument", "New Document Dialog", &[], &["Cmd+N"]),
        command("file.close", "Close", &["File", "Close"], &["Cmd+W"]),
        command("file.save", "Save", &["File", "Save"], &["Cmd+S"]),
        command("file.exit", "Exit", &["File", "Exit"], &[]),
        command("edit.paste", "Paste", &["Edit", "Paste"], &["Cmd+V"]),
        command("edit.clear", "Clear", &["Edit", "Clear"], &[]),
        command("edit.duplicate", "Duplicate", &["Edit", "Duplicate"], &["Cmd+Alt+Shift+D"]),
        command("edit.checkSpelling", "Check Spelling…", &["Edit", "Proofing", "Check Spelling…"], &[]),
        command("edit.pen", "Pen", &["Edit", "Preferences", "Pen"], &[]),
        command("app.palette", "Command Palette", &[], &["Cmd+Alt+K"]),
        command(panel("layers"), "Layers", &["Window", "Layers"], &["F7"]),
        command(panel("pages"), "Pages", &["Window", "Pages"], &["F12"]),
        command("view.guides", "Guides", &["View", "Grids & Guides", "Guides"], &["Cmd+;"]).toggle(),
        command("view.goToPage", "Go to Page…", &["View", "Go to Page…"], &["Cmd+J"]),
        command("view.zoomIn", "Zoom In", &["View", "Zoom In"], &["Cmd+="]),
        command("object.textFrameOptions", "Text Frame Options…", &["Object", "Text Frame Options…"], &["Cmd+B"]),
        command("type.break", "Break", &["Type", "Breaks", "Break"], &[]),
        command("type.columnBreak", "Insert Break", &["Type", "Column", "Insert Break"], &[]),
        command("type.frameBreak", "Insert Break", &["Type", "Frame", "Insert Break"], &[]),
        command("type.bold", "Bold", &["Type", "Style", "Bold"], &[]),
        command("type.italic", "Italic", &["Type", "Style", "Italic"], &[]),
        command("tool.pen", "Pen Tool", &[], &["P"]).with_kind(Kind::Tool { group: None }).with_alias("Pen"),
        command("tool.type", "Type Tool", &[], &["T"]).with_kind(Kind::Tool { group: None }),
        command("scripts.run", "Run Script", &[], &[]),
    ]
}

fn keymap(platform: Platform) -> Keymap {
    Keymap::new(catalog(), Scopes::new().with_context("Text"), platform).with_reserved(reserved::system(platform))
}

fn key(flavor: Flavor, context: &str, text: &str) -> ForeignKey {
    ForeignKey::read(Some(context), text, flavor)
}

fn entry(menu: Option<&str>, path: &[&str], keys: &[(&str, &str)], flavor: Flavor) -> ForeignEntry {
    keys.iter().fold(ForeignEntry::new(menu, path.iter().copied()), |e, (c, t)| e.with_key(key(flavor, c, t)))
}

fn win(menu: &str, path: &[&str], keys: &[(&str, &str)]) -> ForeignEntry {
    entry(Some(menu), path, keys, Flavor::Windows)
}

/// An InDesign-like Windows set, one entry per match rule or apply rule.
fn windows_set() -> ForeignSet {
    let d = "Default";
    let entries = vec![
        win("File", &["New", "Document..."], &[(d, "Ctrl+N")]),
        win("File", &["Close"], &[(d, "Ctrl+W"), (d, "Ctrl+F4")]),
        win("File", &["Save"], &[(d, "Ctrl+Hyperspace")]),
        win("File", &["Exit"], &[(d, "Alt+F4")]),
        win("Edit", &["Paste"], &[(d, "Ctrl+E")]),
        win("Edit", &["Clear"], &[(d, "Ctrl+E")]),
        win("Edit", &["Duplicate"], &[]),
        win("Edit", &["Duplicate"], &[]),
        win("Edit", &["Spelling", "Check Spelling..."], &[(d, "Ctrl+Alt+K"), ("Text", "F7")]),
        win("Edit", &["Frobnicate Everything"], &[(d, "Ctrl+F12")]),
        win("Edit", &["Never Heard Of It"], &[]),
        win("Window", &["Layers"], &[(d, "F7")]),
        win("Window", &["Pages"], &[(d, "Shift+F12")]),
        win("View", &["Grids & Guides", "SHOW/HIDE   Guides"], &[(d, "Ctrl+Alt+;")]),
        win("Layout", &["Go To Page"], &[(d, "Ctrl+Alt+J")]),
        win("Object", &["Text Frame", "Options"], &[(d, "Ctrl+Alt+B")]),
        win("Type", &["Insert", "Break"], &[(d, "Ctrl+Alt+Enter")]),
        win("Type", &["Insert Break"], &[(d, "Shift+Enter")]),
        win("Type", &["Style", "Bold"], &[("Text", "Shift+Ctrl+B")]),
        win("Type", &["Style", "Italic"], &[(d, "Ctrl+I"), ("Table", "Ctrl+Alt+I")]),
        win("Tools", &["Pen"], &[(d, "Shift+P")]).tool(),
        win("Tools", &["Zoom In"], &[(d, "Z")]).tool(),
        entry(None, &["Zoom In"], &[(d, "Ctrl+Alt+I")], Flavor::Windows),
        entry(None, &["Somewhere", "Zoom In"], &[(d, "Ctrl+F11")], Flavor::Windows),
        win("Scripts", &["Frobnicate"], &[(d, "Ctrl+Alt+R")]),
    ];
    ForeignSet::new(entries).with_name("My Set")
}

fn windows_options() -> ImportOptions {
    let mut remembered = BTreeMap::new();
    remembered.insert(ForeignEntry::new(Some("Scripts"), ["Frobnicate"]).remember_key(), CommandRef::new("scripts.run"));
    ImportOptions {
        contexts: vec![("default".into(), ContextRule::Home), ("TEXT".into(), ContextRule::HomeWithoutHomeEntry)],
        remembered,
        ..ImportOptions::default()
    }
}

fn import_windows() -> (Keymap, ImportReport) {
    let mut km = keymap(Platform::Windows);
    let report = plan(&windows_set(), &km, &windows_options()).apply(&mut km);
    (km, report)
}

fn has(km: &Keymap, id: impl Into<CommandRef>, texts: &[&str]) {
    let c = id.into();
    assert_eq!(km.bindings(&c), bindings(texts), "{}", c.key());
}

#[test]
fn every_match_step_finds_its_command() {
    let (km, r) = import_windows();
    assert_eq!(r.set.as_deref(), Some("My Set"));
    // 1: full path, with `&`, case and spacing normalized, and Show/Hide dropped for a toggle.
    has(&km, "view.guides", &["Cmd+Alt+;"]);
    // 2: joined segments `Text Frame Options` in Object.
    has(&km, "object.textFrameOptions", &["Cmd+Alt+B"]);
    // 2 is ambiguous (two `Insert Break` in Type), 3 finds `Break`.
    has(&km, "type.break", &["Cmd+Alt+Enter"]);
    // 3: last segment in the menu, under another submenu.
    has(&km, "edit.checkSpelling", &["Cmd+Alt+K"]);
    // 4: the label anywhere; also with no menu for a one-segment path.
    has(&km, "view.goToPage", &["Cmd+Alt+J"]);
    has(&km, "view.zoomIn", &["Cmd+Alt+I"]);
    // 0: a remembered answer.
    has(&km, "scripts.run", &["Cmd+Alt+R"]);
    // A command with parameters, by its path.
    has(&km, panel("pages"), &["Shift+F12"]);
    // A tool entry matches the tool, not the command with the same label.
    has(&km, "tool.pen", &["Shift+P"]);
    has(&km, "edit.pen", &[]);

    let unmatched: Vec<(&str, UnmatchedReason)> = r.unmatched.iter().map(|u| (u.item.as_str(), u.reason)).collect();
    assert_eq!(
        unmatched,
        [
            ("Edit > Frobnicate Everything", UnmatchedReason::NoMatch),
            ("Type > Insert Break", UnmatchedReason::Ambiguous),
            ("Type > Style > Italic", UnmatchedReason::Context),
            ("Tools > Zoom In", UnmatchedReason::Tool),
            ("Somewhere > Zoom In", UnmatchedReason::NoMatch),
        ]
    );
    assert_eq!(r.unmatched[2].keys, "Table: Ctrl+Alt+I");
    assert_eq!(r.unmatched_without_shortcut, 1);
    assert_eq!(r.unreadable.len(), 1);
    assert_eq!(r.unreadable[0].item, "File > Save");
    assert_eq!(r.unreadable[0].keys, "Default: Ctrl+Hyperspace");
    assert!(matches!(r.unreadable[0].reason, ForeignError::UnknownKey(_)));
    assert!(r.suggestions.is_empty(), "not asked for");
    assert_eq!(r.truncated, 0);
}

#[test]
fn the_set_wins_and_the_report_says_how() {
    let (km, r) = import_windows();
    // Two keys for one command, both kept in order.
    has(&km, "file.close", &["Cmd+W", "Cmd+F4"]);
    // The Text context counts only without a Default key.
    has(&km, "type.bold", &["Cmd+Shift+B"]);
    has(&km, "type.italic", &["Cmd+I"]);
    has(&km, panel("layers"), &["F7"]);
    // A later entry takes a chord from an earlier one; the earlier command keeps what it had.
    has(&km, "edit.paste", &["Cmd+V"]);
    has(&km, "edit.clear", &["Cmd+E"]);
    let skipped: Vec<_> = r.skipped.iter().map(|s| (s.command.id.as_str(), s.shortcut.as_str(), s.kept_by.clone(), s.reason)).collect();
    assert_eq!(
        skipped,
        [("file.exit", "Alt+F4", None, SkipReason::Reserved), ("edit.paste", "Cmd+E", Some(CommandRef::new("edit.clear")), SkipReason::Taken)]
    );
    // Taken from a command outside the set; the twin keeps its shared default.
    has(&km, "app.palette", &[]);
    has(&km, "dialog.newDocument", &["Cmd+N"]);
    let cleared: Vec<_> = r.conflicts_cleared.iter().map(|c| (c.command.id.as_str(), c.label.as_str(), c.shortcut.as_str())).collect();
    assert_eq!(cleared, [("app.palette", "Command Palette", "Cmd+Alt+K")]);
    // None defined clears; an unreadable key leaves the command as it was.
    has(&km, "edit.duplicate", &[]);
    has(&km, "file.save", &["Cmd+S"]);
    // Bindings equal to the defaults leave no override.
    let o = km.overrides();
    assert_eq!(o.get("file.new"), None);
    assert_eq!(o.get(&panel("layers").key()), None);
    assert_eq!(o.get("edit.duplicate"), Some(&[][..]));
    assert_eq!(o.get("file.close"), Some(&["Cmd+W".to_string(), "Cmd+F4".to_string()][..]));

    let changed: Vec<&str> = r.changed.iter().map(|c| c.command.id.as_str()).collect();
    assert_eq!(
        changed,
        [
            "file.close",
            "edit.clear",
            "edit.duplicate",
            "edit.checkSpelling",
            "window.panel",
            "view.guides",
            "view.goToPage",
            "object.textFrameOptions",
            "type.break",
            "type.bold",
            "type.italic",
            "tool.pen",
            "view.zoomIn",
            "scripts.run",
        ]
    );
    assert_eq!(r.imported, changed.len());
    let close = &r.changed[0];
    assert_eq!((close.label.as_str(), close.item.as_str()), ("Close", "File > Close"));
    assert_eq!((close.from.clone(), close.to.clone()), (vec!["Cmd+W".to_string()], vec!["Cmd+W".to_string(), "Cmd+F4".to_string()]));
}

#[test]
fn a_failed_change_keeps_its_shortcut_when_a_later_change_frees_room() {
    // Save's change finds the overrides full. Close's change then frees an entry, and Zoom In
    // takes Cmd+S. Removing Cmd+S from Save would fit now, but Save's import failed, so Save
    // keeps it and Zoom In stays as it was.
    let set = ForeignSet::new(vec![
        win("File", &["Save"], &[("Default", "Ctrl+Alt+S")]),
        win("File", &["Close"], &[("Default", "Ctrl+W")]),
        win("View", &["Zoom In"], &[("Default", "Ctrl+S")]),
    ]);
    let mut o = Overrides::new();
    o.insert("file.close", vec!["Cmd+Q".into()]);
    o.insert("view.zoomIn", vec!["Cmd+Alt+Z".into()]);
    for n in 2..MAX_COMMANDS {
        o.insert(format!("junk.{n}"), vec!["Cmd+K".into()]);
    }
    assert_eq!(o.len(), MAX_COMMANDS);
    let options = ImportOptions { contexts: vec![("Default".into(), ContextRule::Home)], ..ImportOptions::default() };
    let mut km = keymap(Platform::Windows).with_overrides(o);
    let r = plan(&set, &km, &options).apply(&mut km);
    has(&km, "file.save", &["Cmd+S"]);
    has(&km, "file.close", &["Cmd+W"]);
    has(&km, "view.zoomIn", &["Cmd+Alt+Z"]);
    let failed: Vec<(&str, FailedChange)> = r.failed.iter().map(|f| (f.command.id.as_str(), f.change)).collect();
    assert_eq!(failed, [("file.save", FailedChange::Bind), ("view.zoomIn", FailedChange::Bind)]);
    assert!(r.conflicts_cleared.is_empty(), "{:?}", r.conflicts_cleared);
    assert_eq!(r.imported, 1);
}

#[test]
fn changes_that_find_the_overrides_full_are_reported() {
    let set = ForeignSet::new(vec![
        win("File", &["Close"], &[("Default", "Ctrl+J")]),
        win("File", &["Save"], &[("Default", "Ctrl+Alt+S")]),
        win("Edit", &["Duplicate"], &[]),
    ]);
    let full = || {
        let mut o = Overrides::new();
        o.insert("file.close", vec!["Cmd+Q".into()]);
        for n in 1..MAX_COMMANDS {
            o.insert(format!("junk.{n}"), vec!["Cmd+K".into()]);
        }
        assert_eq!(o.len(), MAX_COMMANDS);
        o
    };
    let options = |start| ImportOptions { contexts: vec![("Default".into(), ContextRule::Home)], start, ..ImportOptions::default() };

    // file.close has an entry to rewrite, but taking Cmd+J from view.goToPage needs a new one, so
    // both stay as they were. The other changes each need a new entry too.
    let mut km = keymap(Platform::Windows).with_overrides(full());
    let r = plan(&set, &km, &options(Start::Current)).apply(&mut km);
    has(&km, "file.close", &["Cmd+Q"]);
    has(&km, "view.goToPage", &["Cmd+J"]);
    has(&km, "file.save", &["Cmd+S"]);
    has(&km, "edit.duplicate", &["Cmd+Alt+Shift+D"]);
    let rows = |failed: &[Failed]| -> Vec<(String, String, FailedChange, Vec<String>)> {
        failed.iter().map(|f| (f.command.id.clone(), f.label.clone(), f.change, f.shortcuts.clone())).collect()
    };
    let row = |id: &str, label: &str, change, shortcuts: &[&str]| {
        (id.to_string(), label.to_string(), change, shortcuts.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    };
    assert_eq!(
        rows(&r.failed),
        [
            row("file.close", "Close", FailedChange::Bind, &["Cmd+J"]),
            row("file.save", "Save", FailedChange::Bind, &["Cmd+Alt+S"]),
            row("edit.duplicate", "Duplicate", FailedChange::Clear, &[]),
        ]
    );
    assert_eq!(r.imported, 0);
    assert!(r.conflicts_cleared.is_empty());
    let v = serde_json::to_value(&r).unwrap();
    assert_eq!(
        v["failed"][1],
        json!({"command": {"id": "file.save", "params": null}, "label": "Save", "change": "bind", "shortcuts": ["Cmd+Alt+S"]})
    );

    // Starting from the defaults drops the entries, so everything fits.
    let mut km = keymap(Platform::Windows).with_overrides(full());
    let r = plan(&set, &km, &options(Start::Defaults)).apply(&mut km);
    assert!(r.failed.is_empty(), "{:?}", r.failed);
    has(&km, "file.close", &["Cmd+J"]);
    has(&km, "view.goToPage", &[]);
    has(&km, "file.save", &["Cmd+Alt+S"]);
    has(&km, "edit.duplicate", &[]);

    // A later entry whose change finds the overrides full leaves the chord with the earlier one.
    let set = ForeignSet::new(vec![win("File", &["Close"], &[("Default", "Ctrl+Alt+J")]), win("File", &["Save"], &[("Default", "Ctrl+Alt+J")])]);
    let mut km = keymap(Platform::Windows).with_overrides(full());
    let r = plan(&set, &km, &options(Start::Current)).apply(&mut km);
    has(&km, "file.close", &["Cmd+Alt+J"]);
    has(&km, "file.save", &["Cmd+S"]);
    assert_eq!(rows(&r.failed), [row("file.save", "Save", FailedChange::Bind, &["Cmd+Alt+J"])]);
    assert!(r.skipped.is_empty(), "{:?}", r.skipped);
    assert_eq!(r.imported, 1);
}

#[test]
fn hostile_sets_are_capped_and_counted() {
    let mut km = keymap(Platform::Windows);
    let long: String = "ä€𝄞 x".repeat(MAX_IMPORT_TEXT);
    let mut big = ForeignEntry::new(Some(long.as_str()), std::iter::repeat_n(long.clone(), 100));
    for _ in 0..MAX_KEYS_PER_ENTRY + 10 {
        let trigger = Err(ForeignError::UnknownKey(long.clone()));
        big = big.with_key(ForeignKey { context: Some(long.clone()), text: long.clone(), trigger, notes: vec![ReadNote::Numpad; 1000] });
    }
    // Unmatched entries with keys, more than the suggestions take.
    let near = win("Edit", &["Chek Spelling Now"], &[("Default", "Ctrl+F12")]);
    let mut entries: Vec<ForeignEntry> = std::iter::repeat_n(near, MAX_SUGGESTIONS + 10).collect();
    entries.push(big);
    entries.push(win("Edit", &["Paste"], &[("Default", &long), ("Default", "Ctrl+\u{0}"), ("Default", "Win+E")]));
    // Entries without keys fill the set to the cap and 500 past it.
    let keyless = MAX_IMPORT_ENTRIES - entries.len();
    entries.extend(std::iter::repeat_n(ForeignEntry::new(None, Vec::<String>::new()), keyless + 500));
    let set = ForeignSet::new(entries).with_name(long.clone());
    let options = ImportOptions { contexts: vec![("Default".into(), ContextRule::Home)], suggest: true, ..ImportOptions::default() };
    let r = plan(&set, &km, &options).apply(&mut km);
    assert_eq!(r.suggestions.len(), MAX_SUGGESTIONS);
    // The set name, entries and keys past the caps, and suggestions past the cap (with the big entry's).
    assert!(r.truncated >= 1 + 500 + 10 + 11, "{}", r.truncated);
    assert_eq!(r.set.as_ref().map(|s| s.chars().count()), Some(MAX_IMPORT_TEXT));
    let row = r.unmatched.iter().find(|u| u.item.starts_with('ä')).unwrap();
    assert!(row.item.chars().count() <= MAX_IMPORT_TEXT * 34, "item of capped segments");
    assert!(row.keys.chars().count() <= (2 * MAX_IMPORT_TEXT + 4) * MAX_KEYS_PER_ENTRY);
    assert_eq!(r.unreadable.len(), 1, "Paste's keys");
    has(&km, "edit.paste", &["Cmd+V"]);
    // Entries past the cap are left out.
    assert_eq!(r.unmatched_without_shortcut, keyless);
}

fn import_into(catalog: Vec<Bindable>, entries: Vec<ForeignEntry>, options: &ImportOptions) -> (Keymap, ImportReport) {
    let mut km = Keymap::new(catalog, Scopes::new(), Platform::Windows);
    let report = plan(&ForeignSet::new(entries), &km, options).apply(&mut km);
    (km, report)
}

fn default_context() -> ImportOptions {
    ImportOptions { contexts: vec![("Default".into(), ContextRule::Home)], ..ImportOptions::default() }
}

fn unmatched(r: &ImportReport) -> Vec<(&str, UnmatchedReason)> {
    r.unmatched.iter().map(|u| (u.item.as_str(), u.reason)).collect()
}

#[test]
fn panel_menu_entries_match_by_path_end_or_label() {
    let catalog = vec![
        command("type.allCaps", "All Caps", &["Panels", "Character", "All Caps"], &[]),
        command("pages.overrideAll", "Override All Parent Items", &["Pages", "Override All Parent Page Items"], &[]),
        command("paragraph.justification", "Justification…", &[], &[]),
        command("swatches.new", "New Swatch", &["Swatches", "New Swatch"], &[]),
        command("gradient.new", "New Swatch", &["Gradient", "New Swatch"], &[]),
        command("character.smallCaps", "Small Caps", &[], &[]).with_alias("Character: Small Caps"),
        // The app has Paragraph and Color panels.
        command(panel("paragraph"), "Paragraph", &["Window", "Paragraph"], &[]),
        command(panel("color"), "Color", &["Window", "Color"], &[]),
    ];
    let d = "Default";
    let panel_entry = |path: &[&str], key: &str| entry(None, path, &[(d, key)], Flavor::Windows).in_panel_menu();
    let entries = vec![
        // The end of a path, label last.
        panel_entry(&["Character", "All Caps"], "Ctrl+F1"),
        // The end of a path whose last segment differs from the label.
        panel_entry(&["Pages", "Override All Parent Page Items"], "Ctrl+F2"),
        // The last segment alone, as one command's label, for a panel the app has.
        panel_entry(&["Paragraph", "Justification..."], "Ctrl+F3"),
        // The last segment alone matches nothing for a panel the app lacks.
        panel_entry(&["History", "Justification..."], "Ctrl+F8"),
        // The panel picks one of two commands with the label.
        panel_entry(&["Swatches", "New Swatch"], "Ctrl+F4"),
        // The joined segments as an alias.
        panel_entry(&["Character", "Small Caps"], "Ctrl+F5"),
        // No path end fits and the label alone fits two commands.
        panel_entry(&["Color", "New Swatch"], "Ctrl+F6"),
        // Without the mark, a path with no menu matches only a whole catalog path.
        entry(None, &["Character", "All Caps"], &[(d, "Ctrl+F7")], Flavor::Windows),
    ];
    let (km, r) = import_into(catalog, entries, &default_context());
    has(&km, "type.allCaps", &["Cmd+F1"]);
    has(&km, "pages.overrideAll", &["Cmd+F2"]);
    has(&km, "paragraph.justification", &["Cmd+F3"]);
    has(&km, "swatches.new", &["Cmd+F4"]);
    has(&km, "character.smallCaps", &["Cmd+F5"]);
    has(&km, "gradient.new", &[]);
    assert_eq!(
        unmatched(&r),
        [
            ("History > Justification...", UnmatchedReason::NoMatch),
            ("Color > New Swatch", UnmatchedReason::Ambiguous),
            ("Character > All Caps", UnmatchedReason::NoMatch)
        ]
    );
}

#[test]
fn show_and_hide_are_read_off_only_for_toggles() {
    let catalog = vec![
        command(panel("conveyor"), "Conveyor", &["Window", "Conveyor"], &[]),
        command("view.rulers", "Rulers", &["View", "Rulers"], &[]).toggle(),
        command("view.frameEdges", "Show Frame Edges", &["View", "Extras", "Show Frame Edges"], &[]).toggle(),
        command("view.hiddenCharacters", "Show Hidden Characters", &["Type", "Show Hidden Characters"], &[]),
    ];
    let d = "Default";
    let entries = vec![
        // Not a toggle: the name must fit as written.
        win("View", &["Extras", "Hide Conveyor"], &[(d, "Ctrl+F1")]),
        // One toggle for both of the source's items.
        win("View", &["Show Rulers"], &[(d, "Ctrl+R")]),
        win("View", &["Hide Rulers"], &[(d, "Ctrl+Shift+R")]),
        win("View", &["Extras", "Hide Frame Edges"], &[(d, "Ctrl+H")]),
        // Exact names match any command.
        win("Type", &["Show Hidden Characters"], &[(d, "Ctrl+Alt+I")]),
        win("Type", &["Hide Hidden Characters"], &[(d, "Ctrl+Alt+H")]),
    ];
    let (km, r) = import_into(catalog, entries, &default_context());
    has(&km, panel("conveyor"), &[]);
    has(&km, "view.rulers", &["Cmd+R", "Cmd+Shift+R"]);
    has(&km, "view.frameEdges", &["Cmd+H"]);
    has(&km, "view.hiddenCharacters", &["Cmd+Alt+I"]);
    assert_eq!(
        unmatched(&r),
        [("View > Extras > Hide Conveyor", UnmatchedReason::NoMatch), ("Type > Hide Hidden Characters", UnmatchedReason::NoMatch)]
    );
}

#[test]
fn only_entries_with_keys_to_bind_count_toward_suggestions() {
    let catalog = vec![command("edit.checkSpelling", "Check Spelling…", &["Edit", "Check Spelling…"], &[])];
    let d = "Default";
    // Twice the cap of unmatched entries without keys, or with keys only in a skipped context,
    // before one with a key.
    let mut entries: Vec<ForeignEntry> = (0..MAX_SUGGESTIONS)
        .flat_map(|i| [win("Edit", &[&format!("Nothing {i}")], &[]), win("Edit", &[&format!("Tables {i}")], &[("Table", "Ctrl+T")])])
        .collect();
    entries.push(win("Edit", &["Chek Spelling Now"], &[(d, "Ctrl+K"), ("Table", "Ctrl+T")]));
    let options = ImportOptions {
        contexts: vec![(d.into(), ContextRule::Home), ("Table".into(), ContextRule::Skip)],
        suggest: true,
        ..ImportOptions::default()
    };
    let (_, r) = import_into(catalog, entries, &options);
    assert_eq!(r.unmatched_without_shortcut, 2 * MAX_SUGGESTIONS);
    assert_eq!(r.unmatched.len(), 1);
    assert_eq!(r.unmatched[0].keys, "Default: Ctrl+K", "keys in a skipped context are left out");
    assert_eq!(r.suggestions.len(), 1);
    assert_eq!(r.suggestions[0].candidates, [CommandRef::new("edit.checkSpelling")]);
    assert_eq!(r.truncated, 0);
}

fn pages_catalog() -> Vec<Bindable> {
    vec![
        command("app.newDocumentDialog", "New Document…", &["File", "New Document…"], &["Cmd+N"]),
        command("layout.pages.insert", "Insert Pages…", &["Layout", "Pages", "Insert Pages…"], &["Cmd+Shift+P"]),
        command("file.close", "Close", &["File", "Close"], &["Cmd+W"]),
    ]
}

#[test]
fn panel_menu_entries_without_keys_change_nothing() {
    let none = |path: &[&str]| ForeignEntry::new(None, path.iter().copied()).in_panel_menu();
    let entries = vec![
        // The app has no History panel, so the last segment alone does not match.
        none(&["History", "New Document"]),
        // A path end matches, but without keys the command keeps its shortcuts.
        none(&["Pages", "Insert Pages..."]),
        // The same for the last segment alone, with a panel the app has.
        none(&["Pages", "New Document"]),
    ];
    let (km, r) = import_into(pages_catalog(), entries, &default_context());
    has(&km, "app.newDocumentDialog", &["Cmd+N"]);
    has(&km, "layout.pages.insert", &["Cmd+Shift+P"]);
    assert!(r.changed.is_empty(), "{:?}", r.changed);
    assert_eq!(r.unmatched_without_shortcut, 1);
    let rows: Vec<(&str, &str, NoChangeReason)> =
        r.matched_without_change.iter().map(|m| (m.item.as_str(), m.command.id.as_str(), m.reason)).collect();
    assert_eq!(
        rows,
        [
            ("Pages > Insert Pages...", "layout.pages.insert", NoChangeReason::PanelMenu),
            ("Pages > New Document", "app.newDocumentDialog", NoChangeReason::PanelMenu)
        ]
    );
    let v = serde_json::to_value(&r).unwrap();
    let mut fields: Vec<String> = v["matchedWithoutChange"][0].as_object().unwrap().keys().cloned().collect();
    fields.sort();
    assert_eq!(fields, ["command", "item", "label", "line", "reason"]);
    assert_eq!(v["matchedWithoutChange"][0]["reason"], "panelMenu");
}

#[test]
fn an_entry_without_keys_never_clears_keys_another_entry_gives() {
    let d = "Default";
    let keyed = || win("Layout", &["Pages", "Insert Pages..."], &[(d, "Ctrl+Alt+P")]);
    let keyless = || win("Layout", &["Insert Pages..."], &[]);
    for entries in [vec![keyed(), keyless()], vec![keyless(), keyed()]] {
        let (km, _) = import_into(pages_catalog(), entries, &default_context());
        has(&km, "layout.pages.insert", &["Cmd+Alt+P"]);
    }

    // A panel-menu entry without keys, and the menu entry with the command's keys.
    let entries = vec![
        ForeignEntry::new(None, ["Pages", "Insert Pages..."]).in_panel_menu(),
        win("Layout", &["Pages", "Insert Pages..."], &[(d, "Ctrl+Shift+P")]),
    ];
    let (km, r) = import_into(pages_catalog(), entries, &default_context());
    has(&km, "layout.pages.insert", &["Cmd+Shift+P"]);
    assert_eq!(r.matched_without_change.len(), 1);

    // The later entry still takes the chord from another command; the entry without keys does
    // not clear the command that lost it.
    let entries = vec![keyless(), keyed(), win("File", &["Close"], &[(d, "Ctrl+Alt+P")])];
    let (km, r) = import_into(pages_catalog(), entries, &default_context());
    has(&km, "file.close", &["Cmd+Alt+P"]);
    has(&km, "layout.pages.insert", &["Cmd+Shift+P"]);
    assert_eq!(r.skipped.iter().map(|s| (s.command.id.as_str(), s.reason)).collect::<Vec<_>>(), [("layout.pages.insert", SkipReason::Taken)]);
}
