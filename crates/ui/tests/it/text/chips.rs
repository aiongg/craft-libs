//! The chip field's draft rules and the alt text flag rule.

use craft_behavior::Codec as _;
use craft_ui::contract::{Bind, Show as _};
use craft_ui::testing::focus::active_part;
use craft_ui::testing::{Fixture, trial};
use craft_ui::text::{AltText, AltTextField, ChipCodec, ChipField};
use egui::{Key, Modifiers};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn chips(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

/// A draft names one chip per piece between separators, trimmed; pieces already in the list and empty pieces add
/// nothing, and the new chips follow the old ones in the order typed.
#[trial]
fn a_draft_adds_its_pieces_after_the_existing_chips() {
    let codec = ChipCodec::default();
    let start = chips(&["kite"]);
    assert_eq!(codec.parse("red, beach ;kite,, \n sky", &start), Ok(chips(&["kite", "red", "beach", "sky"])));
    assert_eq!(codec.parse("  ,; ", &start), Ok(start.clone()), "a draft of separators adds nothing");
    assert_eq!(codec.parse("red, red", &Vec::new()), Ok(chips(&["red"])), "a piece the draft names twice is added once");
    assert_eq!(codec.parse("two words", &Vec::new()), Ok(chips(&["two words"])), "a space does not end a chip");
}

/// The text part shows only the draft: committing leaves it empty, whatever the chips are.
#[trial]
fn the_draft_text_is_empty_after_a_commit() {
    let codec = ChipCodec::default();
    let value = chips(&["kite", "red"]);
    assert_eq!(codec.format(&value), "");
    assert_eq!(codec.normalise("red", &value), "");
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

/// Typing a comma commits the draft as a chip, empties the draft, and the comma is in no chip.
#[trial]
fn typing_a_comma_adds_the_chip_and_empties_the_draft() {
    let value = Rc::new(RefCell::new(Vec::<String>::new()));
    let shown = value.clone();
    let mut fixture = Fixture::new(move |ui| {
        ChipField::new("Keywords", Bind::bound(&mut *shown.borrow_mut())).show(ui);
    });
    fixture.settle();
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.type_text("red,");
    fixture.idle(3);
    assert_eq!(*value.borrow(), chips(&["red"]), "the comma ended the chip and is not part of it");
    assert!(fixture.by_label("Keywords").is_some());
    assert!(fixture.texts().iter().all(|run| !run.text.contains("red,")), "the draft shows no separator");
    fixture.type_text("blue;");
    fixture.idle(3);
    assert_eq!(*value.borrow(), chips(&["red", "blue"]), "a semicolon also commits");
}

/// Marking the alt text decorative keeps its text visible, disabled and out of the Tab order; unmarking makes it
/// editable again with the same text.
#[trial]
fn decorative_alt_text_stays_visible_and_is_not_a_tab_stop() {
    let value = Rc::new(RefCell::new(AltText::new("A red kite")));
    let id = Rc::new(Cell::new(egui::Id::NULL));
    let (shown, field) = (value.clone(), id.clone());
    let mut fixture = Fixture::new(move |ui| {
        field.set(AltTextField::new("Describe", Bind::bound(&mut *shown.borrow_mut())).show(ui).response.id);
    });
    fixture.settle();
    let text_id = craft_ui::testing::text::widget_id(id.get(), craft_behavior::PartKey::new("text"));
    let disabled = |fixture: &Fixture| fixture.tree().is_disabled(text_id);
    assert!(!disabled(&fixture), "the description is editable at first");
    let check = fixture.by_label("Mark as decorative").expect("the check is shown");
    fixture.click(&check);
    fixture.idle(3);
    assert!(value.borrow().decorative);
    assert!(disabled(&fixture), "a decorative object's text is disabled");
    assert!(fixture.texts().iter().any(|run| run.text.contains("A red kite")), "its text stays visible");
    fixture.key(Key::Tab, Modifiers::NONE);
    let widget = craft_ui::testing::text::widget_id(id.get(), craft_behavior::PartKey::new("text"));
    assert_ne!(fixture.focused(), Some(widget), "the disabled text takes no Tab stop");
    fixture.click(&check);
    fixture.idle(3);
    assert!(!value.borrow().decorative);
    assert!(!disabled(&fixture), "unchecking makes the text editable");
    assert_eq!(value.borrow().text, "A red kite", "the text was kept");
}

/// Backspace in an empty draft focuses the last chip and removes nothing; Backspace on that chip removes it and
/// returns focus to the draft; Delete in an empty draft does nothing.
#[trial]
fn backspace_in_an_empty_draft_focuses_the_last_chip_then_removes_it() {
    let value = Rc::new(RefCell::new(chips(&["kite", "red"])));
    let shown = value.clone();
    let field = Rc::new(Cell::new(None));
    let seen = field.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut chips = shown.borrow().clone();
        seen.set(Some(ChipField::new("Tags", Bind::bound(&mut chips)).show(ui).response.id));
        *shown.borrow_mut() = chips;
    });
    fixture.settle();
    let id = field.get().expect("the field showed");
    let part = |fixture: &Fixture| active_part(fixture.ctx(), id).and_then(|part| part.index);
    let press = |fixture: &mut Fixture, key: Key| {
        fixture.key(key, Modifiers::NONE);
        fixture.idle(2);
    };
    press(&mut fixture, Key::Tab);
    assert_eq!(part(&fixture), None, "Tab reaches the draft");
    fixture.key(Key::Tab, Modifiers::SHIFT);
    fixture.idle(2);
    assert_eq!(part(&fixture), Some(1), "Shift+Tab from the draft arrives at the chip next to it");
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(part(&fixture), None, "Tab returns to the draft");
    press(&mut fixture, Key::Delete);
    assert_eq!(*value.borrow(), chips(&["kite", "red"]), "Delete in an empty draft does nothing");
    assert_eq!(part(&fixture), None);
    press(&mut fixture, Key::Backspace);
    assert_eq!(*value.borrow(), chips(&["kite", "red"]), "the first Backspace removes nothing");
    assert_eq!(part(&fixture), Some(1), "it focuses the last chip");
    press(&mut fixture, Key::Backspace);
    assert_eq!(*value.borrow(), chips(&["kite"]), "Backspace on a focused chip removes it");
    assert_eq!(part(&fixture), None, "focus returns to the draft when no chip follows");
}
