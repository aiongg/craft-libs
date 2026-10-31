use egui::accesskit::Role;
use egui::{CursorIcon, Event, Modifiers, PointerButton, Pos2, Sense, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use kurbo::{Point, Rect, Size, Vec2};

use super::{CropBox, level_angle};
use crate::ui::{CropFrame, CropLabels, CropNames};
use craft_ui::canvas::{CanvasView, RulerUnits};
use craft_ui::edit::EditEvent;
use craft_ui::localized::LocaleNumbers;
use craft_ui::theme::{Base, install};

const RECT: Rect = Rect::new(100.0, 80.0, 300.0, 220.0);

#[derive(Default)]
struct Host {
    state: Option<CropBox>,
    armed: bool,
    rotation: f64,
    view: Option<CanvasView>,
    straightened: Vec<f64>,
    events: usize,
    claimed: bool,
}

fn harness(armed: bool, rotation: f64) -> Harness<'static, Host> {
    let host = Host { state: Some(CropBox::upright(RECT)), armed, rotation, ..Host::default() };
    Harness::<Host>::builder().with_size([480.0, 400.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut Host| {
            let (rect, canvas) = ui.allocate_exact_size(vec2(400.0, 300.0), Sense::click_and_drag());
            let view = CanvasView { rotation: host.rotation, ..CanvasView::new(rect, Point::new(200.0, 150.0), 1.0) };
            host.view = Some(view);
            let px = RulerUnits::pixels(LocaleNumbers::from_tag("en"));
            let state = host.state.unwrap_or(CropBox::upright(RECT));
            let names = CropNames {
                group: "Crop frame".into(),
                whole: "Whole frame".into(),
                corners: ["a", "b", "c", "d"].map(String::from),
                edges: ["e", "f", "g", "h"].map(String::from),
            };
            let r = CropFrame::new("crop", &state, names)
                .straighten(host.armed)
                .readout(CropLabels::new("W:", "H:", "Angle:"))
                .units(&px, &px)
                .show(ui, &canvas, &view);
            if let Some(EditEvent::Update { value, .. } | EditEvent::Commit { value, .. }) = &r.event {
                host.state = Some(*value);
            }
            host.events += usize::from(r.event.is_some());
            host.straightened.extend(r.straightened);
            host.claimed = r.claimed;
        },
        host,
    )
}

fn at(h: &Harness<'_, Host>, p: Point) -> Pos2 {
    h.state().view.expect("drawn").to_screen(p)
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

fn hold(h: &mut Harness<'_, Host>, from: Pos2, to: Pos2) {
    h.event(Event::PointerMoved(from));
    h.event(button(from, true));
    h.run();
    h.event(Event::PointerMoved(from.lerp(to, 0.5)));
    h.event(Event::PointerMoved(to));
    h.run();
}

fn drag(h: &mut Harness<'_, Host>, from: Pos2, to: Pos2) {
    hold(h, from, to);
    h.event(button(to, false));
    h.run();
}

/// A line of document length `len` from `from` at `deg` degrees clockwise on screen.
fn polar(from: Point, len: f64, deg: f64) -> Point {
    let (s, c) = deg.to_radians().sin_cos();
    from + Vec2::new(c * len, s * len)
}

#[test]
fn a_line_levels_a_horizon_and_plumbs_a_nearer_vertical_one() {
    let from = Point::new(60.0, 60.0);
    // Falling to the right, rising to the right, drawn leftwards, a wall leaning either way (a line at 85 has x growing downward).
    let cases = [(10.0, 10.0), (-7.0, -7.0), (170.0, -10.0), (95.0, 5.0), (85.0, -5.0), (-85.0, 5.0), (44.0, 44.0), (0.0, 0.0), (90.0, 0.0)];
    for (line, want) in cases {
        let mut h = harness(true, 0.0);
        h.run();
        let (a, b) = (at(&h, from), at(&h, polar(from, 120.0, line)));
        drag(&mut h, a, b);
        let got = h.state().straightened.clone();
        assert_eq!(got.len(), 1, "line at {line}: {got:?}");
        assert!((got[0] - want).abs() < 1e-3, "line at {line}: {got:?}");
    }
}

#[test]
fn a_line_is_levelled_in_the_document_not_on_the_screen() {
    // A page turned a quarter: a line along the screen's horizontal is along the document's vertical.
    let mut h = harness(true, 90.0);
    h.run();
    let a = at(&h, Point::new(200.0, 150.0));
    drag(&mut h, a, a + vec2(100.0, 0.0));
    assert_eq!(h.state().straightened.len(), 1);
    assert!(h.state().straightened[0].abs() < 1e-3, "{:?}", h.state().straightened);
    let mut h = harness(true, 90.0);
    h.run();
    let a = at(&h, Point::new(200.0, 150.0));
    drag(&mut h, a, a + vec2(100.0, 10.0));
    assert!((h.state().straightened[0].abs() - (10.0f64 / 100.0).atan().to_degrees()).abs() < 1e-3, "{:?}", h.state().straightened);
}

#[test]
fn a_line_under_the_minimum_does_nothing() {
    for len in [0.0, 2.9] {
        let mut h = harness(true, 0.0);
        h.run();
        let a = at(&h, Point::new(60.0, 60.0));
        drag(&mut h, a, a + vec2(len as f32, 0.0));
        assert!(h.state().straightened.is_empty(), "{len}");
    }
    let mut h = harness(true, 0.0);
    h.run();
    let a = at(&h, Point::new(60.0, 60.0));
    drag(&mut h, a, a + vec2(3.0, 0.0));
    assert_eq!(h.state().straightened.len(), 1, "3 points is the shortest line");
}

#[test]
fn the_drag_sends_no_edit_event_and_leaves_the_frame_alone() {
    let mut h = harness(true, 0.0);
    h.run();
    // A line that starts inside the frame would move it if Straighten were not armed.
    let a = at(&h, Point::new(150.0, 110.0));
    hold(&mut h, a, a + vec2(40.0, 12.0));
    assert!(h.state().claimed);
    assert_eq!(h.state().events, 0);
    assert_eq!(h.output().platform_output.cursor_icon, CursorIcon::Crosshair);
    h.event(button(a + vec2(40.0, 12.0), false));
    h.run();
    assert_eq!((h.state().events, h.state().straightened.len()), (0, 1));
    assert_eq!(h.state().state, Some(CropBox::upright(RECT)));
    // Not armed: the same drag moves the frame and straightens nothing.
    let mut h = harness(false, 0.0);
    h.run();
    drag(&mut h, a, a + vec2(40.0, 12.0));
    assert!(h.state().straightened.is_empty());
    assert!(h.state().events > 0);
}

#[test]
fn the_angle_shows_beside_the_pointer_while_the_line_is_drawn() {
    let mut h = harness(true, 0.0);
    h.run();
    let a = at(&h, Point::new(60.0, 60.0));
    hold(&mut h, a, a + vec2(100.0, -10.0));
    let text = h.query_by(|n| n.role() == Role::Tooltip).and_then(|n| n.accesskit_node().label()).unwrap_or_default();
    assert_eq!(text, "Angle: 5.7°");
    h.event(button(a, false));
    h.run();
    assert!(h.query_by(|n| n.role() == Role::Tooltip).is_none());
}

#[test]
fn level_angle_ignores_points_and_non_finite_lines() {
    assert_eq!(level_angle(Point::new(1.0, 1.0), Point::new(1.0, 1.0)), None);
    assert_eq!(level_angle(Point::new(f64::NAN, 0.0), Point::new(1.0, 1.0)), None);
    assert_eq!(level_angle(Point::ZERO, Point::new(-10.0, 0.0)), Some(0.0));
    assert!(level_angle(Point::ZERO, Point::new(-10.0, 0.0)).unwrap().is_sign_positive());
}

#[test]
fn fitted_is_the_largest_frame_of_the_same_proportions_inside_the_canvas() {
    let canvas = Size::new(200.0, 100.0);
    let frame = CropBox::upright(Rect::new(0.0, 0.0, 200.0, 100.0));
    assert_eq!(frame.fitted(canvas, 0.0), Some(frame));
    for deg in [-45.0, -10.0, 3.0, 10.0, 30.0, 45.0, 90.0] {
        let f = frame.fitted(canvas, deg).unwrap();
        let inside = |b: &CropBox| b.corners().iter().all(|p| p.x >= -1e-6 && p.y >= -1e-6 && p.x <= 200.0 + 1e-6 && p.y <= 100.0 + 1e-6);
        assert!(inside(&f), "{deg}: {f:?}");
        assert!((f.rect.width() / f.rect.height() - 2.0).abs() < 1e-9, "keeps 2:1 at {deg}");
        assert!(f.rect.center().distance(Point::new(100.0, 50.0)) < 1e-9, "centred at {deg}");
        assert_eq!(f.angle, deg);
        let c = f.rect.center();
        let bigger = CropBox {
            rect: Rect::new(c.x - f.rect.width() * 0.505, c.y - f.rect.height() * 0.505, c.x + f.rect.width() * 0.505, c.y + f.rect.height() * 0.505),
            angle: deg,
        };
        assert!(!inside(&bigger), "{deg}: 1% larger still fits");
    }
    // A 100x100 frame in a 200x100 canvas at 0 degrees is as large as the canvas allows.
    let square = CropBox::upright(Rect::new(10.0, 10.0, 110.0, 110.0)).fitted(canvas, 0.0).unwrap();
    assert_eq!(square.rect, Rect::new(50.0, 0.0, 150.0, 100.0));
    // A flipped rect keeps its proportions.
    assert_eq!(CropBox::upright(Rect::new(200.0, 100.0, 0.0, 0.0)).fitted(canvas, 0.0).unwrap().rect, Rect::new(0.0, 0.0, 200.0, 100.0));
    for bad in [Size::new(0.0, 100.0), Size::new(200.0, -1.0), Size::new(f64::NAN, 100.0)] {
        assert_eq!(frame.fitted(bad, 0.0), None);
    }
    assert_eq!(frame.fitted(canvas, f64::NAN), None);
    assert_eq!(CropBox::upright(Rect::new(1.0, 1.0, 1.0, 5.0)).fitted(canvas, 0.0), None);
}
