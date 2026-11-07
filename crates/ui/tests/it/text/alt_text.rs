//! `AltTextField`: the decorative flag rule, the check beside a mixed or decorative description, and a click on a
//! mixed check.

use accesskit::Toggled;
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

/// Mark as decorative starts checked only when there is at least one object and every object is decorative.
#[trial]
fn decorative_starts_checked_only_when_every_object_is() {
    assert!(!AltText::starts_decorative([]));
    assert!(!AltText::starts_decorative([true, false]));
    assert!(!AltText::starts_decorative([false]));
    assert!(AltText::starts_decorative([true]));
    assert!(AltText::starts_decorative([true, true, true]));
}

/// Marking the alt text decorative keeps its text visible, disabled and out of the Tab order; unmarking makes it
/// editable again with the same text.
#[trial]
fn decorative_alt_text_stays_visible_and_is_not_a_tab_stop() {
    let (mut fixture, value, id) = rig(AltText::new("A red kite"));
    let text_id = craft_ui::testing::text::widget_id(id.get(), PartKey::new("text"));
    let disabled = |fixture: &Fixture| fixture.tree().is_disabled(text_id);
    assert!(!disabled(&fixture), "the description is editable at first");
    let check = fixture.by_label("Mark as decorative").expect("the check is shown");
    fixture.click(&check);
    fixture.idle(3);
    assert_eq!(value.borrow().decorative, Mixed::Same(true));
    assert!(disabled(&fixture), "a decorative object's text is disabled");
    assert!(fixture.texts().iter().any(|run| run.text.contains("A red kite")), "its text stays visible");
    fixture.key(Key::Tab, Modifiers::NONE);
    assert_ne!(fixture.focused(), Some(text_id), "the disabled text takes no Tab stop");
    fixture.click(&check);
    fixture.idle(3);
    assert_eq!(value.borrow().decorative, Mixed::Same(false));
    assert!(!disabled(&fixture), "unchecking makes the text editable");
    assert_eq!(value.borrow().text, Mixed::Same("A red kite".to_owned()), "the text was kept");
}

/// With the descriptions mixed and the flag mixed, the check's node reports a mixed state; checking it sets the flag
/// on every target, keeps the text mixed, and the node then reports checked.
#[trial]
fn checking_decorative_on_a_mixed_selection_marks_every_target_and_keeps_the_text_mixed() {
    let (mut fixture, value, id) = rig(AltText::parts(Mixed::Mixed, Mixed::Mixed));
    let check = fixture.part(id.get(), DECORATIVE).expect("the check has its own node");
    assert_eq!(fixture.tree().toggled_of(check.node), Some(Toggled::Mixed), "the check reports a mixed state");
    fixture.click(&check);
    fixture.idle(3);
    assert_eq!(*value.borrow(), AltText::parts(Mixed::Mixed, Mixed::Same(true)), "every target is decorative; the text stays per target");
    let check = fixture.part(id.get(), DECORATIVE).expect("the check keeps its node");
    assert_eq!(fixture.tree().toggled_of(check.node), Some(Toggled::True), "the check reports checked");
}
