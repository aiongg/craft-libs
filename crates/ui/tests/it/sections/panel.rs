//! `Panel`: the header's menu opens from the menu key as well as from a click on the menu button.

use craft_ui::contract::{Role, Show as _, ShowContainer as _};
use craft_ui::menu::MenuItem;
use craft_ui::sections::{HeaderAction, Panel};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};

#[trial]
fn the_menu_key_on_a_panel_header_opens_the_panel_menu() {
    let mut fixture = Fixture::new(|ui| {
        Panel::new("Layers")
            .actions([HeaderAction::Menu])
            .menu(|ui, _| {
                MenuItem::new("Rename").show(ui);
            })
            .show(ui, |_, _| {});
    });
    fixture.settle();
    assert!(fixture.by_role(Role::MENU).is_empty(), "the menu starts closed");
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.key(Key::F10, Modifiers::SHIFT);
    fixture.settle();
    assert!(!fixture.by_role(Role::MENU).is_empty(), "the menu key on the focused header opens the panel menu");
}

#[trial]
fn a_click_on_a_panels_menu_button_opens_the_panel_menu() {
    let mut fixture = Fixture::new(|ui| {
        Panel::new("Layers")
            .actions([HeaderAction::Menu])
            .menu(|ui, _| {
                MenuItem::new("Rename").show(ui);
            })
            .show(ui, |_, _| {});
    });
    fixture.settle();
    let button = fixture.by_label("More actions").expect("the header shows its menu button");
    fixture.click(&button);
    assert!(fixture.by_label("Rename").is_some(), "a click on the menu button opens the panel menu");
}
