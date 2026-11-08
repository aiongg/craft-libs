//! `Canvas` with a view: the space its body reads. The view's own input is in `document_canvas`.

use craft_behavior::CanvasView;
use craft_behavior::kurbo::Point;
use craft_ui::canvas::{Canvas, CanvasSpace, canvas_space};
use craft_ui::contract::ShowContainer as _;
use craft_ui::sections::GroupBox;
use craft_ui::testing::{Fixture, trial};
use egui::{Pos2, Rect, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

fn turned_view() -> CanvasView {
    CanvasView::new(Rect::from_min_size(Pos2::new(10.0, 10.0), Vec2::new(200.0, 120.0)), Point::new(50.0, 25.0), 2.0).rotation(90.0)
}

#[trial]
fn a_member_in_a_canvas_body_reads_the_view_and_one_outside_reads_none() {
    let view = turned_view();
    let seen: Rc<RefCell<(Option<CanvasSpace>, Option<CanvasSpace>)>> = Rc::default();
    let shared = Rc::clone(&seen);
    let mut fixture = Fixture::new(move |ui| {
        GroupBox::new("Outside", None).show(ui, |_, body| shared.borrow_mut().0 = canvas_space(&body.env));
        Canvas::new("Document").view(view).show(ui, |_, body| shared.borrow_mut().1 = canvas_space(&body.env));
    });
    fixture.settle();

    let (outside, inside) = seen.borrow().clone();
    assert_eq!(outside, None, "a member outside every canvas reads no view");
    let space = inside.expect("a member in the canvas body reads the view");
    let shown = space.view;
    assert_eq!((shown.center, shown.zoom, shown.rotation), (view.center, view.zoom, view.rotation), "the view the app gave, on the canvas's rect");
    assert_eq!(space.zoom_device(), view.zoom * f64::from(space.pixels_per_point), "the device zoom follows the interface scale");
}
