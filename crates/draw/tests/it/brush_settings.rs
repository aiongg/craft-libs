//! The Brush Settings panel's rows: the tip angle widget, the stroke test data and the section list.

use craft_draw::ui::brush_settings::{BrushSection, BrushSectionEvent, BrushSectionList, TipAngle, TipEllipse, TipEllipseGrab, ellipse_drag};
use craft_ui::contract::{Bind, Show as _};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, Vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const RADIUS: f32 = 34.0;
const PULL: f32 = 20.0;
const NEAR_CENTRE: f32 = 0.5;
/// The angles and roundness the key trial walks through.
const START: TipAngle = TipAngle::new(179.0, 0.5);
const TURNED_UP: f64 = 180.0;
const WRAPPED: f64 = -179.0;
const TURNED_BACK_COARSE: f64 = 166.0;
const ROUNDER: f64 = 0.51;

/// A drag of the arrow sets the angle from the pointer; a drag of a dot sets the roundness and keeps the angle.
#[trial]
fn ellipse_drag_sets_angle_and_roundness() {
    let start = TipAngle::new(10.0, 0.5);
    let turned = ellipse_drag(TipEllipseGrab::Angle, -Vec2::Y * PULL, start, RADIUS);
    assert_eq!((turned.degrees, turned.roundness), (90.0, 0.5));
    let squashed = ellipse_drag(TipEllipseGrab::Roundness, -Vec2::Y * RADIUS, TipAngle::new(0.0, 0.5), RADIUS);
    assert_eq!((squashed.degrees, squashed.roundness), (0.0, 1.0));
    assert_eq!(ellipse_drag(TipEllipseGrab::Angle, Vec2::X * NEAR_CENTRE, start, RADIUS), start);
}

/// Up and Right turn the angle 1 degree, Shift 15, wrapping at 180; Alt changes the roundness by 1 %.
#[trial]
fn ellipse_keys_turn_and_squash() {
    let value = Rc::new(Cell::new(START));
    let shown = value.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut current = shown.get();
        let _ = TipEllipse::new("Tip angle and roundness", Bind::bound(&mut current)).show(ui);
        shown.set(current);
    });
    fixture.settle();
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.key(Key::ArrowUp, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(value.get().degrees, TURNED_UP);
    fixture.key(Key::ArrowUp, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(value.get().degrees, WRAPPED);
    fixture.key(Key::ArrowDown, Modifiers::SHIFT);
    fixture.idle(2);
    assert_eq!(value.get().degrees, TURNED_BACK_COARSE);
    fixture.key(Key::ArrowUp, Modifiers::ALT);
    fixture.idle(2);
    assert_eq!(value.get().roundness, ROUNDER);
}

/// Shows the list over a few sections and collects the events it reports.
fn rig(events: &Rc<RefCell<Vec<BrushSectionEvent<u8>>>>) -> Fixture {
    let sink = events.clone();
    let mut fixture = Fixture::new(move |ui| {
        let sections = [
            BrushSection::pane(0u8, "Brush Tip Shape"),
            BrushSection::pane(1, "Shape Dynamics").enabled(false).locked(false),
            BrushSection::switch(10, "Noise", false),
        ];
        let out = BrushSectionList::new("Brush settings sections", &sections, Bind::given(0u8)).link_row().show(ui);
        sink.borrow_mut().extend(out.events);
    });
    fixture.settle();
    fixture
}

/// A click on an off pane row shows its pane and turns the section on.
#[trial]
fn a_click_on_a_pane_row_selects_and_enables_it() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut fixture = rig(&events);
    let target = fixture.by_label("Shape Dynamics");
    assert!(target.is_some(), "the pane row is in the access tree");
    let Some(target) = target else { return };
    fixture.click(&target);
    fixture.idle(2);
    let seen = events.borrow();
    assert!(seen.contains(&BrushSectionEvent::Select(1)));
    assert!(seen.iter().any(|event| matches!(event, BrushSectionEvent::Enable { key: 1, on: true, .. })));
}

/// A click on a switch row flips the switch and never selects it.
#[trial]
fn switch_rows_only_toggle() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut fixture = rig(&events);
    let target = fixture.by_label("Noise");
    assert!(target.is_some(), "the switch row is in the access tree");
    let Some(target) = target else { return };
    fixture.click(&target);
    fixture.idle(2);
    let seen = events.borrow();
    assert!(seen.iter().any(|event| matches!(event, BrushSectionEvent::Switch { key: 10, on: true, .. })));
    assert!(!seen.iter().any(|event| matches!(event, BrushSectionEvent::Select(_))));
}
