//! The menus as command-palette entries.

use craft_keymap::{Bindable, Binding, Keymap, Platform, Scope, Scopes};
use craft_ui::behavior::CommandSearch;

use super::*;

fn keys() -> Keymap {
    let catalog = vec![Bindable::new("file.save", "Save", Scope::app()).with_default(Binding::new(chord("Ctrl+S")))];
    Keymap::new(catalog, Scopes::new(), Platform::Linux)
}

fn bar() -> MenuBar {
    let item = |id: &str, label: &str| Item::new(id, label);
    MenuBar {
        menus: vec![
            Menu::new(
                "File",
                vec![
                    item("file.save", "Save").into(),
                    Node::Separator,
                    Node::submenu("Export", vec![item("file.export.png", "PNG…").into(), item("file.export.pdf", "PDF…").enabled(false).into()]),
                    item("file.quit", "Quit").role(ItemRole::Quit).into(),
                ],
            )
            .role(MenuRole::File),
            Menu::new("Image", vec![Node::submenu("Adjustments", vec![Node::submenu("Levels", vec![item("image.curves", "Curves…").into()])])]),
            Menu::new("View", vec![item("view.grid", "Grid").check(true).into()]).role(MenuRole::View),
        ],
        documents: Vec::new(),
    }
}

fn entries(target: Target) -> Vec<craft_ui::behavior::PaletteEntry<CommandRef>> {
    bar().palette_entries(&keys(), target, &LayoutOptions::new("Sample"), None, egui::os::OperatingSystem::Nix)
}

fn find<'a>(all: &'a [craft_ui::behavior::PaletteEntry<CommandRef>], id: &str) -> &'a craft_ui::behavior::PaletteEntry<CommandRef> {
    all.iter().find(|e| e.command().id == id).unwrap()
}

#[test]
fn paths_follow_nested_submenus_and_skip_separators_and_submenu_rows() {
    let all = entries(Target::Linux);
    assert_eq!(find(&all, "image.curves").path_of(), ["Image", "Adjustments", "Levels"]);
    assert_eq!(find(&all, "file.export.png").path_of(), ["File", "Export"]);
    assert!(all.iter().all(|e| !["Export", "Adjustments", "Levels"].contains(&e.label())));
    assert_eq!(all.iter().filter(|e| e.label() == "Save").count(), 1);
}

#[test]
fn shortcut_and_enabled_state_are_carried() {
    let all = entries(Target::Linux);
    let expect = craft_keymap::ui::kbd(&chord("Ctrl+S"), Platform::Linux);
    assert_eq!(find(&all, "file.save").shortcut_of(), Some(&expect));
    assert!(find(&all, "file.save").is_enabled());
    assert!(!find(&all, "file.export.pdf").is_enabled(), "a disabled command is listed, not runnable");
    assert!(find(&all, "view.grid").shortcut_of().is_none());
}

#[test]
fn the_paths_are_the_ones_of_the_laid_out_bar() {
    let linux = entries(Target::Linux);
    assert_eq!(find(&linux, "file.quit").path_of(), ["File"]);
    let mac = entries(Target::Mac);
    assert_eq!(find(&mac, "file.quit").path_of(), ["Sample"], "Quit is in the app menu on a Mac");
    assert!(mac.iter().all(|e| Builtin::of(e.command()).is_none()));
}

#[test]
fn a_search_finds_a_nested_command_by_a_word_of_its_path() {
    let all = entries(Target::Linux);
    let found = CommandSearch::new(&all).query("adjustments").results(5);
    assert_eq!(found.first().map(|m| m.entry.command().id.as_str()), Some("image.curves"));
}

#[test]
fn a_translated_command_is_found_by_its_english_name() {
    let bar = MenuBar { menus: vec![Menu::new("Datei", vec![Item::new("file.save", "Speichern").into()])], documents: Vec::new() };
    let english = |l: Labeled<'_>| match l {
        Labeled::Command(c) if c.id == "file.save" => Some("Save".to_owned()),
        Labeled::Title("Datei") => Some("File".to_owned()),
        _ => None,
    };
    let all = bar.palette_entries(&keys(), Target::Linux, &LayoutOptions::new("Sample"), Some(&english), egui::os::OperatingSystem::Nix);
    assert_eq!(CommandSearch::new(&all).query("save").results(5).len(), 1);
    assert_eq!(CommandSearch::new(&all).query("speich").results(5).len(), 1);
}
