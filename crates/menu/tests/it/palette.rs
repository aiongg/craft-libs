//! The menu commands as help-search entries.

use craft_i18n::Folder;
use craft_menu::{Builtin, Item, ItemRole, Labeled, LayoutOptions, Menu, MenuBar, MenuRole, Node, Target};
use craft_ui::menu::HelpEntry;
use craft_ui::menu::search::search_order;
use craft_ui::runtime::env::Label;
use craft_ui::testing::trial;

fn bar() -> MenuBar {
    let item = |id: &str, label: &str| Item::new(id, label);
    MenuBar::new(vec![
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
    ])
}

fn entries(target: Target) -> Vec<HelpEntry> {
    bar().help_entries(target, &LayoutOptions::new("Sample"), None)
}

fn find<'a>(all: &'a [HelpEntry], id: &str) -> &'a HelpEntry {
    all.iter().find(|entry| entry.command().id == id).expect("the entry")
}

fn path(entry: &HelpEntry) -> Vec<&str> {
    entry.path().iter().map(Label::as_str).collect()
}

/// The search texts of `entries`, as the bar builds them.
fn texts(entries: &[HelpEntry]) -> Vec<[String; 3]> {
    entries
        .iter()
        .map(|entry| {
            [entry.label().as_str().to_owned(), entry.english().map(|label| label.as_str().to_owned()).unwrap_or_default(), path(entry).join(" › ")]
        })
        .collect()
}

fn found(entries: &[HelpEntry], query: &str) -> Vec<String> {
    let texts = texts(entries);
    let refs: Vec<[&str; 3]> = texts.iter().map(|[a, b, c]| [a.as_str(), b.as_str(), c.as_str()]).collect();
    search_order(&Folder::new("en"), query, &refs).into_iter().map(|at| entries[at].command().id.clone()).collect()
}

#[trial]
fn paths_follow_nested_submenus_and_skip_separators_and_submenu_rows() {
    let all = entries(Target::Linux);
    assert_eq!(path(find(&all, "image.curves")), ["Image", "Adjustments", "Levels"]);
    assert_eq!(path(find(&all, "file.export.png")), ["File", "Export"]);
    assert!(all.iter().all(|entry| !["Export", "Adjustments", "Levels"].contains(&entry.label().as_str())));
    assert_eq!(all.iter().filter(|entry| entry.label().as_str() == "Save").count(), 1);
}

#[trial]
fn the_paths_are_the_ones_of_the_laid_out_bar() {
    let linux = entries(Target::Linux);
    assert_eq!(path(find(&linux, "file.quit")), ["File"]);
    let mac = entries(Target::Mac);
    assert_eq!(path(find(&mac, "file.quit")), ["Sample"], "Quit is in the app menu on a Mac");
    assert!(mac.iter().all(|entry| Builtin::of(entry.command()).is_none()));
}

#[trial]
fn a_search_finds_a_nested_command_by_a_word_of_its_path() {
    let all = entries(Target::Linux);
    assert_eq!(found(&all, "adjustments").first().map(String::as_str), Some("image.curves"));
}

#[trial]
fn a_translated_command_is_found_by_its_english_name() {
    let bar = MenuBar::new(vec![Menu::new("Datei", vec![Item::new("file.save", "Speichern").into()])]);
    let english = |labeled: Labeled<'_>| match labeled {
        Labeled::Command(c) if c.id == "file.save" => Some("Save".to_owned()),
        Labeled::Title("Datei") => Some("File".to_owned()),
        _ => None,
    };
    let all = bar.help_entries(Target::Linux, &LayoutOptions::new("Sample"), Some(&english));
    assert_eq!(found(&all, "save").len(), 1);
    assert_eq!(found(&all, "speich").len(), 1);
}
