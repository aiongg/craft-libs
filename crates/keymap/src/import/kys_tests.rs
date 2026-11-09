//! Photoshop .kys sets.

use std::collections::BTreeMap;

use serde_json::Value;

use super::kys::{self, KysError, MAX_BYTES};
use crate::chord::Platform;
use crate::import::{ForeignSet, ImportOptions, Start, UnmatchedReason, plan};
use crate::keymap::{Binding, CommandDef, CommandKind, Keymap};
use crate::scope::{Scope, Scopes};
use crate::{CommandRef, Trigger};

fn binding(text: &str) -> Binding {
    Binding::parse(text).unwrap_or_else(|e| panic!("{text}: {e}"))
}

fn trigger(text: &str) -> Trigger {
    binding(text).trigger
}

fn command(id: &str, label: &str, defaults: &[&str]) -> CommandDef {
    defaults.iter().fold(CommandDef::new(id, label, Scope::app()), |b, d| b.with_default(binding(d)))
}

fn keymap(catalog: Vec<CommandDef>) -> Keymap {
    Keymap::new(catalog, Scopes::new(), Platform::Mac)
}

fn keys(entry: &crate::ForeignEntry) -> Vec<Trigger> {
    entry.keys.iter().map(|k| k.trigger.clone().unwrap_or_else(|e| panic!("{}: {e}", k.text))).collect()
}

fn set(rows: &str, filename: &str) -> String {
    format!("<photoshop-keyboard-shortcuts version=\"4\" filename=\"{filename}\">{rows}</photoshop-keyboard-shortcuts>")
}

#[test]
// A Mac set's Control+Opt+Cmd+G and a Windows set's Ctrl+Alt+G read as the same chord with Cmd; the set name comes from the filename attribute.
fn reads_a_mac_and_a_windows_set() {
    let mac = kys::read(&set(
        "<command kind=\"dynamic\" name=\"Gaussian Blur...\"><shortcut>Opt+Cmd+G</shortcut></command>\
         <command name=\"Rulers\"><shortcut>Control+K</shortcut></command>\
         <command name=\"No Key\"/>\
         <command><shortcut>Cmd+Q</shortcut></command>\
         <tool name=\"Move Tool\">V</tool>",
        "$$$/FileName/Presets/KeyboardCustomization/Retouch 1/2=Retouch 1/2",
    ))
    .unwrap();
    assert_eq!(mac.name.as_deref(), Some("Retouch 1/2"));
    let names: Vec<&str> = mac.entries.iter().map(|e| e.path[0].as_str()).collect();
    assert_eq!(
        names,
        ["Gaussian Blur...", "Rulers", "No Key", "Move Tool"],
        "a row without a name is skipped, one without a key stays, tools come last"
    );
    assert_eq!(keys(&mac.entries[0]), [trigger("Cmd+Alt+G")]);
    assert_eq!(keys(&mac.entries[1]), [trigger("Ctrl+K")], "a Mac set's Control is the Control key");
    assert!(mac.entries[2].keys.is_empty());
    assert!(mac.entries[3].is_tool);

    let windows = kys::read(&set(
        "<command name=\"Gaussian Blur...\"><shortcut>Ctrl+Alt+G</shortcut></command><command name=\"Copy\"><shortcut>F3</shortcut></command>",
        "",
    ))
    .unwrap();
    assert_eq!(windows.name, None);
    assert_eq!(keys(&windows.entries[0]), [trigger("Cmd+Alt+G")]);

    assert_eq!(kys::read("<other/>").unwrap_err(), KysError::NotASet("no <photoshop-keyboard-shortcuts> root".into()));
    assert!(matches!(kys::read("<a>"), Err(KysError::NotASet(_))));
    assert_eq!(kys::read(&" ".repeat(MAX_BYTES + 1)).unwrap_err(), KysError::TooLarge);
    // roxmltree recurses once per level: a deep set is refused before it can overflow the stack.
    let deep = set(&format!("{}{}", "<a>".repeat(20_000), "</a>".repeat(20_000)), "");
    let refused = std::thread::Builder::new().stack_size(512 * 1024).spawn(move || kys::read(&deep).err()).unwrap().join().unwrap();
    assert!(matches!(refused, Some(KysError::NotASet(m)) if m.contains("nested")));
}

#[test]
// A `>` inside a quoted attribute of a self-closing tag does not end the tag early, so 100 such
// rows are not counted as 100 levels of nesting.
fn a_greater_than_in_an_attribute_is_not_nesting() {
    let rows = "<command name=\"A > B\" x='>'/>".repeat(100);
    assert!(kys::read(&set(&rows, "")).is_ok());
}

#[test]
// write then read gives every bound command with its id and keys, tools as tool entries.
fn written_sets_read_back_exactly() {
    let km = keymap(vec![
        command("edit.copy", "Copy", &["Cmd+C"]),
        command("view.zoomIn", "Zoom In", &["Cmd+=", "Cmd++"]),
        command("image.levels", "Levels…", &["Cmd+Alt+Shift+Ctrl+L"]),
        command("odd.name", "A <\"weird\"> & 'name'", &["F9"]),
        command("none.bound", "Unbound", &[]),
        command("only.double", "Double", &[]).with_default(Binding::new(Trigger::double("F5".parse().unwrap()))),
        command("only.scoped", "Scoped", &[]).with_default(Binding::parse("Scoped: F6").unwrap()),
        CommandDef::new("tools.move", "Move Tool", Scope::app()).with_kind(CommandKind::Tool { group: None }).with_default(binding("V")),
    ]);
    let xml = kys::write(" My & Set ", &km);
    assert!(xml.contains("KeyboardCustomization/My &amp; Set=My &amp; Set"), "{xml}");
    assert!(xml.contains("name=\"Levels...\" craft-id=\"image.levels\"><shortcut>Control+Opt+Shift+Cmd+L</shortcut>"), "{xml}");
    for absent in ["Unbound", "Double", "Scoped"] {
        assert!(!xml.contains(absent), "{absent} has no key Photoshop can express");
    }
    assert!(xml.find("Move Tool").unwrap() > xml.find("Levels").unwrap(), "tools after commands");

    let read = kys::read(&xml).unwrap();
    assert_eq!(read.name.as_deref(), Some("My & Set"));
    let rows: Vec<(String, Vec<Trigger>, bool)> = read.entries.iter().map(|e| (e.id.clone().unwrap(), keys(e), e.is_tool)).collect();
    assert_eq!(
        rows,
        [
            ("edit.copy".to_owned(), vec![trigger("Cmd+C")], false),
            ("view.zoomIn".to_owned(), vec![trigger("Cmd+="), trigger("Cmd++")], false),
            ("image.levels".to_owned(), vec![trigger("Cmd+Alt+Shift+Ctrl+L")], false),
            ("odd.name".to_owned(), vec![trigger("F9")], false),
            ("tools.move".to_owned(), vec![trigger("V")], true),
        ]
    );
    assert_eq!(read.entries[3].path, ["A <\"weird\"> & 'name'"]);

    // The ids make the import exact: a keymap whose labels differ still gets every key back.
    let mut renamed = keymap(vec![
        command("edit.copy", "Kopieren", &[]),
        command("view.zoomIn", "Hineinzoomen", &[]),
        command("image.levels", "Tonwertkorrektur", &[]),
        command("odd.name", "Seltsam", &[]),
        CommandDef::new("tools.move", "Verschieben", Scope::app()).with_kind(CommandKind::Tool { group: None }),
    ]);
    let options = ImportOptions { start: Start::Defaults, ..ImportOptions::default() };
    plan(&read, &renamed, &options).apply(&mut renamed);
    for (id, want, _) in &rows {
        let got: Vec<Trigger> = renamed.bindings(&CommandRef::new(id.as_str())).iter().map(|b| b.trigger).collect();
        assert_eq!(&got, want, "{id}");
    }
}

fn texts(value: &Value) -> Vec<String> {
    value.as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_owned()).collect()
}

/// PhotoCraft's `engine/src/kys.rs` `plan`/`plan_tools` run on `pc_plan.json`'s sets (the fixture
/// records what they returned, with the candidates and tool names they were given) against
/// [`plan`] on a keymap of the same commands: the same commands get the same keys, alternates
/// included, and the same rows come back unknown or unreadable. The sets in `differs` are where
/// the two deliberately part ways (a label shared by several commands).
#[test]
fn matches_photocrafts_plan_on_its_test_sets() {
    let fixture: Value = serde_json::from_str(include_str!("kys_fixtures/pc_plan.json")).unwrap();
    let catalog = || {
        let commands = fixture["candidates"].as_array().unwrap().iter().map(|c| {
            let default: Vec<&str> = c["default"].as_str().into_iter().collect();
            command(c["id"].as_str().unwrap(), c["label"].as_str().unwrap().trim_end_matches('…'), &default)
        });
        let tools = fixture["tools"].as_array().unwrap().iter().map(|t| {
            CommandDef::new(t["id"].as_str().unwrap(), t["label"].as_str().unwrap(), Scope::app()).with_kind(CommandKind::Tool { group: None })
        });
        keymap(commands.chain(tools).collect())
    };
    let mut ran = 0;
    for s in fixture["sets"].as_array().unwrap() {
        let name = s["name"].as_str().unwrap();
        let pc = &s["pc"];
        let read: ForeignSet = kys::read(s["xml"].as_str().unwrap()).unwrap();
        assert_eq!(read.name.as_deref(), pc["set_name"].as_str(), "{name}");
        let mut km = catalog();
        let planned = plan(&read, &km, &ImportOptions::default());
        let report = planned.apply(&mut km);
        if let Some(why) = s["differs"].as_str() {
            assert!(!why.is_empty(), "{name}: say why");
            assert!(report.unmatched.iter().any(|u| u.reason == UnmatchedReason::Ambiguous), "{name}: reported as ambiguous");
            continue;
        }
        ran += 1;

        // Commands: PhotoCraft's main key then its alternates, in the order the keymap holds them.
        let alternates = pc["alternate_keys"].as_object().unwrap();
        for (id, main) in pc["set"].as_object().unwrap() {
            let mut want = vec![trigger(main.as_str().unwrap())];
            want.extend(texts(&alternates[id]).iter().map(|k| trigger(k)));
            let mut got: Vec<Trigger> = km.bindings(&CommandRef::new(id.as_str())).iter().map(|b| b.trigger).collect();
            want.sort();
            got.sort();
            assert_eq!(got, want, "{name}: {id}");
        }
        let mut unmatched: Vec<String> = report.unmatched.iter().map(|u| u.item.clone()).collect();
        unmatched.sort();
        let mut want_unknown = texts(&pc["unknown"]);
        want_unknown.extend(texts(&pc["tool_unknown"]));
        want_unknown.sort();
        assert_eq!(unmatched, want_unknown, "{name}: unmatched rows");
        let mut unreadable: Vec<String> = report.unreadable.iter().map(|u| u.item.clone()).collect();
        unreadable.sort();
        let mut want_unreadable = texts(&pc["unreadable"]);
        // A tool key with modifiers: PhotoCraft skips it silently, plan reports it unreadable.
        want_unreadable.extend(read.entries.iter().filter(|e| e.is_tool && e.keys.iter().any(|k| k.trigger.is_err())).map(|e| e.path[0].clone()));
        want_unreadable.sort();
        assert_eq!(unreadable, want_unreadable, "{name}: unreadable rows");

        // Tools: PhotoCraft takes a key without modifiers.
        let tool_keys: BTreeMap<&String, &Value> = pc["tool_keys"].as_object().unwrap().iter().collect();
        for (id, key) in tool_keys {
            let got: Vec<Trigger> = km.bindings(&CommandRef::new(id.as_str())).iter().map(|b| b.trigger).collect();
            assert_eq!(got, [trigger(key.as_str().unwrap())], "{name}: tool {id}");
        }
    }
    assert!(ran >= 8, "the fixture's comparable sets ran: {ran}");
}
