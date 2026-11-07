//! `LinkedFields`: the ratio and equal rules, and a typed edit keeping the gesture's starting ratio.

use craft_behavior::{EditEvent, Mixed};
use craft_ui::contract::{Bind, Show as _};
use craft_ui::testing::{Fixture, trial};
use craft_ui::text::{LinkedFields, follow_equal, follow_ratio};
use egui::{Key, Modifiers};
use std::cell::{Cell, RefCell};
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

type Parts = [Mixed<Option<f64>>; 2];
type Values = Rc<Cell<Parts>>;

/// Both fields shared.
fn same(width: f64, height: f64) -> Parts {
    Mixed::each([Some(width), Some(height)])
}

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
    let values: Values = Rc::new(Cell::new(same(500.0, 400.0)));
    let mut fixture = rig(&values);
    fixture.type_text("100");
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(values.get(), same(100.0, 80.0));
}

/// Escape ends the typing and every field returns to the values the gesture began with.
#[trial]
fn escape_restores_every_linked_field() {
    let values: Values = Rc::new(Cell::new(same(400.0, 300.0)));
    let mut fixture = rig(&values);
    fixture.type_text("40");
    fixture.key(Key::Escape, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(values.get(), same(400.0, 300.0));
}

/// Clicking the link writes the bound bool through the child: one edit event with the link's label.
#[trial]
fn clicking_the_link_writes_the_bool_through_the_child() {
    let linked = Rc::new(Cell::new(false));
    let events = Rc::new(RefCell::new(Vec::new()));
    let (shown, seen) = (linked.clone(), events.clone());
    let mut fixture = Fixture::new(move |ui| {
        let mut on = shown.get();
        let out = LinkedFields::new("Size", Bind::given(same(10.0, 20.0)), Bind::bound(&mut on)).captions(["W", "H"]).show(ui);
        shown.set(on);
        if let Some(event) = out.link {
            seen.borrow_mut().push(event);
        }
    });
    fixture.settle();
    let toggle = fixture.by_label("Maintain aspect ratio").expect("the link toggle is named");
    fixture.click(&toggle);
    fixture.idle(2);
    assert!(linked.get(), "the bound bool is written");
    let events = events.borrow();
    assert_eq!(events.len(), 1, "one edit event: {events:?}");
    assert!(matches!(&events[0], EditEvent::Set { label, .. } if &**label == "Maintain aspect ratio"), "the link's label: {events:?}");
}

/// With the link on, editing one field moves the other.
#[trial]
fn with_the_link_on_editing_one_field_moves_the_other() {
    let values: Values = Rc::new(Cell::new(same(100.0, 50.0)));
    let mut fixture = rig(&values);
    fixture.type_text("200");
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(values.get(), same(200.0, 100.0));
}

/// On a selection where only the height differs, typing a width commits the width alone: `[Same(width), Mixed]` as one
/// edit. The width shows its value.
#[trial]
fn typing_a_width_on_a_selection_with_mixed_heights_commits_the_width_alone() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let shown = Rc::new(Cell::new([Mixed::Same(Some(100.0)), Mixed::Mixed]));
    let (seen, parts) = (events.clone(), shown.clone());
    let mut fixture = Fixture::new(move |ui| {
        let mut current = parts.get();
        let out = LinkedFields::new("Size", Bind::bound(&mut current), Bind::given(false)).captions(["W", "H"]).show(ui);
        parts.set(current);
        seen.borrow_mut().extend(out.edit.event.clone());
    });
    fixture.settle();
    let texts = fixture.texts();
    assert!(texts.iter().any(|run| run.text == "100"), "the width shows its value");
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.type_text("5");
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(shown.get(), [Mixed::Same(Some(5.0)), Mixed::Mixed], "the height part stays mixed");
    let events = events.borrow();
    let commits: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            EditEvent::Commit { after, .. } | EditEvent::Set { after, .. } => Some(*after),
            _ => None,
        })
        .collect();
    assert_eq!(commits, vec![[Mixed::Same(Some(5.0)), Mixed::Mixed]], "one edit: {events:?}");
}
