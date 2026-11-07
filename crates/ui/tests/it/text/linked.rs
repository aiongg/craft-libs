//! `LinkedFields`: the ratio and equal rules, and a typed edit keeping the gesture's starting ratio.

use craft_ui::contract::{Bind, Show as _};
use craft_ui::testing::{Fixture, trial};
use craft_ui::text::{LinkedFields, follow_equal, follow_ratio};
use egui::{Key, Modifiers};
use std::cell::Cell;
use std::ops::RangeInclusive;
use std::rc::Rc;

const UNBOUNDED: RangeInclusive<f64> = f64::NEG_INFINITY..=f64::INFINITY;

/// The factor is lowered so no value leaves its range, and an old zero scales nothing.
#[trial]
fn a_ratio_link_scales_the_others_within_the_range() {
    let free = [UNBOUNDED, UNBOUNDED];
    assert_eq!(follow_ratio([100.0, 50.0], 0, 200.0, &free), [200.0, 100.0]);
    assert_eq!(follow_ratio([100.0, 50.0], 1, 25.0, &free), [50.0, 25.0]);
    let capped = [0.0..=150.0, 0.0..=80.0];
    assert_eq!(follow_ratio([100.0, 50.0], 0, 200.0, &capped), [150.0, 75.0], "the first reaches its limit and the second keeps the ratio");
    assert_eq!(follow_ratio([100.0, 50.0], 1, 100.0, &capped), [150.0, 75.0], "the second would pass the first's limit");
    assert_eq!(follow_ratio([100.0, 0.0], 1, 10.0, &free), [100.0, 10.0], "an old zero scales nothing");
    assert_eq!(follow_ratio([500.0, 400.0], 0, 100.0, &free), [100.0, 80.0], "no rounding noise");
}

#[trial]
fn an_equal_link_copies_the_value() {
    assert_eq!(follow_equal([1.0, 2.0, 3.0, 4.0], 2, 9.0), [9.0; 4]);
}

type Values = Rc<Cell<[Option<f64>; 2]>>;

/// Linked width and height with the link on, the values kept in `values` between frames.
fn rig(values: &Values) -> Fixture {
    let shown = values.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut current = shown.get();
        LinkedFields::new("Size", Bind::bound(&mut current), Bind::given(true)).captions(["W", "H"]).show(ui);
        shown.set(current);
    });
    fixture.settle();
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture
}

/// Digits typed one by one into the width scale the height from the values the typing began with, not from the
/// previous digit.
#[trial]
fn typing_into_a_linked_field_keeps_the_starting_ratio() {
    let values: Values = Rc::new(Cell::new([Some(500.0), Some(400.0)]));
    let mut fixture = rig(&values);
    fixture.type_text("100");
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(values.get(), [Some(100.0), Some(80.0)]);
}

/// Escape ends the typing and every field returns to the values the gesture began with.
#[trial]
fn escape_restores_every_linked_field() {
    let values: Values = Rc::new(Cell::new([Some(400.0), Some(300.0)]));
    let mut fixture = rig(&values);
    fixture.type_text("40");
    fixture.key(Key::Escape, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(values.get(), [Some(400.0), Some(300.0)]);
}
