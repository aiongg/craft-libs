//! The Brush Settings panel's rows: the tip angle widget, the stroke test data and the section list.

use craft_draw::ui::brush_settings::{
    BrushSection, BrushSectionEvent, BrushSectionList, PreviewStroke, TipAngle, TipEllipse, TipEllipseGrab, ellipse_drag, ellipse_grab,
    ellipse_handles,
};
use craft_ui::contract::{Bind, Show as _};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const RADIUS: f32 = 34.0;
const REACH: f32 = 7.0;

/// A press within reach of a roundness dot grabs the dot; anywhere else on the disc grabs the arrow.
#[trial]
fn ellipse_grab_prefers_the_dots_within_the_reach() {
    let angle = TipAngle::new(0.0, 0.5);
    let [dot, _] = ellipse_handles(angle, RADIUS);
    assert_eq!(ellipse_grab(dot + vec2(REACH - 1.0, 0.0), angle, RADIUS, REACH), TipEllipseGrab::Roundness);
    assert_eq!(ellipse_grab(dot + vec2(REACH + 1.0, 0.0), angle, RADIUS, REACH), TipEllipseGrab::Angle);
}

/// A drag of the arrow sets the angle from the pointer; a drag of a dot sets the roundness and keeps the angle.
#[trial]
fn ellipse_drag_sets_angle_and_roundness() {
    let start = TipAngle::new(10.0, 0.5);
    let turned = ellipse_drag(TipEllipseGrab::Angle, vec2(0.0, -20.0), start, RADIUS);
    assert_eq!((turned.degrees, turned.roundness), (90.0, 0.5));
    let squashed = ellipse_drag(TipEllipseGrab::Roundness, vec2(0.0, -RADIUS), TipAngle::new(0.0, 0.5), RADIUS);
    assert_eq!((squashed.degrees, squashed.roundness), (0.0, 1.0));
    assert_eq!(ellipse_drag(TipEllipseGrab::Angle, vec2(0.5, 0.0), start, RADIUS), start);
}

/// Up and Right turn the angle 1 degree, Shift 15, wrapping at 180; Alt changes the roundness by 1 %.
#[trial]
fn ellipse_keys_turn_and_squash() {
    let value = Rc::new(Cell::new(TipAngle::new(179.0, 0.5)));
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
    assert_eq!(value.get().degrees, 180.0);
    fixture.key(Key::ArrowUp, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(value.get().degrees, -179.0);
    fixture.key(Key::ArrowDown, Modifiers::SHIFT);
    fixture.idle(2);
    assert_eq!(value.get().degrees, 166.0);
    fixture.key(Key::ArrowUp, Modifiers::ALT);
    fixture.idle(2);
    assert_eq!(value.get().roundness, 0.51);
}

/// The preview stroke is the S-curve: its samples start and end at the floor pressure and swing the amplitude.
#[trial]
fn preview_stroke_samples_match_the_s_curve() {
    let stroke = PreviewStroke::new(200, 100);
    let samples = stroke.samples();
    assert_eq!(samples.len(), PreviewStroke::SAMPLES);
    assert_eq!(samples[0].pressure, PreviewStroke::PRESSURE_FLOOR);
    assert_eq!(samples[0].x, PreviewStroke::MARGIN);
    let quarter = &samples[PreviewStroke::SAMPLES / 4];
    assert!((quarter.y - (50.0 - 100.0 * PreviewStroke::AMPLITUDE)).abs() < 1e-6);
    assert_eq!(stroke.fit(10.0), 1.0);
    assert!(stroke.fit(400.0) < 1.0);
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
        let out = BrushSectionList::new("Brush settings sections", &sections, Bind::given(0u8)).link().show(ui);
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
    assert!(seen.iter().any(|event| matches!(event, BrushSectionEvent::Enable { key: 1, on: true })));
}

/// A click on the link row opens the Brushes panel and nothing else.
#[trial]
fn the_link_row_reports_open_link() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut fixture = rig(&events);
    let target = fixture.by_label("Brushes");
    assert!(target.is_some(), "the link row is in the access tree");
    let Some(target) = target else { return };
    fixture.click(&target);
    fixture.idle(2);
    assert_eq!(*events.borrow(), vec![BrushSectionEvent::OpenLink]);
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
    assert!(seen.iter().any(|event| matches!(event, BrushSectionEvent::Switch { key: 10, on: true })));
    assert!(!seen.iter().any(|event| matches!(event, BrushSectionEvent::Select(_))));
}
