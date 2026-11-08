//! Window menu document rows.

use craft_keymap::CommandRef;

use craft_menu::{Document, Item, ItemKind, LayoutOptions, Menu, MenuBar, MenuRole, Node, Target, layout};
use craft_ui::testing::trial;

fn window_bar() -> MenuBar {
    MenuBar::new(vec![Menu::new("Window", vec![Item::new("panel.layers", "Layers").into()]).role(MenuRole::Window)]).documents(vec![
        Document::numbered(0, "One").active(true),
        Document::numbered(1, "Two"),
        Document::numbered(2, "Hidden").hidden(true),
    ])
}

// Every layout lists the documents last in Window as check items with the active one checked (Photoshop's check mark), not radio items.
#[trial]
fn window_documents_are_check_items() {
    for target in [Target::Mac, Target::Windows, Target::Linux, Target::Web] {
        let bar = layout(&window_bar(), target, &LayoutOptions::new("Sample")).bar;
        let state = |id: &str| bar.find(&CommandRef::new(id)).map(|i| i.kind);
        assert_eq!(state("window.document.0"), Some(ItemKind::Check(true)), "{target:?}");
        assert_eq!(state("window.document.1"), Some(ItemKind::Check(false)), "{target:?}");
        assert_eq!(state("window.document.2"), None, "{target:?}: hidden documents aren't listed");
        let window = bar.menu(MenuRole::Window).unwrap();
        assert!(matches!(window.children.last(), Some(Node::Item(i)) if i.command.id == "window.document.1"), "{target:?}: documents are last");
    }
}

// listed_items ends with window.document.0 and window.document.1 for two documents; hidden documents are left out; index_from_command reads the index back.
#[trial]
fn listed_items_include_numbered_document_rows() {
    let listed = window_bar().listed_items();
    let ids: Vec<&str> = listed.iter().map(|i| i.command.id.as_str()).collect();
    assert_eq!(ids, ["panel.layers", "window.document.0", "window.document.1"]);
    assert_eq!(listed[1].kind, ItemKind::Check(true));
    assert_eq!(Document::index_from_command(&listed[2].command), Some(1));
    assert_eq!(Document::index_from_command(&CommandRef::new("window.document.x")), None);
    assert_eq!(Document::index_from_command(&CommandRef::new("panel.layers")), None);
}
