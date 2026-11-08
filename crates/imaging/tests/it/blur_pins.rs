//! The Blur Gallery's pins: the model's handles and drags, the overlay's pointer and keys, and the panel.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use craft_behavior::kurbo::Point;
use craft_behavior::{CanvasView, Cursor, CursorIcon, EditEvent};
use craft_imaging::ui::blur_pins::{
    AXIS_HANDLES, BlurGrab, BlurKind, BlurPin, BlurPins, BlurPinsCommand, BlurPinsOverlay, BlurPinsPanel, DEFAULT_AMOUNT, IrisShape, MAX_PINS,
    PinReach, TiltBand, drag, feather_handles, grab, iris_point, round_handle, tilt_point, turn_handle,
};
use craft_ui::canvas::Canvas;
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::testing::{Fixture, pointer, trial};
use egui::{Key, Modifiers, Pos2, Rect, pos2};

const BOUNDS: [f64; 4] = [0.0, 0.0, 1000.0, 800.0];

fn reach() -> PinReach {
    PinReach::new(9.0, 24.0, 5.0, 5.0)
}

/// One screen point per document pixel.
fn view() -> CanvasView {
    CanvasView::new(Rect::from_min_max(pos2(0.0, 0.0), pos2(1000.0, 800.0)), Point::new(500.0, 400.0), 1.0)
}

fn start(kind: BlurKind) -> BlurPins {
    BlurPins::new(kind, BOUNDS, BlurPin::new(0.0, 0.0, DEFAULT_AMOUNT))
}

fn screen(point: Point) -> Pos2 {
    view().to_screen(point)
}

fn dragged(pins: &mut BlurPins, grab: BlurGrab, to: Point) {
    let start = pins.clone();
    let from = pins.to_document(&start.pins[0]);
    drag(pins, grab, &start, from, to);
}

// A new set has one pin in the middle, selected; adding takes the selected pin's amount and shape; moves stay inside
// the bounds; removing keeps the last pin; 64 pins at most.
#[trial]
fn blur_pins_add_move_tune_remove_and_stay_in_bounds() {
    let mut pins = start(BlurKind::Field);
    assert_eq!((pins.pins.len(), pins.selected), (1, Some(0)));
    assert_eq!((pins.pins[0].x, pins.pins[0].y), (0.5, 0.5));
    assert!(!pins.add(Point::new(2000.0, 10.0)), "outside the bounds");
    assert!(pins.add(Point::new(100.0, 80.0)));
    assert_eq!((pins.pins.len(), pins.selected), (2, Some(1)));
    assert_eq!((pins.pins[1].x, pins.pins[1].y), (0.1, 0.1));

    let before = pins.clone();
    drag(&mut pins, BlurGrab::pin(1), &before, Point::new(100.0, 80.0), Point::new(-500.0, 5000.0));
    assert_eq!((pins.pins[1].x, pins.pins[1].y), (0.0, 1.0));

    assert!(pins.remove_selected());
    assert_eq!((pins.pins.len(), pins.selected), (1, Some(0)));
    assert!(!pins.remove_selected(), "the last pin stays");

    while pins.pins.len() < MAX_PINS {
        assert!(pins.add(Point::new(10.0, 10.0)));
    }
    assert!(!pins.add(Point::new(10.0, 10.0)));
    pins.pins[0].amount = f64::NAN;
    pins.pins[1].x = 7.0;
    pins.selected = Some(99);
    pins.clean();
    assert_eq!((pins.pins[0].amount, pins.pins[1].x, pins.selected), (DEFAULT_AMOUNT, 1.0, None));
}

// Iris handles: an axis handle sets its radius, the square the roundness, a feather dot the feather; `clean` puts each
// back in range.
#[trial]
fn blur_pins_iris_handles_resize_turn_round_and_feather() {
    let mut pins = start(BlurKind::Iris);
    let pin = pins.pins[0];
    let Some(shape) = pin.iris else { panic!("an Iris pin has a shape") };
    assert_eq!(shape, IrisShape::iris());
    let centre = pins.to_document(&pin);

    // The press grabs each handle of the selected pin.
    let at = |u: f64, v: f64| screen(iris_point(&pins, &pin, shape, u, v));
    assert_eq!(grab(&pins, &view(), at(AXIS_HANDLES[0].0, AXIS_HANDLES[0].1), &reach()), BlurGrab::axis(0, 0));
    assert_eq!(grab(&pins, &view(), at(round_handle(shape).0, round_handle(shape).1), &reach()), BlurGrab::round(0));
    let dot = feather_handles(shape)[2];
    assert_eq!(grab(&pins, &view(), at(dot.0, dot.1), &reach()), BlurGrab::feather(0, 2));

    // Dragging +u out to 0.5 of the short side (400 px) turns nothing and sets radius_x.
    dragged(&mut pins, BlurGrab::axis(0, 0), Point::new(centre.x + 400.0, centre.y));
    let Some(resized) = pins.pins[0].iris else { panic!("shape") };
    assert!((resized.radius_x - 0.5).abs() < 1e-9 && resized.angle.abs() < 1e-9);
    // Dragging +v straight right turns the shape by a quarter turn (its +v axis points right).
    dragged(&mut pins, BlurGrab::axis(0, 1), Point::new(centre.x + 200.0, centre.y));
    let Some(turned) = pins.pins[0].iris else { panic!("shape") };
    assert!((turned.radius_y - 0.25).abs() < 1e-9 && (turned.angle + 90.0).abs() < 1e-9);

    // The square far out is a rectangle (roundness 100), at the centre an ellipse.
    let mut pins = start(BlurKind::Iris);
    dragged(&mut pins, BlurGrab::round(0), Point::new(centre.x + 5000.0, centre.y - 5000.0));
    assert!((pins.pins[0].iris.map_or(0.0, |s| s.roundness) - 100.0).abs() < 1e-9);
    dragged(&mut pins, BlurGrab::round(0), centre);
    assert!(pins.pins[0].iris.map_or(1.0, |s| s.roundness).abs() < 1e-9);

    // A feather dot at the shape's edge sets 1, clamped to 0.99; at the centre 0.
    dragged(&mut pins, BlurGrab::feather(0, 0), Point::new(centre.x + 5000.0, centre.y));
    assert_eq!(pins.pins[0].iris.map(|s| s.feather), Some(0.99));
    dragged(&mut pins, BlurGrab::feather(0, 0), centre);
    assert_eq!(pins.pins[0].iris.map(|s| s.feather), Some(0.0));

    let wild = IrisShape::new(9.0, f64::NAN, 270.0, -4.0, 3.0).clean();
    assert_eq!(wild, IrisShape::new(1.0, 0.25, -90.0, 0.0, 0.99));
}

// Tilt-Shift: the solid lines set the focus, the dashed the transition, the dots the angle (folded into −90..=90); a
// click adds no pin.
#[trial]
fn blur_pins_tilt_shift_lines_and_dots() {
    let mut pins = start(BlurKind::TiltShift);
    let pin = pins.pins[0];
    let Some(band) = pin.tilt else { panic!("a Tilt-Shift pin has a band") };
    let centre = pins.to_document(&pin);
    assert!(!pins.add(Point::new(10.0, 10.0)));
    assert_eq!(grab(&pins, &view(), screen(Point::new(10.0, 10.0)), &reach()), BlurGrab::New);

    // The band is horizontal: its normal is vertical, so the lines sit above and below.
    let side = pins.short_side();
    let focus_line = screen(tilt_point(&pins, &pin, band, band.focus * side, 150.0));
    assert_eq!(grab(&pins, &view(), focus_line, &reach()), BlurGrab::focus(0));
    let edge_line = screen(tilt_point(&pins, &pin, band, -(band.focus + band.transition) * side, 150.0));
    assert_eq!(grab(&pins, &view(), edge_line, &reach()), BlurGrab::transition(0));
    let dot = screen(turn_handle(&pins, &pin, band, &view(), &reach(), 1.0));
    assert_eq!(grab(&pins, &view(), dot, &reach()), BlurGrab::turn(0, 1));

    dragged(&mut pins, BlurGrab::focus(0), Point::new(centre.x + 33.0, centre.y - 160.0));
    assert!((pins.pins[0].tilt.map_or(0.0, |b| b.focus) - 0.2).abs() < 1e-9);
    dragged(&mut pins, BlurGrab::transition(0), Point::new(centre.x, centre.y + 240.0));
    assert!((pins.pins[0].tilt.map_or(0.0, |b| b.transition) - 0.1).abs() < 1e-9);
    // The +1 dot dragged straight right turns the normal (sin a, cos a) to a = 90°.
    dragged(&mut pins, BlurGrab::turn(0, 1), Point::new(centre.x + 300.0, centre.y));
    assert!((pins.pins[0].tilt.map_or(0.0, |b| b.angle) - 90.0).abs() < 1e-9);
    // The -1 dot dragged the same way is the other half turn: folded to −90.
    dragged(&mut pins, BlurGrab::turn(0, -1), Point::new(centre.x + 300.0, centre.y));
    assert!((pins.pins[0].tilt.map_or(0.0, |b| b.angle) + 90.0).abs() < 1e-9);
    assert_eq!(TiltBand::new(135.0, f64::NAN, 0.0).clean(), TiltBand::new(-45.0, 0.1, 0.01));
}

// A whole turn of the ring sweeps the full range (Spin: 360°), clamped at the ends.
#[trial]
fn blur_pins_spin_ring_sets_the_angle() {
    let mut pins = start(BlurKind::Spin);
    let centre = pins.to_document(&pins.pins[0]);
    let ring = screen(Point::new(centre.x + f64::from(reach().ring), centre.y));
    assert_eq!(grab(&pins, &view(), ring, &reach()), BlurGrab::ring(0));
    assert_eq!(BlurKind::Spin.max(), 360.0);
    pins.pins[0].amount = 0.0;

    // A quarter turn clockwise on screen (from the right to below the centre) adds 90°; four quarter turns, one frame
    // each, come to the full range.
    let round = [(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0), (1.0, 0.0)].map(|(x, y)| Point::new(centre.x + 100.0 * x, centre.y + 100.0 * y));
    for (index, pair) in round.windows(2).enumerate() {
        let frame = pins.clone();
        drag(&mut pins, BlurGrab::ring(0), &frame, pair[0], pair[1]);
        assert!((pins.pins[0].amount - 90.0 * (index + 1) as f64).abs() < 1e-9);
    }
    // Further round stays at the top; back down comes off it.
    let frame = pins.clone();
    drag(&mut pins, BlurGrab::ring(0), &frame, round[0], round[1]);
    assert_eq!(pins.pins[0].amount, 360.0);
    let frame = pins.clone();
    drag(&mut pins, BlurGrab::ring(0), &frame, round[1], round[0]);
    assert!((pins.pins[0].amount - 270.0).abs() < 1e-9);

    let mut field = start(BlurKind::Field);
    field.pins[0].amount = 499.0;
    let frame = field.clone();
    drag(&mut field, BlurGrab::ring(0), &frame, round[0], round[1]);
    assert_eq!(field.pins[0].amount, 500.0);
}

// A new pin copies the selected pin's amount and shape.
#[trial]
fn blur_pins_new_pin_takes_the_selected_pins_amount_and_shape() {
    let mut pins = start(BlurKind::Iris);
    let shape = IrisShape::new(0.2, 0.1, 30.0, 40.0, 0.3);
    pins.pins[0].amount = 77.0;
    pins.pins[0].iris = Some(shape);
    assert!(pins.add(Point::new(300.0, 300.0)));
    assert_eq!((pins.pins[1].amount, pins.pins[1].iris), (77.0, Some(shape)));
    // After selecting the first pin again, the next copies it, not the newest.
    pins.pins[1].amount = 5.0;
    pins.selected = Some(0);
    assert!(pins.add(Point::new(600.0, 300.0)));
    assert_eq!(pins.pins[2].amount, 77.0);

    let mut spin = start(BlurKind::Spin);
    assert_eq!(spin.pins[0].iris, Some(IrisShape::spin()));
    assert!(spin.add(Point::new(300.0, 300.0)));
    assert_eq!(spin.pins[1].iris, Some(IrisShape::spin()));
}

// The cursor over each grab: Alias, Move, ResizeVertical, Grab, Crosshair (Default for Tilt-Shift).
#[trial]
fn blur_pins_cursors_follow_the_grab() {
    let cases = [
        (BlurGrab::ring(0), CursorIcon::Alias),
        (BlurGrab::round(0), CursorIcon::Alias),
        (BlurGrab::turn(0, 1), CursorIcon::Alias),
        (BlurGrab::axis(0, 1), CursorIcon::Move),
        (BlurGrab::feather(0, 0), CursorIcon::Move),
        (BlurGrab::focus(0), CursorIcon::ResizeVertical),
        (BlurGrab::transition(0), CursorIcon::ResizeVertical),
        (BlurGrab::pin(0), CursorIcon::Grab),
        (BlurGrab::New, CursorIcon::Crosshair),
    ];
    for (grab, icon) in cases {
        assert_eq!(grab.cursor(false), icon, "{grab:?}");
    }
    assert_eq!(BlurGrab::New.cursor(true), CursorIcon::Default);

    // A press on the pin's dot grabs the pin; far from everything grabs nothing.
    let pins = start(BlurKind::Field);
    assert_eq!(grab(&pins, &view(), screen(Point::new(503.0, 400.0)), &reach()), BlurGrab::pin(0));
    assert_eq!(grab(&pins, &view(), screen(Point::new(100.0, 100.0)), &reach()), BlurGrab::New);
}

/// How much farther Shift moves a pin with an arrow key.
const NUDGE_RATIO: f64 = 10.0;

const CANVAS: [f64; 4] = [0.0, 0.0, 400.0, 300.0];

fn pins_of(kind: BlurKind, amounts: &[f64]) -> BlurPins {
    let mut pins = BlurPins::new(kind, CANVAS, BlurPin::new(0.5, 0.5, 15.0));
    pins.pins = amounts.iter().enumerate().map(|(i, &amount)| BlurPin::new(0.25 + 0.5 * i as f64 / amounts.len() as f64, 0.5, amount)).collect();
    pins.selected = Some(0);
    pins.clean();
    pins
}

struct Overlay {
    fixture: Fixture,
    pins: Rc<RefCell<BlurPins>>,
    seen: Rc<RefCell<Vec<&'static str>>>,
    view: Rc<Cell<Option<CanvasView>>>,
}

fn overlay(pins: BlurPins) -> Overlay {
    let pins = Rc::new(RefCell::new(pins));
    let seen = Rc::new(RefCell::new(Vec::new()));
    let view = Rc::new(Cell::new(None));
    let (shown_pins, shown_seen, shown_view) = (Rc::clone(&pins), Rc::clone(&seen), Rc::clone(&view));
    let mut fixture = Fixture::new(move |ui| {
        let room = ui.available_rect_before_wrap();
        let canvas_view = CanvasView::new(room, Point::new(200.0, 150.0), 1.0);
        shown_view.set(Some(canvas_view));
        let shown = Canvas::new("Document").view(canvas_view).show(ui, |ui, body| {
            let mut held = shown_pins.borrow_mut();
            BlurPinsOverlay::new("Blur pins", Bind::bound(&mut *held)).show_at(ui, body.rect)
        });
        if let Some(output) = shown.inner
            && let Some(event) = &output.edit.event
        {
            shown_seen.borrow_mut().push(match event {
                EditEvent::Begin { .. } => "begin",
                EditEvent::Update { .. } => "update",
                EditEvent::Commit { .. } => "commit",
                EditEvent::Cancel { .. } => "cancel",
                _ => "set",
            });
        }
    });
    fixture.settle();
    Overlay { fixture, pins, seen, view }
}

impl Overlay {
    fn at(&self, x: f64, y: f64) -> Pos2 {
        self.view.get().expect("shown").to_screen(Point::new(x, y))
    }

    fn click(&mut self, x: f64, y: f64) {
        let at = self.at(x, y);
        self.fixture.click_at(at);
    }

    fn press(&mut self, at: Pos2) {
        self.fixture.frame(vec![pointer::moved(at)]);
        self.fixture.frame(vec![pointer::press(at)]);
    }

    fn move_to(&mut self, at: Pos2) {
        self.fixture.frame(vec![pointer::moved(at)]);
    }

    fn release(&mut self, at: Pos2) {
        self.fixture.frame(vec![pointer::release(at)]);
        self.fixture.frame(Vec::new());
    }
}

// A click adds a pin (Set), a drag reports Begin, Update, Commit, Delete removes the focused, selected pin.
#[trial]
fn blur_pins_overlay_reports_edit_events_and_delete_removes() {
    let mut o = overlay(pins_of(BlurKind::Field, &[15.0]));

    o.click(50.0, 50.0);
    assert_eq!(*o.seen.borrow(), ["set"], "a click on nothing adds a pin");
    assert_eq!(o.pins.borrow().pins.len(), 2);
    assert_eq!(o.pins.borrow().selected, Some(1));

    o.seen.borrow_mut().clear();
    let (from, to) = (o.at(50.0, 50.0), o.at(120.0, 80.0));
    o.press(from);
    o.move_to(to);
    o.release(to);
    let events = o.seen.borrow().clone();
    assert_eq!(events.first(), Some(&"begin"), "{events:?}");
    assert_eq!(events.last(), Some(&"commit"), "{events:?}");
    let pin = o.pins.borrow().pins[1];
    assert!((pin.x - 120.0 / 400.0).abs() < 1e-6 && (pin.y - 80.0 / 300.0).abs() < 1e-6, "{pin:?}");

    o.seen.borrow_mut().clear();
    o.click(120.0, 80.0);
    o.seen.borrow_mut().clear();
    o.fixture.key(Key::Delete, Modifiers::NONE);
    assert_eq!(*o.seen.borrow(), ["set"]);
    assert_eq!(o.pins.borrow().pins.len(), 1);
    assert_eq!(o.pins.borrow().selected, Some(0), "the pin before the removed one is selected");
}

// Page picks a pin as a keyboard part, arrows move it, and the part is named "Pin 2, Blur 15 px".
#[trial]
fn blur_pins_are_keyboard_parts() {
    let mut o = overlay(pins_of(BlurKind::Field, &[30.0, 15.0]));
    assert!(o.fixture.by_label("Pin 2, Blur 15 px").is_some());
    assert!(o.fixture.by_label("Pin 1, Blur 30 px").is_some());

    let first = o.pins.borrow().to_document(&o.pins.borrow().pins[0]);
    o.click(first.x, first.y);
    o.fixture.key(Key::PageDown, Modifiers::NONE);
    assert_eq!(o.pins.borrow().selected, Some(1), "Page Down picks and selects the next pin");

    let x = o.pins.borrow().pins[1].x;
    o.fixture.key(Key::ArrowRight, Modifiers::NONE);
    let one = o.pins.borrow().pins[1].x - x;
    assert!(one > 0.0, "an arrow moves the pin right");
    o.fixture.key(Key::ArrowRight, Modifiers::SHIFT);
    let ten = o.pins.borrow().pins[1].x - x - one;
    assert!((ten / one - NUDGE_RATIO).abs() < 1e-6, "Shift moves ten times as far: {one} then {ten}");
}

// A Spin Blur pin's part is named with its angle and the degree unit.
#[trial]
fn spin_pins_are_named_with_their_angle() {
    let o = overlay(pins_of(BlurKind::Spin, &[90.0]));
    assert!(o.fixture.by_label("Pin 1, Blur Angle 90°").is_some());
}

// Add Pin puts a pin at the middle of the bounds, selected: the keyboard path to creating a pin.
#[trial]
fn add_pin_command_adds_a_pin_at_the_centre() {
    let mut pins = pins_of(BlurKind::Field, &[15.0]);
    assert!(BlurPinsCommand::AddPin.apply(&mut pins));
    assert_eq!(pins.pins.len(), 2);
    let added = pins.pins[1];
    assert_eq!((added.x, added.y), (0.5, 0.5));
    assert_eq!(pins.selected, Some(1));
    let mut tilt = pins_of(BlurKind::TiltShift, &[15.0]);
    assert!(!BlurPinsCommand::AddPin.apply(&mut tilt), "Tilt-Shift has one band");
}

struct Panel {
    fixture: Fixture,
}

fn panel(pins: BlurPins) -> Panel {
    let pins = Rc::new(RefCell::new(pins));
    let mut fixture = Fixture::new(move |ui| {
        BlurPinsPanel::new(&mut pins.borrow_mut()).show(ui);
    });
    fixture.settle();
    Panel { fixture }
}

impl Panel {
    fn has(&self, text: &str) -> bool {
        let tree = self.fixture.tree();
        tree.keys().into_iter().any(|key| tree.name_of(key).is_some_and(|name| name.contains(text)))
    }
}

// The panel shows Blur (Spin: Blur Angle) for the selected pin, "No pin selected" without one, and the kind's hints.
#[trial]
fn blur_pins_panel_shows_the_selected_pins_amount_and_hints() {
    let p = panel(pins_of(BlurKind::Field, &[15.0]));
    assert!(p.has("Blur"));
    assert!(!p.has("No pin selected"));
    assert!(p.has("Click the image to add a pin"));
    assert!(!p.has("Drag the handles on the outline"));

    let p = panel(pins_of(BlurKind::Spin, &[90.0]));
    assert!(p.fixture.by_label("Blur Angle").is_some() || p.has("Blur Angle"));
    assert!(p.has("the ring sets the blur angle"));

    let p = panel(pins_of(BlurKind::TiltShift, &[15.0]));
    assert!(p.has("Drag the solid lines"));
    assert!(!p.has("Click the image"));

    let mut none = pins_of(BlurKind::Iris, &[15.0]);
    none.selected = None;
    let p = panel(none);
    assert!(p.has("No pin selected"));
    assert!(p.fixture.by_label("Blur").is_none());
    assert!(p.has("square to round its corners"));
}

// A drag on the selected pin's ring turns the amount frame by frame: a quarter turn clockwise on screen adds a quarter
// of the range, and the turns add up past a half turn.
#[trial]
fn blur_pins_ring_drag_turns_the_amount_over_several_frames() {
    let mut o = overlay(pins_of(BlurKind::Field, &[15.0]));
    let centre = o.at(100.0, 150.0);
    assert!((o.pins.borrow().pins[0].x - 0.25).abs() < 1e-9);
    // The ring is where the pointer shows the ring's cursor.
    let mut radius = None;
    for step in 1..=200u16 {
        let distance = f32::from(step) / 2.0;
        o.move_to(centre + egui::Vec2::new(distance, 0.0));
        if craft_ui::runtime::cursor::current(o.fixture.ctx()) == Some(Cursor::System(CursorIcon::Alias)) {
            radius = Some(distance);
            break;
        }
    }
    let radius = radius.expect("the pointer meets the ring");
    let on_ring = |quarter: u8| centre + egui::Vec2::angled(std::f32::consts::FRAC_PI_2 * f32::from(quarter)) * radius;
    o.press(on_ring(0));
    for quarter in 1..=3 {
        o.move_to(on_ring(quarter));
    }
    o.release(on_ring(3));
    // Three quarter turns: 15 + 0.75 * 500.
    let amount = o.pins.borrow().pins[0].amount;
    assert!((amount - 390.0).abs() < 1.0, "{amount}");
}
