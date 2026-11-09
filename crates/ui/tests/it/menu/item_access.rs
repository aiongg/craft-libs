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
fn a_menu_item_reads_its_mark_as_checked_or_unchecked_and_has_no_toggled_state_without_one() {
    let mut fixture = Fixture::new(|ui| {
        MenuItem::new("Rulers").check(true).show(ui);
        MenuItem::new("Guides").check(false).show(ui);
        MenuItem::new("Fit").radio_mark(true).show(ui);
        MenuItem::new("Fill").radio_mark(false).show(ui);
        MenuItem::new("Open").show(ui);
    });
    fixture.settle();
    assert_eq!(fixture.by_role(Role::MENU_ITEM_CHECK_BOX).len(), 2);
    assert_eq!(fixture.by_role(Role::MENU_ITEM_RADIO).len(), 2);
    assert_eq!(toggled(&fixture, "Rulers"), Some(Mixed::Same(true)));
    assert_eq!(toggled(&fixture, "Guides"), Some(Mixed::Same(false)));
    assert_eq!(toggled(&fixture, "Fit"), Some(Mixed::Same(true)));
    assert_eq!(toggled(&fixture, "Fill"), Some(Mixed::Same(false)));
    assert_eq!(toggled(&fixture, "Open"), None);
}
