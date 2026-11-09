//! What a menu item's AccessKit node says about its mark.

use craft_behavior::Mixed;
use craft_ui::contract::{Role, Show as _};
use craft_ui::menu::MenuItem;
use craft_ui::testing::{Fixture, trial};

fn toggled(fixture: &Fixture, label: &str) -> Option<Mixed<bool>> {
    let target = fixture.by_label(label).unwrap_or_else(|| panic!("{label} has a node"));
    fixture.tree().toggled_of(target.node)
}

