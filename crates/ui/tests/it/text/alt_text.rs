//! `AltTextField`: the decorative flag rule, the check beside a mixed or decorative description, and a click on a
//! mixed check.

use craft_behavior::{Mixed, PartKey};
use craft_ui::contract::{Bind, Show as _};
use craft_ui::testing::{Fixture, trial};
use craft_ui::text::{AltText, AltTextField};
use egui::{Key, Modifiers};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const DECORATIVE: PartKey = PartKey::new("decorative");

/// A field over `value` that keeps what the app writes, and the id it showed under.
fn rig(value: AltText) -> (Fixture, Rc<RefCell<AltText>>, Rc<Cell<egui::Id>>) {
    let value = Rc::new(RefCell::new(value));
    let id = Rc::new(Cell::new(egui::Id::NULL));
    let (shown, field) = (value.clone(), id.clone());
    let mut fixture = Fixture::new(move |ui| {
        field.set(AltTextField::new("Describe", Bind::bound(&mut *shown.borrow_mut())).show(ui).response.id);
    });
    fixture.settle();
    (fixture, value, id)
}

/// With the descriptions mixed and the flag mixed, the check's node reports a mixed state; checking it sets the flag
/// on every target, keeps the text mixed, and the node then reports checked.
#[trial]
fn checking_decorative_on_a_mixed_selection_marks_every_target_and_keeps_the_text_mixed() {
    let (mut fixture, value, id) = rig(AltText::parts(Mixed::Mixed, Mixed::Mixed));
    let check = fixture.part(id.get(), DECORATIVE).expect("the check has its own node");
    assert_eq!(fixture.tree().toggled_of(check.node), Some(Mixed::Mixed), "the check reports a mixed state");
    fixture.click(&check);
    fixture.idle(3);
    assert_eq!(*value.borrow(), AltText::parts(Mixed::Mixed, Mixed::Same(true)), "every target is decorative; the text stays per target");
    let check = fixture.part(id.get(), DECORATIVE).expect("the check keeps its node");
    assert_eq!(fixture.tree().toggled_of(check.node), Some(Mixed::Same(true)), "the check reports checked");
}
