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

