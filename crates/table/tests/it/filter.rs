//! `FilterButton`: Alt+Down on the focused button opens the filter menu.

use std::cell::RefCell;
use std::rc::Rc;

use craft_table::ui::{FilterButton, FilterButtonEvent};
use craft_ui::contract::Show as _;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};

#[trial]
fn alt_down_on_the_filter_button_opens_it() {
    let opened: Rc<RefCell<Vec<FilterButtonEvent>>> = Rc::default();
    let shown = opened.clone();
    let mut fixture = Fixture::new(move |ui| {
        let out = FilterButton::new("Region").show(ui);
        shown.borrow_mut().extend(out.event);
    });
    fixture.settle();
    let button = fixture.by_label("Region (Showing All)").expect("the button is named by its column");
    fixture.click(&button);
    assert_eq!(*opened.borrow(), [FilterButtonEvent::Open], "a click opens it");
    opened.borrow_mut().clear();
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.key(Key::ArrowDown, Modifiers::ALT);
    assert_eq!(*opened.borrow(), [FilterButtonEvent::Open], "Alt+Down on the focused button opens it");
}
