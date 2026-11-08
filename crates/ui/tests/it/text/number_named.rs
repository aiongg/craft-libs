//! A number field over `NamedOr`: the presets list names values beside numbers.

use craft_ui::contract::{Bind, Entry, Props as _, Show as _};
use craft_ui::testing::{Fixture, trial};
use craft_ui::text::{NamedOr, NumberField, NumberName, NumberPick};
use egui::{Key, Modifiers};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kerning {
    Metrics,
    Optical,
}

impl NumberName for Kerning {
    fn id(self) -> u32 {
        self as u32
    }

    fn from_id(id: u32) -> Option<Self> {
        [Kerning::Metrics, Kerning::Optical].into_iter().find(|name| name.id() == id)
    }
}

const START: f64 = 10.0;
const TYPED: f64 = 25.0;

fn rig() -> (Fixture, Rc<RefCell<NamedOr<Kerning>>>) {
    let value = Rc::new(RefCell::new(NamedOr::Number(START)));
    let shown = value.clone();
    let presets = vec![
        Entry::new(NumberPick::Choice(Kerning::Metrics.id()), "Metrics"),
        Entry::new(NumberPick::Choice(Kerning::Optical.id()), "Optical"),
        Entry::new(NumberPick::Number(0.0), "0"),
    ];
    let mut fixture = Fixture::new(move |ui| {
        let mut guard = shown.borrow_mut();
        NumberField::named("Kerning", Bind::bound(&mut *guard)).id_salt("kerning").presets(&presets).show(ui);
    });
    fixture.settle();
    (fixture, value)
}

/// Picking a named row holds the name, typing a number holds the number.
#[trial]
fn a_named_row_gives_the_name_and_typing_gives_the_number() {
    let (mut fixture, value) = rig();
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.idle(2);
    fixture.key(Key::ArrowDown, Modifiers::ALT);
    fixture.idle(4);
    fixture.key(Key::ArrowDown, Modifiers::NONE);
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(3);
    assert_eq!(*value.borrow(), NamedOr::Named(Kerning::Optical), "the picked row is the value");

    let (mut fixture, value) = rig();
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.idle(2);
    fixture.type_text("25");
    fixture.idle(2);
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(3);
    assert_eq!(*value.borrow(), NamedOr::Number(TYPED), "typed digits are the number");
}
