//! `CanvasScrollbars`: the clamp of the view and the bars' drag and page press.

use craft_behavior::CanvasView;
use craft_behavior::kurbo::{Point, Rect as DocRect};
use craft_ui::canvas::{Canvas, CanvasScrollbars};
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::testing::{Fixture, pointer, trial};
use egui::{Pos2, Rect, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

/// The view's rect in the fixture, in points.
const VIEW: Rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(400.0, 300.0));

const ZOOM: f64 = 2.0;

/// Where a thumb at the start of its track is pressed, from the start of the track, in points.
const GRIP: f32 = 8.0;

/// The slack of a rect comparison after f32 maths, in points.
const TOLERANCE: f32 = 0.01;

/// How far a thumb is dragged, in points.
const DRAG: f32 = 20.0;

fn view_at(center: Point, zoom: f64) -> CanvasView {
    CanvasView::new(VIEW, center, zoom)
}

#[trial]
fn without_overscroll_the_view_stays_on_the_document() {
    let large = DocRect::new(0.0, 0.0, 1000.0, 800.0);
    let mut view = view_at(Point::new(-50.0, 100.0), 1.0);
    assert!(CanvasScrollbars::clamp(&mut view, large), "a view past the edge moves");
    assert_eq!(view.center, Point::new(200.0, 150.0), "the view's edges meet the document's");
    assert!(!CanvasScrollbars::clamp(&mut view, large), "a view on the document stays");

    let small = DocRect::new(0.0, 0.0, 100.0, 100.0);
    let mut view = view_at(Point::new(0.0, 0.0), 1.0);
    assert!(!CanvasScrollbars::clamp(&mut view, small), "a document smaller than the view may sit anywhere inside it");
    let mut view = view_at(Point::new(-500.0, 0.0), 1.0);
    assert!(CanvasScrollbars::clamp(&mut view, small), "but not leave it");
}

#[trial]
fn a_rotated_view_is_clamped_by_the_document_as_it_is_shown() {
    let doc = DocRect::new(0.0, 0.0, 1000.0, 800.0);
    let mut view = view_at(Point::new(-50.0, 100.0), 1.0).rotation(90.0);
    assert!(CanvasScrollbars::clamp(&mut view, doc));
    let quad = view.quad(doc);
    let shown = quad.iter().skip(1).fold(Rect::from_min_max(quad[0], quad[0]), |rect, &corner| rect.union(Rect::from_min_max(corner, corner)));
    assert!(shown.expand(TOLERANCE).contains_rect(VIEW), "the turned document covers the view: {shown:?}");
}

/// Canvas with bars over a view at the start of a document of 800 by 600; the view is kept.
struct Shown {
    fixture: Fixture,
    view: Rc<RefCell<CanvasView>>,
}

fn shown(document: DocRect, overscroll: bool) -> Shown {
    let view = Rc::new(RefCell::new(view_at(Point::new(0.0, 0.0), ZOOM)));
    let edited = Rc::clone(&view);
    let mut fixture = Fixture::new(move |ui| {
        let view = *edited.borrow();
        Canvas::new("Document").view(view).show(ui, |ui, body| {
            let _ =
                CanvasScrollbars::new("Scroll bars", Bind::bound(&mut *edited.borrow_mut()), document).overscroll(overscroll).show_at(ui, body.rect);
        });
    });
    fixture.settle();
    Shown { fixture, view }
}

/// Presses at `from`, moves by `by` in steps and releases.
fn drag_from(fixture: &mut Fixture, from: Pos2, by: Vec2) {
    fixture.frame(vec![pointer::moved(from)]);
    fixture.frame(vec![pointer::press(from)]);
    for step in 1..=4 {
        fixture.frame(vec![pointer::moved(from + by * (step as f32 / 4.0))]);
    }
    fixture.frame(vec![pointer::release(from + by)]);
    fixture.frame(Vec::new());
}

#[trial]
fn dragging_the_thumb_moves_the_view_centre_along_the_bar() {
    let mut shown = shown(DocRect::new(0.0, 0.0, 800.0, 600.0), false);
    let bar = shown.fixture.by_label("Horizontal scroll bar").expect("the bar has a node");
    let before = shown.view.borrow().center;
    drag_from(&mut shown.fixture, bar.rect.left_center() + Vec2::new(GRIP, 0.0), Vec2::new(DRAG, 0.0));
    let after = shown.view.borrow().center;
    assert!(after.x > before.x && after.y == before.y, "{before:?} -> {after:?}");
}

#[trial]
fn pressing_the_track_pages_by_one_screenful_toward_the_press() {
    let mut shown = shown(DocRect::new(0.0, 0.0, 800.0, 600.0), false);
    let bar = shown.fixture.by_label("Horizontal scroll bar").expect("the bar has a node");
    shown.fixture.click_at(bar.rect.right_center() - Vec2::new(GRIP, 0.0));
    let page = f64::from(VIEW.width()) / ZOOM;
    let moved = shown.view.borrow().center.x;
    assert!((moved - page).abs() < 1e-6, "paged {moved}, a screenful is {page}");
}

#[trial]
fn only_overscroll_lets_a_document_smaller_than_the_window_scroll() {
    let small = DocRect::new(0.0, 0.0, 100.0, 100.0);
    for (overscroll, scrolls) in [(false, false), (true, true)] {
        let mut shown = shown(small, overscroll);
        let bar = shown.fixture.by_label("Horizontal scroll bar").expect("the bar has a node");
        let before = shown.view.borrow().center;
        drag_from(&mut shown.fixture, bar.rect.left_center() + Vec2::new(GRIP, 0.0), Vec2::new(DRAG, 0.0));
        assert_eq!(shown.view.borrow().center != before, scrolls, "overscroll {overscroll}");
    }
}
