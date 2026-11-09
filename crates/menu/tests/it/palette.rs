//! The menu commands as help-search entries.

use craft_menu::{Item, ItemRole, LayoutOptions, Menu, MenuBar, MenuRole, Node, Target};
use craft_ui::menu::HelpEntry;
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

#[trial]
fn paths_follow_nested_submenus_and_skip_separators_and_submenu_rows() {
    let all = entries(Target::Linux);
    assert_eq!(path(find(&all, "image.curves")), ["Image", "Adjustments", "Levels"]);
    assert_eq!(path(find(&all, "file.export.png")), ["File", "Export"]);
    assert!(all.iter().all(|entry| !["Export", "Adjustments", "Levels"].contains(&entry.label().as_str())));
    assert_eq!(all.iter().filter(|entry| entry.label().as_str() == "Save").count(), 1);
}
