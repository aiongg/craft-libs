//! What a menu item's AccessKit node says about its mark.

use craft_behavior::Mixed;
use craft_ui::contract::{Role, Show as _};
use craft_ui::menu::MenuItem;
use craft_ui::testing::{Fixture, trial};

fn toggled(fixture: &Fixture, label: &str) -> Option<Mixed<bool>> {
    let target = fixture.by_label(label).unwrap_or_else(|| panic!("{label} has a node"));
    fixture.tree().toggled_of(target.node)
}

#[trial]
fn a_check_item_reads_checked_or_unchecked() {
    let mut fixture = Fixture::new(|ui| {
        MenuItem::new("Rulers").check(true).show(ui);
        MenuItem::new("Guides").check(false).show(ui);
    });
    fixture.settle();
    assert_eq!(fixture.by_role(Role::MENU_ITEM_CHECK_BOX).len(), 2);
    assert_eq!(toggled(&fixture, "Rulers"), Some(Mixed::Same(true)));
    assert_eq!(toggled(&fixture, "Guides"), Some(Mixed::Same(false)));
}

#[trial]
fn a_radio_item_reads_checked_for_the_current_choice() {
    let mut fixture = Fixture::new(|ui| {
        MenuItem::new("Fit").radio_mark(true).show(ui);
        MenuItem::new("Fill").radio_mark(false).show(ui);
    });
    fixture.settle();
    assert_eq!(fixture.by_role(Role::MENU_ITEM_RADIO).len(), 2);
    assert_eq!(toggled(&fixture, "Fit"), Some(Mixed::Same(true)));
    assert_eq!(toggled(&fixture, "Fill"), Some(Mixed::Same(false)));
}

#[trial]
fn an_item_without_a_mark_has_no_toggled_state() {
    let mut fixture = Fixture::new(|ui| {
        MenuItem::new("Open").show(ui);
    });
    fixture.settle();
    assert_eq!(toggled(&fixture, "Open"), None);
}
