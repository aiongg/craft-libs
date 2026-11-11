//! `CanvasScrollbars`: the clamp of the view and the bars' drag and page press.

use craft_behavior::CanvasView;
use craft_behavior::kurbo::{Point, Rect as DocRect};
use craft_ui::canvas::{Canvas, CanvasScrollbars};
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::testing::{Fixture, trial};
use egui::{Pos2, Rect};
use std::cell::RefCell;
use std::rc::Rc;

/// The view's rect in the fixture, in points.
const VIEW: Rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(400.0, 300.0));

const ZOOM: f64 = 2.0;

/// The slack of a rect comparison after f32 maths, in points.
const TOLERANCE: f32 = 0.01;

fn view_at(center: Point, zoom: f64) -> CanvasView {
    CanvasView::new(VIEW, center, zoom)
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
