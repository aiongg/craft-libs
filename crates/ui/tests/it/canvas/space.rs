//! `Canvas` with a view: the space its body reads, the wheel and the middle-button pan.

use craft_behavior::kurbo::Point;
use craft_behavior::{CanvasView, Wheel};
use craft_ui::canvas::{Canvas, CanvasSpace, ToolPath, canvas_space};
use craft_ui::contract::{Role, ShowContainer as _};
use craft_ui::sections::GroupBox;
use craft_ui::testing::pointer;
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
        Canvas::new("Document", ToolPath::Freehand).view(view).show(ui, |_, body| shared.borrow_mut().1 = canvas_space(&body.env));
    });
    fixture.settle();

    let (outside, inside) = seen.borrow().clone();
    assert_eq!(outside, None, "a member outside every canvas reads no view");
    let space = inside.expect("a member in the canvas body reads the view");
    assert_eq!(space.view, view);
    assert_eq!(space.zoom_device, view.zoom * f64::from(space.pixels_per_point), "the device zoom follows the interface scale");
}

/// A canvas with a view that records the wheel, the pan and the tool press of every pass.
struct Watched {
    fixture: Fixture,
    wheels: Rc<RefCell<Vec<Wheel>>>,
    pans: Rc<RefCell<Vec<Vec2>>>,
    presses: Rc<RefCell<usize>>,
}

fn watched() -> Watched {
    let (wheels, pans, presses) = (Rc::new(RefCell::new(Vec::new())), Rc::new(RefCell::new(Vec::new())), Rc::new(RefCell::new(0)));
    let (w, p, t) = (Rc::clone(&wheels), Rc::clone(&pans), Rc::clone(&presses));
    let mut fixture = Fixture::new(move |ui| {
        let canvas = Canvas::new("Document", ToolPath::Freehand).view(turned_view()).show(ui, |_, _| {});
        w.borrow_mut().extend(canvas.output.wheel);
        p.borrow_mut().extend(canvas.output.pan);
        if canvas.output.press.phase.is_press() {
            *t.borrow_mut() += 1;
        }
    });
    fixture.settle();
    Watched { fixture, wheels, pans, presses }
}

fn centre(fixture: &Fixture) -> Pos2 {
    fixture.by_role(Role::CANVAS).into_iter().next().expect("the canvas has a node").center()
}

#[trial]
fn a_wheel_over_the_canvas_reaches_its_output() {
    let mut watched = watched();
    let at = centre(&watched.fixture);
    watched.fixture.frame(vec![pointer::moved(at)]);
    watched.fixture.frame(vec![pointer::wheel(-30.0)]);
    let wheels = watched.wheels.borrow();
    assert!(wheels.iter().any(|wheel| wheel.scroll.y != 0.0), "the scroll shows in the output: {wheels:?}");
}

#[trial]
fn a_middle_drag_on_the_background_pans_and_gives_the_tool_no_press() {
    let mut watched = watched();
    let from = centre(&watched.fixture);
    let fixture = &mut watched.fixture;
    fixture.frame(vec![pointer::moved(from)]);
    fixture.frame(vec![pointer::middle(from, true)]);
    for step in 1..=4 {
        fixture.frame(vec![pointer::moved(from + Vec2::new(8.0 * step as f32, 0.0))]);
    }
    fixture.frame(vec![pointer::middle(from + Vec2::new(32.0, 0.0), false)]);
    fixture.idle(1);
    let moved: f32 = watched.pans.borrow().iter().map(|pan| pan.x).sum();
    assert!(moved > 0.0, "the drag reports its movement as a pan");
    assert_eq!(*watched.presses.borrow(), 0, "a middle press is never a tool press");
}
