//! `GradientOverlay` and the gradient line geometry, through the public API.

use std::cell::RefCell;
use std::rc::Rc;

use craft_behavior::CanvasView;
use craft_behavior::kurbo::Point;
use craft_draw::ui::gradient::gradient_line::{GradientLine, LineGradient, at, project, snap45};
use craft_draw::ui::gradient::{ColorStop, GradientOverlay, GradientStops, StopRef};
use craft_theme::Srgba;
use craft_ui::canvas::Canvas;
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::testing::{Fixture, trial};
use egui::Vec2;

fn black() -> Srgba {
    Srgba { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }
}

fn gradient() -> LineGradient {
    LineGradient::new(
        GradientLine::new(Point::new(20.0, 100.0), Point::new(220.0, 100.0)),
        GradientStops::new(vec![ColorStop::new(0.0, black()), ColorStop::new(0.5, black()), ColorStop::new(1.0, black())]),
    )
}

struct Shown {
    fixture: Fixture,
    value: Rc<RefCell<LineGradient>>,
    selected: Rc<RefCell<Option<StopRef>>>,
}

fn shown() -> Shown {
    let value = Rc::new(RefCell::new(gradient()));
    let selected = Rc::new(RefCell::new(None));
    let (held, chosen) = (Rc::clone(&value), Rc::clone(&selected));
    let mut fixture = Fixture::new(move |ui| {
        let room = ui.available_rect_before_wrap();
        let view = CanvasView::new(room, Point::new(f64::from(room.width()) / 2.0, f64::from(room.height()) / 2.0), 1.0);
        Canvas::new("Document").view(view).show(ui, |ui, body| {
            let mut value = held.borrow_mut();
            let mut selection = chosen.borrow_mut();
            GradientOverlay::new("Gradient", Bind::bound(&mut *value)).shared_selection(Bind::bound(&mut *selection)).show_at(ui, body.rect)
        });
    });
    fixture.settle();
    Shown { fixture, value, selected }
}

#[trial]
fn locations_follow_reverse() {
    let line = GradientLine::new(Point::new(10.0, 10.0), Point::new(110.0, 10.0));
    assert_eq!(at(&line, 0.25, false), Point::new(35.0, 10.0));
    assert_eq!(at(&line, 0.25, true), Point::new(85.0, 10.0));
    let (t, d) = project(&line, Point::new(85.0, 40.0), true);
    assert!((t - 0.25).abs() < 1e-6 && (d - 30.0).abs() < 1e-9);
    assert_eq!(project(&line, Point::new(210.0, 10.0), false), (1.0, 100.0));
}

#[trial]
fn snap_steps_start_at_the_base_angle() {
    let fixed = Point::new(0.0, 0.0);
    let snapped = snap45(fixed, Point::new(10.0, 1.0), 0.0);
    assert!((snapped.x - 10.0_f64.hypot(1.0)).abs() < 1e-9 && snapped.y.abs() < 1e-9);
    let tilted = snap45(fixed, Point::new(10.0, 0.5), 10.0);
    assert!((tilted.y.atan2(tilted.x).to_degrees() - 10.0).abs() < 1e-9);
}

#[trial]
fn a_press_on_a_stop_selects_it_and_its_drag_moves_it() {
    let mut s = shown();
    let stop = s.fixture.by_label("Color stop 2 of 3").expect("the overlay shows its stops as parts");
    s.fixture.click(&stop);
    assert_eq!(*s.selected.borrow(), Some(StopRef::Color(1)));
    let before = s.value.borrow().stops.color[1].location;
    s.fixture.drag(&stop, Vec2::new(40.0, 0.0));
    let moved = s.value.borrow().stops.color[1].location;
    assert!(moved > before, "the stop followed the pointer to {moved}");
}
