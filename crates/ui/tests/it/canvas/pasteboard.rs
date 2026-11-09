//! `Pasteboard`: the pasteboard quads and the page under a point.

use craft_behavior::CanvasView;
use craft_behavior::kurbo::{Point, Rect as DocRect};
use craft_ui::canvas::{Canvas, PageFrame, Pasteboard};
use craft_ui::contract::{Role, Show as _, ShowContainer as _};
use craft_ui::testing::{Fixture, pointer, trial};
use egui::{Pos2, Rect, Vec2};
use std::cell::Cell;
use std::rc::Rc;

const VIEW_SIZE: Vec2 = Vec2::new(200.0, 100.0);
const TURN: f64 = 30.0;
const AREA_TOLERANCE: f64 = 1.0;

fn view() -> CanvasView {
    CanvasView::new(Rect::from_min_size(Pos2::ZERO, VIEW_SIZE), Point::new(100.0, 50.0), 1.0)
}

fn page() -> PageFrame {
    PageFrame::new(DocRect::new(80.0, 40.0, 120.0, 60.0))
}

/// The area of a quad given in order round it.
fn area(quad: &[Pos2; 4]) -> f64 {
    let twice: f32 = (0..4).map(|i| quad[i].x * quad[(i + 1) % 4].y - quad[(i + 1) % 4].x * quad[i].y).sum();
    f64::from(twice.abs()) / 2.0
}

fn covered(view: &CanvasView, pages: &[PageFrame]) -> f64 {
    Pasteboard::quads(view, pages).iter().map(area).sum()
}

#[trial]
fn the_pasteboard_is_the_view_minus_the_pages_whether_the_view_is_turned_or_not() {
    let pages = [page()];
    let page_area = 40.0 * 20.0;
    for view in [view(), view().rotation(TURN)] {
        let expected = view.visible().area() - page_area;
        assert!((covered(&view, &pages) - expected).abs() < AREA_TOLERANCE, "the quads cover the visible document but the page");
    }
}

/// The document point the pasteboard reports for a secondary click `offset` points from the canvas's top left, where
/// the document origin lies.
fn context_after_secondary_click(offset: Vec2) -> Option<Point> {
    let seen: Rc<Cell<Option<Point>>> = Rc::default();
    let shown = Rc::clone(&seen);
    let pages = [page()];
    let mut fixture = Fixture::new(move |ui| {
        let room = ui.available_rect_before_wrap();
        let view = CanvasView::new(room, Point::new(f64::from(room.width()) / 2.0, f64::from(room.height()) / 2.0), 1.0);
        Canvas::new("Document").view(view).show(ui, |ui, body| {
            if let Some(at) = Pasteboard::new("Pasteboard", &pages).show_at(ui, body.rect).context_at {
                shown.set(Some(at));
            }
        });
    });
    fixture.settle();
    let canvas = fixture.by_role(Role::CANVAS).into_iter().next().expect("the canvas has a node");
    let at = canvas.rect.min + offset;
    fixture.frame(vec![pointer::moved(at)]);
    fixture.frame(vec![pointer::secondary(at, true)]);
    fixture.frame(vec![pointer::secondary(at, false)]);
    fixture.idle(1);
    seen.get()
}

#[trial]
fn a_secondary_click_off_the_pages_reports_the_document_point_and_one_on_a_page_does_not() {
    let off = context_after_secondary_click(Vec2::new(10.0, 10.0)).expect("the click on the pasteboard is reported");
    assert!((off.x - 10.0).abs() < 1e-3 && (off.y - 10.0).abs() < 1e-3, "{off:?}");
    assert_eq!(context_after_secondary_click(Vec2::new(100.0, 50.0)), None, "a click on a page is the document's");
}
