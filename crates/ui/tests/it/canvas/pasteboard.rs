//! `Pasteboard`: the pasteboard quads and the page under a point.

use craft_behavior::CanvasView;
use craft_behavior::kurbo::{Point, Rect as DocRect};
use craft_ui::canvas::{PageFrame, Pasteboard};
use craft_ui::testing::trial;
use egui::{Pos2, Rect, Vec2};

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

#[trial]
fn a_page_outside_the_view_takes_nothing_from_the_pasteboard() {
    let far = PageFrame::new(DocRect::new(1000.0, 1000.0, 1100.0, 1100.0));
    let view = view();
    assert!((covered(&view, &[far]) - view.visible().area()).abs() < AREA_TOLERANCE);
}

#[trial]
fn the_last_of_overlapping_pages_is_the_one_under_a_point_and_a_flipped_page_counts() {
    let pages = [page(), PageFrame::new(DocRect::new(100.0, 60.0, 90.0, 45.0))];
    assert_eq!(Pasteboard::page_at(&pages, Point::new(95.0, 50.0)), Some(1));
    assert_eq!(Pasteboard::page_at(&pages, Point::new(85.0, 42.0)), Some(0));
    assert_eq!(Pasteboard::page_at(&pages, Point::new(10.0, 10.0)), None);
}
