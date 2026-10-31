//! The Swatches panel on a kittest harness: clicks, New Swatch, the panel menu, and the library.

use craft_cms::convert::CmykConverter;
use craft_theme::{ModelValues, Srgba};
use craft_ui::behavior::{DropPosition, Move};
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Event, Modifiers, PointerButton, Pos2, Ui};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, Node};

use super::swatches::Key;
use super::{PickedColor, SwatchAt, SwatchLibrary, SwatchesAction, SwatchesPanel};
use crate::palette::{SwatchColor, SwatchEntry, SwatchFormat, SwatchGroup};

struct Host {
    library: SwatchLibrary,
    foreground: Srgba,
    used: Vec<(SwatchAt, Srgba, bool)>,
    time: f64,
}

fn rgb(name: &str, c: [f32; 3]) -> SwatchEntry {
    SwatchEntry::new(name, SwatchColor::Rgb(c))
}

/// "Basics" with Red and Swatch 1, "Pastels" with Pink.
fn library() -> SwatchLibrary {
    SwatchLibrary {
        groups: vec![
            SwatchGroup { name: "Basics".into(), entries: vec![rgb("Red", [1.0, 0.0, 0.0]), rgb("Swatch 1", [0.0, 0.0, 1.0])] },
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
    let label = label.to_owned();
    harness.get_by(move |n| matches!(n.role(), Role::TreeItem | Role::ListBoxOption) && n.label().as_deref() == Some(label.as_str()))
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
    assert_eq!(added.name, "Swatch 2", "Swatch 1 is taken");
    assert_eq!(added.color, SwatchColor::Rgb(host.foreground.to_rgb_array()));
}

/// Add to Swatches from the picker keeps typed CMYK inks as a CMYK swatch.
#[test]
fn add_picked_keeps_the_typed_model() {
    let mut library = library();
    let color = Srgba::rgb(0.5, 0.5, 0.5);
    let cmyk = PickedColor { color, model: Some(ModelValues::new("cmyk", &[10.0, 20.0, 30.0, 40.0])) };
    let at = library.add_picked(&cmyk);
    let entry = library.get(at).expect("added");
    assert_eq!(entry.color, SwatchColor::Cmyk([0.1, 0.2, 0.3, 0.4]));
    let plain = library.add_picked(&PickedColor { color, model: None });
    assert_eq!(library.get(plain).map(|e| &e.color), Some(&SwatchColor::Rgb([0.5, 0.5, 0.5])));
}

struct MenuHost {
    library: SwatchLibrary,
    action: Option<SwatchesAction>,
}

fn menu_harness(library: SwatchLibrary) -> Harness<'static, MenuHost> {
    let mut harness =
        Harness::<MenuHost>::builder().with_size([320.0, 320.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, host: &mut MenuHost| {
                host.action = SwatchesPanel::panel_menu(ui, &mut host.library, Srgba::rgb(0.0, 0.0, 0.0)).or(host.action);
            },
            MenuHost { library, action: None },
        );
    harness.run();
    harness
}

/// The export entries are disabled with no swatches.
#[test]
fn export_is_disabled_when_empty() {
    let disabled = |library| {
        let harness = menu_harness(library);
        let state = |label: &str| harness.get_by_label(label).accesskit_node().is_disabled();
        (state("Export Swatches…"), state("Export Swatches for Exchange…"), state("Import Swatches…"))
    };
    assert_eq!(disabled(SwatchLibrary { groups: vec![SwatchGroup { name: "Empty".into(), entries: Vec::new() }] }), (true, true, false));
    assert_eq!(disabled(library()), (false, false, false));

    let mut harness = menu_harness(library());
    harness.get_by_label("Export Swatches…").click();
    harness.run();
    assert_eq!(harness.state().action, Some(SwatchesAction::Export(SwatchFormat::Aco)));
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
