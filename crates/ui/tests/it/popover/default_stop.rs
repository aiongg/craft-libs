//! A popover opens on the field that declares `default_stop`, and typed text reaches that field's draft.

use craft_ui::contract::{Bind, Props as _, Show as _, ShowContainer as _};
use craft_ui::popover::Popover;
use craft_ui::runtime::focus::Initial;
use craft_ui::testing::{Fixture, trial};
use craft_ui::text::TextField;
use egui::Id;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct Rig {
    fixture: Fixture,
    second: Rc<Cell<Id>>,
    drafts: Rc<RefCell<[String; 2]>>,
}

fn rig() -> Rig {
    let (second, drafts) = (Rc::new(Cell::new(Id::NULL)), Rc::new(RefCell::new([String::new(), String::new()])));
    let (shown_second, shown_drafts) = (second.clone(), drafts.clone());
    let mut fixture = Fixture::new(move |ui| {
        let mut open = true;
        Popover::new("Reply", Bind::bound(&mut open)).at_point(egui::pos2(40.0, 40.0)).initial_focus(Initial::Default).show(ui, |ui, _| {
            let mut drafts = shown_drafts.borrow_mut();
            let [first, second] = &mut *drafts;
            TextField::new("Subject", Bind::bound(first)).show(ui);
            shown_second.set(TextField::new("Reply", Bind::bound(second)).default_stop().show(ui).response.id);
        });
    });
    fixture.settle();
    Rig { fixture, second, drafts }
}

/// `Initial::Default` focuses the field that declared `default_stop`, not the first field.
#[trial]
fn a_popover_opens_on_the_field_that_declares_the_default_stop() {
    let rig = rig();
    assert_eq!(rig.fixture.focused_stop().map(|(id, _)| id), Some(rig.second.get()));
}

