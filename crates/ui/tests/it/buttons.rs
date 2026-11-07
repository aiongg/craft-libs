//! `SplitButton`: the main part runs the action and never opens the menu, the arrow and Alt+Down open it, a pick
//! reports the entry as one choice and closes the menu.

use craft_ui::buttons::SplitButton;
use craft_ui::contract::{Entry, Role, Show as _};
use craft_ui::testing::focus::AccessActionKind;
use craft_ui::testing::{Fixture, trial};
use egui::{Id, Key, Modifiers};
use std::cell::Cell;
use std::rc::Rc;

struct Rig {
    fixture: Fixture,
    clicks: Rc<Cell<u32>>,
    chosen: Rc<Cell<Option<u8>>>,
    id: Rc<Cell<Option<Id>>>,
}

fn rig() -> Rig {
    let (clicks, chosen, id) = (Rc::new(Cell::new(0)), Rc::new(Cell::new(None)), Rc::new(Cell::new(None)));
    let (shown_clicks, shown_chosen, shown_id) = (clicks.clone(), chosen.clone(), id.clone());
    let mut fixture = Fixture::new(move |ui| {
        let entries =
            [Entry::new(7u8, "Colour").checked(true), Entry::separator(), Entry::new(8, "Duotone").enabled(false), Entry::new(9, "Grayscale")];
        let out = SplitButton::new("Trace", &entries).show(ui);
        shown_id.set(Some(out.response.id));
        if craft_ui::testing::contract::clicked(&out.response) {
            shown_clicks.set(shown_clicks.get() + 1);
        }
        if out.chosen.is_some() {
            shown_chosen.set(out.chosen);
        }
    });
    fixture.settle();
    Rig { fixture, clicks, chosen, id }
}

impl Rig {
    fn menu_shown(&self) -> bool {
        !self.fixture.by_role(Role::MENU).is_empty()
    }

    fn press(&mut self, key: Key, modifiers: Modifiers) {
        self.fixture.key(key, modifiers);
        self.fixture.idle(2);
    }
}

/// AccessKit Expand on the arrow's sub-node opens the menu and Collapse closes it, and the sub-node reports which.
#[trial]
fn expand_and_collapse_on_the_arrow_open_and_close_the_menu() {
    let mut rig = rig();
    let id = rig.id.get().expect("the button showed");
    let arrow = |rig: &Rig| rig.fixture.part(id, craft_behavior::PartKey::new("arrow")).expect("the arrow has a node");
    assert_eq!(rig.fixture.tree().expanded_of(arrow(&rig).node), Some(false), "the closed menu's arrow is expandable");
    let target = arrow(&rig);
    rig.fixture.access(AccessActionKind::Expand, &target);
    rig.fixture.idle(4);
    assert!(rig.menu_shown(), "Expand opens the menu");
    assert_eq!(rig.fixture.tree().expanded_of(arrow(&rig).node), Some(true), "the arrow reports it open");
    let target = arrow(&rig);
    rig.fixture.access(AccessActionKind::Collapse, &target);
    rig.fixture.idle(4);
    assert!(!rig.menu_shown(), "Collapse closes it");
    assert_eq!(rig.fixture.tree().expanded_of(arrow(&rig).node), Some(false), "the arrow reports it closed");
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
    rig.fixture.idle(4);
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
