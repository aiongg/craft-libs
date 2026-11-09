//! Openers open on the press inside them, as pop-up buttons and dropdowns do: a press held past the click time still
//! opens, and the release that ends a click leaves the popup open.

use craft_ui::buttons::Button;
use craft_ui::contract::{Bind, Entry, Role, Show as _, ShowContainer as _, Trigger};
use craft_ui::menu::{Menu, MenuItem};
use craft_ui::pickers::Select;
use craft_ui::testing::{Fixture, trial};

/// Frames for a layer's open motion to end.
const FADE_FRAMES: usize = 10;
/// Seconds past every click time.
const HELD: f64 = 2.0;

fn select() -> Fixture {
    let mut fixture = Fixture::new(|ui| {
        let entries = [Entry::new(0u8, "Blue"), Entry::new(1, "Red")];
        let mut color = 0u8;
        Select::new("Color", Bind::bound(&mut color), &entries).show(ui);
    });
    fixture.settle();
    fixture
}

fn menu_button() -> Fixture {
    let mut fixture = Fixture::new(|ui| {
        let opener = Button::new("File").show(ui);
        Menu::new("File").opened_by(&opener, Trigger::Click).show(ui, |ui, _| {
            MenuItem::new("New").show(ui);
        });
    });
    fixture.settle();
    fixture
}

/// Presses `label`, holds the press past the click time, releases it, and tells whether a popup of `role` is open.
fn held_open(mut fixture: Fixture, label: &str, role: Role) -> bool {
    let target = fixture.by_label(label).expect("the opener shows");
    fixture.press(&target);
    fixture.advance(HELD);
    fixture.frame(Vec::new());
    fixture.release(&target);
    fixture.idle(FADE_FRAMES);
    !fixture.by_role(role).is_empty()
}

/// Clicks `label` and tells whether a popup of `role` is open once the motion ends.
fn clicked_open(mut fixture: Fixture, label: &str, role: Role) -> bool {
    let target = fixture.by_label(label).expect("the opener shows");
    fixture.click(&target);
    fixture.idle(FADE_FRAMES);
    !fixture.by_role(role).is_empty()
}

#[trial]
fn a_held_press_opens_a_select_and_a_menu_and_a_click_opens_each_once() {
    {
        assert!(held_open(select(), "Color", Role::LIST_BOX), "a press held past the click time opens the list");
        assert!(clicked_open(select(), "Color", Role::LIST_BOX), "a click opens the list, and its release does not close it");
    }
    {
        assert!(held_open(menu_button(), "File", Role::MENU), "a press held past the click time opens the menu");
        assert!(clicked_open(menu_button(), "File", Role::MENU), "a click opens the menu, and its release does not close it");
    }
}
