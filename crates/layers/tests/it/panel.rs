//! `LayersPanel`: what a click, an Alt-click and a footer press report.

use craft_layers::ui::mock::FakeLayers;
use craft_layers::ui::panel::{LayersEvent, LayersPanel};
use craft_ui::testing::{Fixture, trial};
use std::cell::RefCell;
use std::rc::Rc;

type Seen = Rc<RefCell<Vec<LayersEvent<u32>>>>;

/// The sample tree in a panel whose events are collected.
fn rig() -> (Fixture, Seen) {
    let seen: Seen = Rc::new(RefCell::new(Vec::new()));
    let sink = Rc::clone(&seen);
    let tree = FakeLayers::sample();
    let mut fixture = Fixture::new(move |ui| {
        let out = LayersPanel::new(&tree).show(ui);
        sink.borrow_mut().extend(out.events);
    });
    fixture.settle();
    (fixture, seen)
}

/// A plain click on an eye toggles it, and selects nothing.
#[trial]
fn a_click_on_an_eye_toggles_it_without_selecting() {
    let (mut fixture, seen) = rig();
    let eye = fixture.by_label("Visibility Title").expect("the Title eye");
    fixture.click(&eye);
    fixture.idle(2);
    let events = seen.borrow();
    assert!(events.iter().any(|event| matches!(event, LayersEvent::Toggle(_))), "events: {events:?}");
    assert!(!events.iter().any(|event| matches!(event, LayersEvent::Select(_))), "events: {events:?}");
}
