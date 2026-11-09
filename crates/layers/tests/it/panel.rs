//! `LayersPanel`: what a click, an Alt-click and a footer press report.

use craft_behavior::SelectionChange;
use craft_layers::ui::mock::FakeLayers;
use craft_layers::ui::panel::{LayersEvent, LayersFooterAction, LayersPanel};
use craft_layers::ui::tree::LayerRowKey;
use craft_ui::testing::{Fixture, trial};
use egui::Modifiers;
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

/// A click on a row selects that layer alone.
#[trial]
fn a_click_on_a_row_selects_the_layer() {
    let (mut fixture, seen) = rig();
    let row = fixture.by_label("Title").expect("the Title row");
    fixture.click(&row);
    fixture.idle(2);
    let picked = seen.borrow().iter().any(|event| matches!(event, LayersEvent::Select(select) if select.change == SelectionChange::Set(vec![4])));
    assert!(picked, "events: {:?}", seen.borrow());
}

/// A plain click on an eye toggles it, and selects nothing; Alt-click shows only that layer instead of toggling it.
#[trial]
fn a_click_on_an_eye_toggles_it_without_selecting() {
    let (mut fixture, seen) = rig();
    let eye = fixture.by_label("Visibility Title").expect("the Title eye");
    fixture.click(&eye);
    fixture.idle(2);
    let events = seen.borrow();
    assert!(events.iter().any(|event| matches!(event, LayersEvent::Toggle(_))), "events: {events:?}");
    assert!(!events.iter().any(|event| matches!(event, LayersEvent::Select(_))), "events: {events:?}");
    drop(events);

    let (mut fixture, seen) = rig();
    let eye = fixture.by_label("Visibility Title").expect("the Title eye");
    fixture.hold(Modifiers::ALT);
    fixture.click(&eye);
    fixture.idle(2);
    let events = seen.borrow();
    assert!(events.iter().any(|event| matches!(event, LayersEvent::ShowOnly(LayerRowKey::Layer(4)))), "events: {events:?}");
    assert!(!events.iter().any(|event| matches!(event, LayersEvent::Toggle(_))), "events: {events:?}");
}

/// Alt on New Layer asks for the options; Cmd adds below.
#[trial]
fn new_layer_reports_the_modifiers_held() {
    let (mut fixture, seen) = rig();
    let button = fixture.by_label("Create a new layer").expect("the New Layer button");
    fixture.hold(Modifiers::ALT);
    fixture.click(&button);
    fixture.idle(2);
    let asked = seen.borrow().iter().any(|event| matches!(event, LayersEvent::Footer(LayersFooterAction::NewLayer { ask: true, below: false, .. })));
    assert!(asked, "events: {:?}", seen.borrow());
}
