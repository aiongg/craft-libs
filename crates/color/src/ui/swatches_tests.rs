//! The Swatches panel on a kittest harness: clicks, New Swatch, the panel menu, and the library.

use craft_cms::convert::CmykConverter;
use std::sync::Arc;

use craft_host::{AppInfo, Picked};
use craft_theme::{ModelValues, Srgba};
use craft_ui::behavior::{DropPosition, Move};
use craft_ui::composites::{StatusLine, Tone};
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Event, Modifiers, PointerButton, Pos2, Ui};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, Node};

use super::lut_picker_tests::Scripted;
use super::swatches::Key;
use super::{PickedColor, SwatchAt, SwatchLibrary, SwatchesPanel, SwatchesPanelResponse};
use crate::palette::{SwatchColor, SwatchEntry, SwatchFormat, SwatchGroup, SwatchNote, SwatchReport};

struct Host {
    library: SwatchLibrary,
    foreground: Srgba,
    used: Vec<(SwatchAt, Srgba, bool)>,
    /// What the panel reported besides `used`, per pass.
    reported: Vec<SwatchesPanelResponse>,
    time: f64,
}

fn rgb(name: &str, c: [f32; 3]) -> SwatchEntry {
    SwatchEntry::new(name, SwatchColor::Rgb(c))
}

/// "Basics" with Red and Color Swatch 1, "Pastels" with Pink.
fn library() -> SwatchLibrary {
    SwatchLibrary {
        groups: vec![
            SwatchGroup { name: "Basics".into(), entries: vec![rgb("Red", [1.0, 0.0, 0.0]), rgb("Color Swatch 1", [0.0, 0.0, 1.0])] },
            SwatchGroup { name: "Pastels".into(), entries: vec![rgb("Pink", [1.0, 0.8, 0.8])] },
        ],
    }
}

fn show(ui: &mut Ui, host: &mut Host) {
    let response = SwatchesPanel::new("swatches", &mut host.library, host.foreground).cmyk(CmykConverter::coated()).show(ui);
    host.used.extend(response.used);
    if response.imported.is_some() || response.exported.is_some() || response.status.is_some() {
        host.reported.push(response);
    }
}

fn harness(library: SwatchLibrary) -> Harness<'static, Host> {
    harness_on(library, None)
}

/// The panel with `files` as the installed host's file service (no host without it).
fn harness_on(library: SwatchLibrary, files: Option<Arc<Scripted>>) -> Harness<'static, Host> {
    let host = Host { library, foreground: Srgba::rgb(0.2, 0.4, 0.6), used: Vec::new(), reported: Vec::new(), time: 0.0 };
    let mut harness = Harness::<Host>::builder()
        .with_size([320.0, 320.0])
        .with_context_setup(move |ctx| {
            install(ctx, &Base::suite_default());
            if let Some(files) = &files {
                craft_host::Host::builder(AppInfo { id: "test", name: "Test", version: "1" }).files(files.clone()).build().install(ctx);
            }
        })
        .build_ui_state(show, host);
    harness.run();
    harness
}

fn center(node: Node<'_>) -> Pos2 {
    node.rect().center()
}

fn cell<'h>(harness: &'h Harness<'_, Host>, label: &str) -> Node<'h> {
    let prefix = format!("{label}, ");
    harness.get_by(move |n| matches!(n.role(), Role::TreeItem | Role::ListBoxOption) && n.label().is_some_and(|l| l.starts_with(&prefix)))
}

fn click(harness: &mut Harness<'_, Host>, pos: Pos2, modifiers: Modifiers) {
    let button = |pressed| Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers };
    harness.state_mut().time += 1.0;
    let time = harness.state().time;
    harness.input_mut().time = Some(time);
    harness.input_mut().events.extend([Event::ModifiersChanged(modifiers), Event::PointerMoved(pos), button(true), button(false)]);
    harness.run_steps(1);
    harness.input_mut().events.push(Event::ModifiersChanged(Modifiers::NONE));
    harness.run_steps(2);
}

/// A click uses the swatch for the foreground, Alt-click for the background.
#[test]
fn click_and_alt_click_use_the_swatch() {
    let mut harness = harness(library());
    let red = center(cell(&harness, "Red"));
    click(&mut harness, red, Modifiers::NONE);
    click(&mut harness, red, Modifiers::ALT);
    let at = SwatchAt { group: 0, index: 0 };
    let color = Srgba::rgb(1.0, 0.0, 0.0);
    assert_eq!(harness.state().used, [(at, color, false), (at, color, true)]);
}

/// New Swatch adds the foreground to the selected swatch's group under a free name.
#[test]
fn new_swatch_adds_the_foreground_with_a_free_name() {
    let mut harness = harness(library());
    let pink = center(cell(&harness, "Pink"));
    click(&mut harness, pink, Modifiers::NONE);
    let new = center(harness.get_by_label("Create new swatch"));
    click(&mut harness, new, Modifiers::NONE);
    let host = harness.state();
    assert_eq!(host.library.groups[0].entries.len(), 2, "the other group is untouched");
    let added = host.library.groups[1].entries.last().expect("the new swatch");
    assert_eq!(added.name, "Color Swatch 2", "Color Swatch 1 is taken");
    assert_eq!(added.color, SwatchColor::Rgb(host.foreground.to_rgb_array()));
}

/// Add to Swatches from the picker keeps typed CMYK inks as a CMYK swatch.
#[test]
fn add_picked_keeps_the_typed_model() {
    let mut library = library();
    let color = Srgba::rgb(0.5, 0.5, 0.5);
    let cmyk = PickedColor { color, model: Some(ModelValues::new("cmyk", &[10.0, 20.0, 30.0, 40.0])) };
    let at = library.add_picked(&cmyk).expect("room");
    let entry = library.get(at).expect("added");
    assert_eq!(entry.color, SwatchColor::Cmyk([0.1, 0.2, 0.3, 0.4]));
    let plain = library.add_picked(&PickedColor { color, model: None }).expect("room");
    assert_eq!(library.get(plain).map(|e| &e.color), Some(&SwatchColor::Rgb([0.5, 0.5, 0.5])));
}

/// The export entries are disabled with no swatches.
#[test]
fn export_is_disabled_when_empty() {
    let ctx = egui::Context::default();
    let enabled = |library: &SwatchLibrary| {
        let entries = SwatchesPanel::panel_menu(&ctx, "swatches", library);
        let state = |label: &str| entries.iter().filter_map(|e| e.item()).find(|i| i.label() == label).map(|i| i.is_enabled());
        (state("Export Swatches…"), state("Export Swatches for Exchange…"), state("Import Swatches…"))
    };
    let empty = SwatchLibrary { groups: vec![SwatchGroup { name: "Empty".into(), entries: Vec::new() }] };
    assert_eq!(enabled(&empty), (Some(false), Some(false), Some(true)));
    assert_eq!(enabled(&library()), (Some(true), Some(true), Some(true)));
}

/// The menu's New Swatch goes to the selected swatch's group, like the footer's, and only for
/// the panel whose selection it is.
#[test]
fn menu_commands_use_the_panels_selection() {
    let mut harness = harness(library());
    let pink = center(cell(&harness, "Pink"));
    click(&mut harness, pink, Modifiers::NONE);
    let ctx = harness.ctx.clone();
    let fg = harness.state().foreground;
    let new_swatch = "swatches.newSwatch".to_owned();
    let response = SwatchesPanel::run_command(&ctx, "swatches", &mut harness.state_mut().library, &new_swatch, fg);
    assert!(response.changed);
    assert_eq!(harness.state().library.groups[1].entries.len(), 2);
    let other = SwatchesPanel::run_command(&ctx, "another panel", &mut harness.state_mut().library, &new_swatch, fg);
    assert!(other.changed);
    assert_eq!(harness.state().library.groups[1].entries.len(), 2, "another panel has no selection");
    assert_eq!(harness.state().library.groups[0].entries.len(), 3, "so the swatch goes to the first group");
}

/// Import and Replace pick a swatch file through the host's file service, and the panel reads it
/// in the pass the answer arrives.
#[test]
fn import_and_replace_read_the_picked_file() {
    let dir = std::env::temp_dir().join(format!("craft-swatches-import-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("brand.gpl");
    std::fs::write(&path, "GIMP Palette\nName: Brand\n#\n  0 128 255 Sky\n").unwrap();
    for (command, replace) in [("swatches.import", false), ("swatches.replace", true)] {
        let files = Scripted::new(Picked::Paths(vec![path.clone()]));
        let mut harness = harness_on(library(), Some(files.clone()));
        let ctx = harness.ctx.clone();
        let fg = harness.state().foreground;
        let response = SwatchesPanel::run_command(&ctx, "swatches", &mut harness.state_mut().library, &command.to_owned(), fg);
        assert!(!response.changed, "the library changes in the pass the answer arrives");
        harness.run_steps(2);
        let asked = files.asked.lock().unwrap().clone();
        assert_eq!(asked.len(), 1);
        assert_eq!(asked[0].purpose, "swatches");
        assert_eq!(asked[0].kind, craft_host::PickKind::Open);
        let names: Vec<&str> = harness.state().library.groups.iter().flat_map(|g| g.entries.iter().map(|e| e.name.as_str())).collect();
        assert!(names.contains(&"Sky"), "{names:?}");
        assert_eq!(names.contains(&"Red"), !replace, "{names:?}");
        assert_eq!(harness.state().reported.len(), 1, "reported once");
        assert!(harness.state().reported[0].imported.is_some());
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// A file that is not a swatch file is a status line, not a change; a cancelled picker is nothing.
#[test]
fn a_bad_file_is_a_status_line_and_a_cancel_is_nothing() {
    let dir = std::env::temp_dir().join(format!("craft-swatches-bad-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("junk.bin");
    std::fs::write(&path, "nothing").unwrap();
    let mut harness = harness_on(library(), Some(Scripted::new(Picked::Paths(vec![path]))));
    let ctx = harness.ctx.clone();
    let fg = harness.state().foreground;
    let _ = SwatchesPanel::run_command(&ctx, "swatches", &mut harness.state_mut().library, &"swatches.import".to_owned(), fg);
    harness.run_steps(2);
    let _ = std::fs::remove_dir_all(&dir);
    let status = harness.state().reported[0].status.clone();
    assert_eq!(status, Some(StatusLine { text: "Swatches could not be imported: not a swatch file".into(), tone: Tone::Bad }));
    assert_eq!(harness.state().library, library());

    let mut harness = harness_on(library(), Some(Scripted::new(Picked::Cancelled)));
    let ctx = harness.ctx.clone();
    let _ = SwatchesPanel::run_command(&ctx, "swatches", &mut harness.state_mut().library, &"swatches.import".to_owned(), fg);
    harness.run_steps(2);
    assert!(harness.state().reported.is_empty());
}

/// Export asks for a path to write, and the panel writes the file there.
#[test]
fn export_writes_the_picked_path() {
    let path = std::path::PathBuf::from("/swatches/brand.ase");
    let files = Scripted::new(Picked::Paths(vec![path.clone()]));
    let mut harness = harness_on(library(), Some(files.clone()));
    let ctx = harness.ctx.clone();
    let fg = harness.state().foreground;
    let _ = SwatchesPanel::run_command(&ctx, "swatches", &mut harness.state_mut().library, &"swatches.exportExchange".to_owned(), fg);
    harness.run_steps(2);
    let asked = files.asked.lock().unwrap().clone();
    assert_eq!((asked[0].kind, asked[0].suggested_extension.as_deref()), (craft_host::PickKind::Save, Some("ase")));
    let written = files.written.lock().unwrap().clone();
    assert_eq!(written.len(), 1);
    assert_eq!(written[0].0, path);
    assert_eq!(crate::palette::SwatchFormat::sniff(&written[0].1, "x.ase"), Some(SwatchFormat::Ase));
    assert!(harness.state().reported[0].exported.is_some());
}

/// A swatch's accessible name carries its value; the plain name stays the label.
#[test]
fn a_swatch_is_named_with_its_value() {
    let harness = harness(library());
    assert_eq!(cell(&harness, "Red").accesskit_node().label().as_deref(), Some("Red, #ff0000"));
}

/// Notes are worded in the panel, with plural forms for counts, and make one status line.
#[test]
fn notes_are_worded_and_make_a_status_line() {
    let ctx = egui::Context::default();
    let report = SwatchReport { notes: vec![SwatchNote::BookColors(1), SwatchNote::ConvertedToRgb(3), SwatchNote::GroupsFlattened] };
    assert_eq!(
        SwatchesPanel::note_lines(&ctx, &report, true),
        ["1 color-book color was left out.", "3 colors were converted to RGB.", "The format has no groups; the swatches were written as one list."]
    );
    assert_eq!(
        SwatchesPanel::note_lines(&ctx, &SwatchReport { notes: vec![SwatchNote::BookColors(2)] }, false),
        ["2 color-book colors were read as black."]
    );
    assert_eq!(SwatchesPanel::status_line(&ctx, &SwatchReport::default(), false), None);
    let line = SwatchesPanel::status_line(&ctx, &report, true).expect("notes to say");
    assert_eq!(line.tone, Tone::Warn);
    assert!(line.text.starts_with("Swatches exported with changes: 1 color-book color was left out. 3 colors"), "{}", line.text);
}

/// Dragging and the Move to command keep names unique and the library whole.
#[test]
fn import_and_moves_keep_names_unique() {
    let mut library = library();
    library.import(vec![SwatchGroup { name: "Basics".into(), entries: vec![rgb("Red", [0.5, 0.0, 0.0])] }], false);
    assert_eq!((library.groups[2].name.as_str(), library.groups[2].entries[0].name.as_str()), ("Basics 1", "Red 2"));

    let mv = |items: Vec<Key>, target: Key, position| Move { items, target, position, copy: false };
    let first = Key::Swatch("Red".into());
    assert!(library.apply_move(&mv(vec![first.clone()], Key::Swatch("Pink".into()), DropPosition::After)));
    assert_eq!(library.groups[1].entries.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), ["Pink", "Red"]);
    assert!(library.apply_move(&mv(vec![first], Key::Group("Basics".into()), DropPosition::Into)));
    assert_eq!(library.groups[0].entries[0].name, "Red");
    assert!(library.apply_move(&mv(vec![Key::Group("Pastels".into())], Key::Group("Basics".into()), DropPosition::Before)));
    assert_eq!(library.groups[0].name, "Pastels");
    assert_eq!(library.total(), 4);
}

/// A swatch added without a group goes to the first group, not the last; a full library or group
/// list refuses the next one.
#[test]
fn add_defaults_to_the_first_group_and_the_library_has_caps() {
    let mut library = library();
    let at = library.add(SwatchColor::Rgb([0.0; 3]), None, None).expect("room");
    assert_eq!(at.group, 0);
    assert_eq!(library.get(at).map(|e| e.name.as_str()), Some("Color Swatch 2"));
    library.groups[1].entries.resize(super::MAX_SWATCHES, SwatchEntry::new("x", SwatchColor::Rgb([0.0; 3])));
    assert!(library.add(SwatchColor::Rgb([0.0; 3]), None, None).is_none(), "full");
    library.groups.resize(super::MAX_GROUPS, SwatchGroup { name: "g".into(), entries: Vec::new() });
    assert!(library.new_group("More").is_none());
}
