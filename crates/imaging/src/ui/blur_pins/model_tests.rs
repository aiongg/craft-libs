//! Tests of the pins' model and grabs (task WS-BP1).

use egui::{Color32, CursorIcon, Rect, pos2};

use craft_ui::canvas::CanvasView;
use craft_ui::canvas::kurbo::Point;

use super::grab::{AXIS_HANDLES, feather_handles, iris_point, round_handle, tilt_point, turn_handle};
use super::{BlurGrab, BlurKind, BlurPin, BlurPins, BlurPinsStyle, DEFAULT_AMOUNT, IrisShape, MAX_PINS, TiltBand, drag, grab};

const BOUNDS: [f64; 4] = [0.0, 0.0, 1000.0, 800.0];

fn style() -> BlurPinsStyle {
    BlurPinsStyle {
        dot: 5.0,
        grab: 9.0,
        ring: 24.0,
        ring_band: 5.0,
        handle: 4.5,
        line_grab: 5.0,
        line: Color32::WHITE,
        halo: Color32::BLACK,
        amount: Color32::BLUE,
    }
}

/// One screen point per document pixel.
fn view() -> CanvasView {
    CanvasView::new(Rect::from_min_max(pos2(0.0, 0.0), pos2(1000.0, 800.0)), Point::new(500.0, 400.0), 1.0)
}

fn start(kind: BlurKind) -> BlurPins {
    let pin = BlurPin { x: 0.0, y: 0.0, amount: DEFAULT_AMOUNT, iris: None, tilt: None };
    BlurPins::new(kind, BOUNDS, pin)
}

fn screen(point: Point) -> egui::Pos2 {
    view().to_screen(point)
}

fn dragged(pins: &mut BlurPins, grab: BlurGrab, to: Point) {
    let start = pins.clone();
    let from = pins.to_document(&start.pins[0]);
    drag(pins, grab, &start, from, to);
}

// A new set has one pin in the middle, selected; adding takes the selected pin's amount and shape;
// moves stay inside the bounds; removing keeps the last pin; 64 pins at most.
#[test]
fn blur_pins_add_move_tune_remove_and_stay_in_bounds() {
    let mut pins = start(BlurKind::Field);
    assert_eq!((pins.pins.len(), pins.selected), (1, Some(0)));
    assert_eq!((pins.pins[0].x, pins.pins[0].y), (0.5, 0.5));
    assert!(!pins.add(Point::new(2000.0, 10.0)), "outside the bounds");
    assert!(pins.add(Point::new(100.0, 80.0)));
    assert_eq!((pins.pins.len(), pins.selected), (2, Some(1)));
    assert_eq!((pins.pins[1].x, pins.pins[1].y), (0.1, 0.1));

    let before = pins.clone();
    drag(&mut pins, BlurGrab::Pin { pin: 1 }, &before, Point::new(100.0, 80.0), Point::new(-500.0, 5000.0));
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

// Iris handles: an axis handle sets its radius, the square the roundness, a feather dot the
// feather; `clean` puts each back in range.
#[test]
fn blur_pins_iris_handles_resize_turn_round_and_feather() {
    let mut pins = start(BlurKind::Iris);
    let pin = pins.pins[0];
    let Some(shape) = pin.iris else { panic!("an Iris pin has a shape") };
    assert_eq!(shape, IrisShape::iris());
    let centre = pins.to_document(&pin);

    // The press grabs each handle of the selected pin.
    let at = |u: f64, v: f64| screen(iris_point(&pins, &pin, shape, u, v));
    assert_eq!(grab(&pins, &view(), at(AXIS_HANDLES[0].0, AXIS_HANDLES[0].1), &style()), BlurGrab::Axis { pin: 0, axis: 0 });
    assert_eq!(grab(&pins, &view(), at(round_handle(shape).0, round_handle(shape).1), &style()), BlurGrab::Round { pin: 0 });
    let dot = feather_handles(shape)[2];
    assert_eq!(grab(&pins, &view(), at(dot.0, dot.1), &style()), BlurGrab::Feather { pin: 0, dot: 2 });

    // Dragging +u out to 0.5 of the short side (400 px) turns nothing and sets radius_x.
    dragged(&mut pins, BlurGrab::Axis { pin: 0, axis: 0 }, Point::new(centre.x + 400.0, centre.y));
    let Some(resized) = pins.pins[0].iris else { panic!("shape") };
    assert!((resized.radius_x - 0.5).abs() < 1e-9 && resized.angle.abs() < 1e-9);
    // Dragging +v straight right turns the shape by a quarter turn (its +v axis points right).
    dragged(&mut pins, BlurGrab::Axis { pin: 0, axis: 1 }, Point::new(centre.x + 200.0, centre.y));
    let Some(turned) = pins.pins[0].iris else { panic!("shape") };
    assert!((turned.radius_y - 0.25).abs() < 1e-9 && (turned.angle + 90.0).abs() < 1e-9);

    // The square far out is a rectangle (roundness 100), at the centre an ellipse.
    let mut pins = start(BlurKind::Iris);
    dragged(&mut pins, BlurGrab::Round { pin: 0 }, Point::new(centre.x + 5000.0, centre.y - 5000.0));
    assert!((pins.pins[0].iris.map_or(0.0, |s| s.roundness) - 100.0).abs() < 1e-9);
    dragged(&mut pins, BlurGrab::Round { pin: 0 }, centre);
    assert!(pins.pins[0].iris.map_or(1.0, |s| s.roundness).abs() < 1e-9);

    // A feather dot at the shape's edge sets 1, clamped to 0.99; at the centre 0.
    dragged(&mut pins, BlurGrab::Feather { pin: 0, dot: 0 }, Point::new(centre.x + 5000.0, centre.y));
    assert_eq!(pins.pins[0].iris.map(|s| s.feather), Some(0.99));
    dragged(&mut pins, BlurGrab::Feather { pin: 0, dot: 0 }, centre);
    assert_eq!(pins.pins[0].iris.map(|s| s.feather), Some(0.0));

    let wild = IrisShape { radius_x: 9.0, radius_y: f64::NAN, angle: 270.0, roundness: -4.0, feather: 3.0 }.clean();
    assert_eq!(wild, IrisShape { radius_x: 1.0, radius_y: 0.25, angle: -90.0, roundness: 0.0, feather: 0.99 });
}

// Tilt-Shift: the solid lines set the focus, the dashed the transition, the dots the angle (folded
// into −90..=90); a click adds no pin.
#[test]
fn blur_pins_tilt_shift_lines_and_dots() {
    let mut pins = start(BlurKind::TiltShift);
    let pin = pins.pins[0];
    let Some(band) = pin.tilt else { panic!("a Tilt-Shift pin has a band") };
    let centre = pins.to_document(&pin);
    assert!(!pins.add(Point::new(10.0, 10.0)));
    assert_eq!(grab(&pins, &view(), screen(Point::new(10.0, 10.0)), &style()), BlurGrab::New);

    // The band is horizontal: its normal is vertical, so the lines sit above and below.
    let side = pins.short_side();
    let focus_line = screen(tilt_point(&pins, &pin, band, band.focus * side, 150.0));
    assert_eq!(grab(&pins, &view(), focus_line, &style()), BlurGrab::Focus { pin: 0 });
    let edge_line = screen(tilt_point(&pins, &pin, band, -(band.focus + band.transition) * side, 150.0));
    assert_eq!(grab(&pins, &view(), edge_line, &style()), BlurGrab::Transition { pin: 0 });
    let dot = screen(turn_handle(&pins, &pin, band, &view(), &style(), 1.0));
    assert_eq!(grab(&pins, &view(), dot, &style()), BlurGrab::Turn { pin: 0, side: 1 });

    dragged(&mut pins, BlurGrab::Focus { pin: 0 }, Point::new(centre.x + 33.0, centre.y - 160.0));
    assert!((pins.pins[0].tilt.map_or(0.0, |b| b.focus) - 0.2).abs() < 1e-9);
    dragged(&mut pins, BlurGrab::Transition { pin: 0 }, Point::new(centre.x, centre.y + 240.0));
    assert!((pins.pins[0].tilt.map_or(0.0, |b| b.transition) - 0.1).abs() < 1e-9);
    // The +1 dot dragged straight right turns the normal (sin a, cos a) to a = 90°.
    dragged(&mut pins, BlurGrab::Turn { pin: 0, side: 1 }, Point::new(centre.x + 300.0, centre.y));
    assert!((pins.pins[0].tilt.map_or(0.0, |b| b.angle) - 90.0).abs() < 1e-9);
    // The -1 dot dragged the same way is the other half turn: folded to −90.
    dragged(&mut pins, BlurGrab::Turn { pin: 0, side: -1 }, Point::new(centre.x + 300.0, centre.y));
    assert!((pins.pins[0].tilt.map_or(0.0, |b| b.angle) + 90.0).abs() < 1e-9);
    assert_eq!(TiltBand { angle: 135.0, focus: f64::NAN, transition: 0.0 }.clean(), TiltBand { angle: -45.0, focus: 0.1, transition: 0.01 });
}

// A whole turn of the ring sweeps the full range (Spin: 360°), clamped at the ends.
#[test]
fn blur_pins_spin_ring_sets_the_angle() {
    let mut pins = start(BlurKind::Spin);
    let centre = pins.to_document(&pins.pins[0]);
    let ring = screen(Point::new(centre.x + f64::from(style().ring), centre.y));
    assert_eq!(grab(&pins, &view(), ring, &style()), BlurGrab::Ring { pin: 0 });
    assert_eq!(BlurKind::Spin.max(), 360.0);
    pins.pins[0].amount = 0.0;

    // A quarter turn clockwise on screen (from the right to below the centre) adds 90°; four
    // quarter turns, one frame each, come to the full range.
    let round = [(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0), (1.0, 0.0)].map(|(x, y)| Point::new(centre.x + 100.0 * x, centre.y + 100.0 * y));
    for (index, pair) in round.windows(2).enumerate() {
        let frame = pins.clone();
        drag(&mut pins, BlurGrab::Ring { pin: 0 }, &frame, pair[0], pair[1]);
        assert!((pins.pins[0].amount - 90.0 * (index + 1) as f64).abs() < 1e-9);
    }
    // Further round stays at the top; back down comes off it.
    let frame = pins.clone();
    drag(&mut pins, BlurGrab::Ring { pin: 0 }, &frame, round[0], round[1]);
    assert_eq!(pins.pins[0].amount, 360.0);
    let frame = pins.clone();
    drag(&mut pins, BlurGrab::Ring { pin: 0 }, &frame, round[1], round[0]);
    assert!((pins.pins[0].amount - 270.0).abs() < 1e-9);

    let mut field = start(BlurKind::Field);
    field.pins[0].amount = 499.0;
    let frame = field.clone();
    drag(&mut field, BlurGrab::Ring { pin: 0 }, &frame, round[0], round[1]);
    assert_eq!(field.pins[0].amount, 500.0);
}

// A new pin copies the selected pin's amount and shape.
#[test]
fn blur_pins_new_pin_takes_the_selected_pins_amount_and_shape() {
    let mut pins = start(BlurKind::Iris);
    let shape = IrisShape { radius_x: 0.2, radius_y: 0.1, angle: 30.0, roundness: 40.0, feather: 0.3 };
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
#[test]
fn blur_pins_cursors_follow_the_grab() {
    let cases = [
        (BlurGrab::Ring { pin: 0 }, CursorIcon::Alias),
        (BlurGrab::Round { pin: 0 }, CursorIcon::Alias),
        (BlurGrab::Turn { pin: 0, side: 1 }, CursorIcon::Alias),
        (BlurGrab::Axis { pin: 0, axis: 1 }, CursorIcon::Move),
        (BlurGrab::Feather { pin: 0, dot: 0 }, CursorIcon::Move),
        (BlurGrab::Focus { pin: 0 }, CursorIcon::ResizeVertical),
        (BlurGrab::Transition { pin: 0 }, CursorIcon::ResizeVertical),
        (BlurGrab::Pin { pin: 0 }, CursorIcon::Grab),
        (BlurGrab::New, CursorIcon::Crosshair),
    ];
    for (grab, icon) in cases {
        assert_eq!(grab.cursor(false), icon, "{grab:?}");
    }
    assert_eq!(BlurGrab::New.cursor(true), CursorIcon::Default);

    // A press on the pin's dot grabs the pin; far from everything grabs nothing.
    let pins = start(BlurKind::Field);
    assert_eq!(grab(&pins, &view(), screen(Point::new(503.0, 400.0)), &style()), BlurGrab::Pin { pin: 0 });
    assert_eq!(grab(&pins, &view(), screen(Point::new(100.0, 100.0)), &style()), BlurGrab::New);
}
