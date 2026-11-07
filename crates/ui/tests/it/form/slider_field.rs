//! `SliderField`: the step rules of the number field beside the slider.

use craft_ui::contract::{Bind, Show as _};
use craft_ui::form::SliderField;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::Cell;
use std::ops::RangeInclusive;
use std::rc::Rc;

const PERCENT: RangeInclusive<f64> = 0.0..=100.0;
const UNIT: RangeInclusive<f64> = 0.0..=1.0;

/// A field over `range` with `step`, the value kept in `value` between frames, with focus on the number field.
fn rig(value: &Rc<Cell<f64>>, range: RangeInclusive<f64>, step: Option<f64>) -> Fixture {
    let shown = value.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut current = shown.get();
        let field = SliderField::new("Opacity", Bind::bound(&mut current), range.clone());
        match step {
            Some(step) => field.step(step).show(ui),
            None => field.show(ui),
        };
        shown.set(current);
    });
    fixture.settle();
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture
}

fn typed(value: &Rc<Cell<f64>>, mut fixture: Fixture, text: &str) -> f64 {
    fixture.type_text(text);
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(2);
    value.get()
}

/// A set step is the grid typed values go to.
#[trial]
fn a_typed_value_snaps_to_the_set_step() {
    let value = Rc::new(Cell::new(10.0));
    let fixture = rig(&value, PERCENT, Some(5.0));
    assert_eq!(typed(&value, fixture, "42"), 40.0);
}

/// Without a step a typed value keeps the places the default step needs.
#[trial]
fn a_typed_value_rounds_to_the_default_step() {
    let value = Rc::new(Cell::new(0.5));
    let fixture = rig(&value, UNIT, None);
    assert_eq!(typed(&value, fixture, "0.456"), 0.46);
}

/// A step that is not a positive number is ignored.
#[trial]
fn an_unusable_step_is_ignored() {
    let value = Rc::new(Cell::new(10.0));
    let fixture = rig(&value, PERCENT, Some(f64::NAN));
    assert_eq!(typed(&value, fixture, "42"), 42.0);
}
