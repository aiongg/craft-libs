//! `SliderField`: the step rules of the number field beside the slider.

use craft_behavior::Mixed;
use craft_ui::contract::{Bind, Role, Show as _};
use craft_ui::form::SliderField;
use craft_ui::testing::pointer::{moved, press, release};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
use std::cell::{Cell, RefCell};
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

#[trial]
fn a_typed_value_snaps_to_the_step_set_or_default() {
    {
        // A set step is the grid typed values go to.
        let value = Rc::new(Cell::new(10.0));
        let fixture = rig(&value, PERCENT, Some(5.0));
        assert_eq!(typed(&value, fixture, "42"), 40.0);
    }
    {
        // Without a step a typed value keeps the places the default step needs.
        let value = Rc::new(Cell::new(0.5));
        let fixture = rig(&value, UNIT, None);
        assert_eq!(typed(&value, fixture, "0.456"), 0.46);
    }
}

/// A step that is not a positive number is ignored.
#[trial]
fn an_unusable_step_is_ignored() {
    let value = Rc::new(Cell::new(10.0));
    let fixture = rig(&value, PERCENT, Some(f64::NAN));
    assert_eq!(typed(&value, fixture, "42"), 42.0);
}

/// Clearing the number field of a mixed slider field and pressing Enter changes no target: empty text is not a
/// value while the targets differ.
#[trial]
fn clearing_a_mixed_field_leaves_every_target_unchanged() {
    let targets = Rc::new(RefCell::new(Mixed::Mixed));
    let shown = targets.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut current = *shown.borrow();
        SliderField::new("Opacity", Bind::bound_mixed(&mut current), PERCENT).step(5.0).show(ui);
        *shown.borrow_mut() = current;
    });
    fixture.settle();
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.type_text("5");
    fixture.key(Key::Backspace, Modifiers::NONE);
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(*targets.borrow(), Mixed::Mixed, "the targets stay mixed");
}

/// A double click on the slider restores the binding's reset value, not the value a single click jumps to.
#[trial]
fn a_double_click_on_the_slider_resets_to_the_binding_reset_value() {
    const RESET: f64 = 25.0;
    let value = Rc::new(Cell::new(80.0));
    let shown = value.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut current = shown.get();
        SliderField::new("Opacity", Bind::bound(&mut current).reset(RESET), PERCENT).show(ui);
        shown.set(current);
    });
    fixture.settle();
    let at = fixture.by_role(Role::SLIDER).first().expect("the field shows a slider").center();
    fixture.frame(vec![moved(at)]);
    for _ in 0..2 {
        fixture.frame(vec![press(at)]);
        fixture.frame(vec![release(at)]);
    }
    fixture.idle(2);
    assert_eq!(value.get(), RESET);
}
