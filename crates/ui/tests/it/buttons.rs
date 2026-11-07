//! `SplitButton`: the main part runs the action and never opens the menu, the arrow and Alt+Down open it, a pick
//! reports the entry as one choice and closes the menu.

use craft_ui::buttons::SplitButton;
use craft_ui::contract::{Entry, Role, Show as _};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::Cell;
use std::rc::Rc;

struct Rig {
    fixture: Fixture,
    clicks: Rc<Cell<u32>>,
    chosen: Rc<Cell<Option<u8>>>,
}

fn rig() -> Rig {
    let (clicks, chosen) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(None)));
    let (shown_clicks, shown_chosen) = (clicks.clone(), chosen.clone());
    let mut fixture = Fixture::new(move |ui| {
        let entries =
            [Entry::new(7u8, "Colour").checked(true), Entry::separator(), Entry::new(8, "Duotone").enabled(false), Entry::new(9, "Grayscale")];
        let out = SplitButton::new("Trace", &entries).show(ui);
        if craft_ui::testing::contract::clicked(&out.response) {
            shown_clicks.set(shown_clicks.get() + 1);
        }
        if out.chosen.is_some() {
            shown_chosen.set(out.chosen);
        }
    });
    fixture.settle();
    Rig { fixture, clicks, chosen }
}

impl Rig {
    fn menu_shown(&self) -> bool {
        !self.fixture.by_role(Role::LIST_BOX).is_empty()
    }

    fn press(&mut self, key: Key, modifiers: Modifiers) {
        self.fixture.key(key, modifiers);
        self.fixture.idle(2);
    }
}

/// Enter on the focused button runs the action and leaves the menu closed.
#[trial]
fn a_split_button_runs_its_action_without_opening_the_menu() {
    let mut rig = rig();
    rig.press(Key::Tab, Modifiers::NONE);
    rig.press(Key::Enter, Modifiers::NONE);
    assert_eq!(rig.clicks.get(), 1);
    assert!(!rig.menu_shown());
}

/// Alt+Down opens the menu with its first row active; Enter picks it once and closes the menu without running the
/// action.
#[trial]
fn a_split_button_opens_from_the_keyboard_and_reports_the_pick() {
    let mut rig = rig();
    rig.press(Key::Tab, Modifiers::NONE);
    rig.press(Key::ArrowDown, Modifiers::ALT);
    assert!(rig.menu_shown(), "Alt+Down opens the menu");
    rig.press(Key::Enter, Modifiers::NONE);
    assert_eq!(rig.chosen.get(), Some(7), "the first row was active");
    rig.fixture.idle(30);
    assert!(!rig.menu_shown(), "a pick closes the menu");
    assert_eq!(rig.clicks.get(), 0, "a pick is not the action");
}

/// Down from the first row skips the separator and the disabled entry, and Enter reports the value of the entry it
/// lands on.
#[trial]
fn a_split_button_menu_skips_separators_and_disabled_entries() {
    let mut rig = rig();
    rig.press(Key::Tab, Modifiers::NONE);
    rig.press(Key::ArrowDown, Modifiers::ALT);
    rig.press(Key::ArrowDown, Modifiers::NONE);
    rig.press(Key::Enter, Modifiers::NONE);
    assert_eq!(rig.chosen.get(), Some(9));
}

/// A click on a disabled entry picks nothing and leaves the menu open.
#[trial]
fn a_disabled_split_button_entry_cannot_be_picked() {
    let mut rig = rig();
    rig.press(Key::Tab, Modifiers::NONE);
    rig.press(Key::ArrowDown, Modifiers::ALT);
    let entry = rig.fixture.by_label("Duotone").expect("the disabled entry is listed");
    rig.fixture.click(&entry);
    rig.fixture.idle(2);
    assert_eq!(rig.chosen.get(), None);
    assert!(rig.menu_shown(), "the menu stays open");
}
