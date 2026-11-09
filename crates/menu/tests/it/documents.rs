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

// listed_items ends with window.document.0 and window.document.1 for two documents; hidden documents are left out; index_from_command reads the index back.
