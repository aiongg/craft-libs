//! `Rulers`: the tick rule, the guide drag-out and the corner's zero point.

use craft_behavior::kurbo::Point;
use craft_behavior::{Axis, CanvasView, GesturePhase, RulerUnits, Ticks};
use craft_ui::canvas::{Canvas, GuideDrag, INCH_EIGHTHS, RulerPart, RulerText, Rulers, TickRule};
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::runtime::env::UnitKind;
use craft_ui::testing::{Fixture, pointer, trial};
use egui::{Key, Modifiers, Pos2, Rect, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

/// The view's rect in the fixture, in points.
const VIEW: Rect = Rect::from_min_max(Pos2::new(30.0, 30.0), Pos2::new(390.0, 290.0));

/// How far a drag from a strip goes: out of the strip and onto the document.
const ONTO_DOCUMENT: Vec2 = Vec2::new(0.0, 100.0);

/// How far a drag along a strip goes: not out of it.
const ALONG_STRIP: Vec2 = Vec2::new(40.0, 0.0);

/// The corner's drag: out of the corner and onto the document.
const FROM_CORNER: Vec2 = Vec2::new(90.0, 90.0);

/// The units the menu offers.
const MENU_UNITS: [UnitKind; 2] = [UnitKind::Pixels, UnitKind::Points];

/// The menu's last row, which is there only while the units menu is open.
const MENU_ROW: &str = "Reset Zero Point";

/// The minor tick spacing of the pixel ruler at 100 percent, in document pixels.
const PIXEL_MINOR: f64 = 10.0;

fn pixels() -> RulerText {
    RulerText::new(RulerUnits::pixels(), UnitKind::Pixels)
}

fn majors(ticks: &[craft_ui::canvas::RulerTick]) -> Vec<f64> {
    ticks.iter().filter(|tick| tick.major).map(|tick| tick.value).collect()
}

#[trial]
fn the_pixel_ruler_labels_every_hundred_and_ticks_every_ten_at_full_size() {
    let ticks = Rulers::ticks(&RulerUnits::pixels(), TickRule::Metric, 1.0, 0.0..=200.0);
    let majors = majors(&ticks);
    assert!(majors.contains(&0.0) && majors.contains(&100.0) && majors.contains(&200.0), "{majors:?}");
    assert!(majors.iter().all(|value| value.rem_euclid(100.0) == 0.0), "labels sit on multiples of 100: {majors:?}");
    let gaps: Vec<f64> = ticks.windows(2).map(|pair| pair[1].value - pair[0].value).collect();
    assert!(gaps.iter().all(|gap| (gap - PIXEL_MINOR).abs() < 1e-9), "ticks are ten apart: {gaps:?}");
}

#[trial]
fn zooming_in_labels_closer_values() {
    let zoomed = majors(&Rulers::ticks(&RulerUnits::pixels(), TickRule::Metric, 4.0, 0.0..=200.0));
    assert!(zoomed.windows(2).all(|pair| (pair[1] - pair[0] - 20.0).abs() < 1e-9), "labels are twenty apart at 400 percent: {zoomed:?}");
}

#[trial]
fn the_inch_rule_labels_each_inch_and_ticks_every_eighth() {
    let inches = RulerUnits::new(1.0 / 96.0, Ticks::decimal(false));
    let ticks = Rulers::ticks(&inches, TickRule::Inches(&INCH_EIGHTHS), 1.0, 0.0..=3.0);
    assert!(majors(&ticks).iter().all(|value| value.fract() == 0.0), "labels sit on whole inches");
    assert!(ticks.windows(2).all(|pair| (pair[1].value - pair[0].value - 0.125).abs() < 1e-9), "ticks are an eighth of an inch apart");
}

/// A canvas with rulers over a view at 100 percent; each guide drag and the zero point are kept.
struct Shown {
    fixture: Fixture,
    guides: Rc<RefCell<Vec<GuideDrag>>>,
    zero: Rc<RefCell<Point>>,
    contexts: Rc<RefCell<Vec<RulerPart>>>,
}

fn shown(locked: bool) -> Shown {
    let guides: Rc<RefCell<Vec<GuideDrag>>> = Rc::default();
    let zero = Rc::new(RefCell::new(Point::ZERO));
    let contexts: Rc<RefCell<Vec<RulerPart>>> = Rc::default();
    let (seen, edited, asked) = (Rc::clone(&guides), Rc::clone(&zero), Rc::clone(&contexts));
    let mut fixture = Fixture::new(move |ui| {
        let text = pixels();
        let view = CanvasView::new(VIEW, Point::new(180.0, 130.0), 1.0);
        Canvas::new("Document").view(view).rulers(text, text, Point::ZERO).show(ui, |ui, body| {
            let out = Rulers::new("Rulers", Bind::bound(&mut *edited.borrow_mut())).origin_locked(locked).units(&MENU_UNITS).show_at(ui, body.rect);
            seen.borrow_mut().extend(out.guide);
            asked.borrow_mut().extend(out.context);
        });
    });
    fixture.settle();
    Shown { fixture, guides, zero, contexts }
}

#[trial]
fn a_drag_from_the_top_ruler_places_a_horizontal_guide_and_ends_on_the_document() {
    let mut shown = shown(false);
    let strip = shown.fixture.by_label("Horizontal ruler").expect("the top ruler has a node");
    shown.fixture.drag(&strip, ONTO_DOCUMENT);
    let guides = shown.guides.borrow();
    let (first, last) = (guides.first().expect("the drag is reported"), guides.last().expect("the drag is reported"));
    assert_eq!(first.phase, GesturePhase::Begin);
    assert_eq!(last.phase, GesturePhase::Commit);
    assert!(guides.iter().all(|drag| drag.along == Axis::X && drag.key.is_none()), "the top ruler makes new horizontal guides");
    assert!(last.at.is_some(), "released on the document, the guide stays");
}

#[trial]
fn a_guide_released_over_the_ruler_has_no_position() {
    let mut shown = shown(false);
    let strip = shown.fixture.by_label("Vertical ruler").expect("the left ruler has a node");
    shown.fixture.drag(&strip, Vec2::new(0.0, ALONG_STRIP.x));
    let guides = shown.guides.borrow();
    let last = guides.last().expect("the drag is reported");
    assert_eq!((last.phase, last.at, last.along), (GesturePhase::Commit, None, Axis::Y), "a release over a ruler removes the guide");
}

#[trial]
fn shift_snaps_a_dragged_guide_to_the_ticks() {
    let mut shown = shown(false);
    let strip = shown.fixture.by_label("Horizontal ruler").expect("the top ruler has a node");
    shown.fixture.hold(Modifiers::SHIFT);
    shown.fixture.drag(&strip, ONTO_DOCUMENT + Vec2::new(0.0, 3.7));
    let at = shown.guides.borrow().last().and_then(|drag| drag.at).expect("released on the document");
    assert!((at / PIXEL_MINOR - (at / PIXEL_MINOR).round()).abs() < 1e-9, "{at} is not on a tick");
}

#[trial]
fn dragging_the_corner_moves_the_zero_point_unless_it_is_locked() {
    let mut shown = shown(false);
    let corner = shown.fixture.by_label("Ruler origin").expect("the corner has a node");
    shown.fixture.drag(&corner, FROM_CORNER);
    assert_ne!(*shown.zero.borrow(), Point::ZERO, "the zero point followed the drag");

    let mut locked = self::shown(true);
    let corner = locked.fixture.by_label("Ruler origin").expect("the corner has a node");
    locked.fixture.drag(&corner, FROM_CORNER);
    assert_eq!(*locked.zero.borrow(), Point::ZERO, "a locked zero point stays");
}

#[trial]
fn a_double_click_on_the_corner_puts_the_bound_zero_back_at_the_origin() {
    let mut shown = shown(false);
    let corner = shown.fixture.by_label("Ruler origin").expect("the corner has a node");
    shown.fixture.drag(&corner, FROM_CORNER);
    assert_ne!(*shown.zero.borrow(), Point::ZERO, "the zero point followed the drag");
    let at = corner.center();
    for events in
        [vec![pointer::moved(at)], vec![pointer::press(at)], vec![pointer::release(at)], vec![pointer::press(at)], vec![pointer::release(at)]]
    {
        shown.fixture.frame(events);
    }
    assert_eq!(*shown.zero.borrow(), Point::ZERO, "the double click wrote the origin through the bind");
}

#[trial]
fn shift_f10_on_the_focused_corner_opens_the_units_menu() {
    let mut shown = shown(false);
    let corner = shown.fixture.by_label("Ruler origin").expect("the corner has a node");
    shown.fixture.click(&corner);
    assert!(shown.fixture.by_label(MENU_ROW).is_none(), "the menu is closed until it is asked for");
    shown.fixture.key(Key::F10, Modifiers::SHIFT);
    shown.fixture.settle();
    assert_eq!(shown.contexts.borrow().last(), Some(&RulerPart::Corner), "the focused corner reports the request");
    assert!(shown.fixture.by_label(MENU_ROW).is_some(), "the units menu is open");
}

#[trial]
fn a_secondary_click_on_a_strip_opens_the_units_menu() {
    let mut shown = shown(false);
    let strip = shown.fixture.by_label("Horizontal ruler").expect("the top ruler has a node");
    let at = strip.center();
    shown.fixture.frame(vec![pointer::moved(at)]);
    shown.fixture.frame(vec![pointer::secondary(at, true)]);
    shown.fixture.frame(vec![pointer::secondary(at, false)]);
    shown.fixture.settle();
    assert_eq!(shown.contexts.borrow().last(), Some(&RulerPart::Horizontal));
    assert!(shown.fixture.by_label(MENU_ROW).is_some(), "the units menu is open");
}

#[trial]
fn a_secondary_click_on_the_locked_corner_opens_the_units_menu_and_one_on_the_document_does_not() {
    let mut shown = shown(true);
    let secondary_click = |shown: &mut Shown, at: Pos2| {
        shown.fixture.frame(vec![pointer::moved(at)]);
        shown.fixture.frame(vec![pointer::secondary(at, true)]);
        shown.fixture.frame(vec![pointer::secondary(at, false)]);
        shown.fixture.settle();
    };
    secondary_click(&mut shown, VIEW.center());
    assert!(shown.contexts.borrow().is_empty(), "a secondary click on the document is no ruler request");

    let corner = shown.fixture.by_label("Ruler origin").expect("the corner has a node");
    secondary_click(&mut shown, corner.center());
    assert_eq!(shown.contexts.borrow().last(), Some(&RulerPart::Corner));
    assert!(shown.fixture.by_label(MENU_ROW).is_some(), "the units menu is open");
}
