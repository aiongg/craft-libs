//! The Swatches panel on a kittest harness: clicks, New Swatch, the panel menu, and the library.

use craft_cms::convert::CmykConverter;
use craft_theme::{ModelValues, Srgba};
use craft_ui::behavior::{DropPosition, Move};
use craft_ui::composites::NoticeCenter;
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Event, Modifiers, PointerButton, Pos2, Ui};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, Node};

use super::swatches::Key;
use super::{PickedColor, SwatchAt, SwatchLibrary, SwatchesAction, SwatchesPanel};
use crate::palette::{SwatchColor, SwatchEntry, SwatchFormat, SwatchGroup, SwatchNote, SwatchReport};

struct Host {
    library: SwatchLibrary,
    foreground: Srgba,
    used: Vec<(SwatchAt, Srgba, bool)>,
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
    let response = SwatchesPanel::new("swatches", &mut host.library, CmykConverter::coated(), host.foreground).show(ui);
    host.used.extend(response.used);
}

fn harness(library: SwatchLibrary) -> Harness<'static, Host> {
    let host = Host { library, foreground: Srgba::rgb(0.2, 0.4, 0.6), used: Vec::new(), time: 0.0 };
    let mut harness = Harness::<Host>::builder()
        .with_size([320.0, 320.0])
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
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
        let entries = SwatchesPanel::panel_menu(&ctx, library);
        let state = |label: &str| entries.iter().filter_map(|e| e.item()).find(|i| i.label() == label).map(|i| i.is_enabled());
        (state("Export Swatches…"), state("Export Swatches for Exchange…"), state("Import Swatches…"))
    };
    let empty = SwatchLibrary { groups: vec![SwatchGroup { name: "Empty".into(), entries: Vec::new() }] };
    assert_eq!(enabled(&empty), (Some(false), Some(false), Some(true)));
    assert_eq!(enabled(&library()), (Some(true), Some(true), Some(true)));
}

/// The menu's New Swatch goes to the selected swatch's group, like the footer's, and the file
/// entries come back as actions.
#[test]
fn menu_commands_use_the_panels_selection() {
    let mut harness = harness(library());
    let pink = center(cell(&harness, "Pink"));
    click(&mut harness, pink, Modifiers::NONE);
    let ctx = harness.ctx.clone();
    let fg = harness.state().foreground;
    let response = SwatchesPanel::run_command(&ctx, "swatches.newSwatch", &mut harness.state_mut().library, fg);
    assert!(response.changed);
    assert_eq!(harness.state().library.groups[1].entries.len(), 2);
    let export = SwatchesPanel::run_command(&ctx, "swatches.exportExchange", &mut harness.state_mut().library, fg);
    assert_eq!((export.changed, export.action), (false, Some(SwatchesAction::Export(SwatchFormat::Ase))));
}

/// A swatch's accessible name carries its value; the plain name stays the label.
#[test]
fn a_swatch_is_named_with_its_value() {
    let harness = harness(library());
    assert_eq!(cell(&harness, "Red").accesskit_node().label().as_deref(), Some("Red, #ff0000"));
}

/// Notes are worded in the panel, with plural forms for counts, and post as one notice.
#[test]
fn notes_are_worded_and_posted() {
    let ctx = egui::Context::default();
    let report = SwatchReport { notes: vec![SwatchNote::BookColors(1), SwatchNote::ConvertedToRgb(3), SwatchNote::GroupsFlattened] };
    assert_eq!(
        SwatchesPanel::note_lines(&ctx, &report, true),
        [
            "1 colour-book colour was left out.",
            "3 colours were converted to RGB.",
            "The format has no groups; the swatches were written as one list."
        ]
    );
    assert_eq!(
        SwatchesPanel::note_lines(&ctx, &SwatchReport { notes: vec![SwatchNote::BookColors(2)] }, false),
        ["2 colour-book colours were read as black."]
    );
    let mut center = NoticeCenter::new();
    assert_eq!(SwatchesPanel::post_notes(&ctx, &mut center, &SwatchReport::default(), false), None);
    assert!(SwatchesPanel::post_notes(&ctx, &mut center, &report, true).is_some());
    assert_eq!(center.notices()[0].title, "Swatches exported with changes");
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
