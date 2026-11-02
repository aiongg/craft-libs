//! Tests of the Magnetic Lasso (task SE-F10), with a fake `EdgeTracer` that snaps to a circle and
//! traces along it, and a fake `SelectionProbe`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use craft_ui::canvas::kurbo::{Point, Size, Vec2};
use craft_ui::canvas::{CanvasId, CanvasView, SelectionOperation, ToolHost, ToolInput, ToolKey};
use craft_ui::edit::CancelReason;
use craft_ui::testing::canvas;
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, Sense, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::lasso_tests::{Fake, NONE};
use super::{EdgeSearch, EdgeTracer, MagneticLassoTool, SelectEvent, SelectPhase, SelectToolPass, SelectionShape};

const CENTRE: Point = Point::new(200.0, 150.0);
const RADIUS: f64 = 80.0;

/// What the fake tracer was asked.
#[derive(Debug, Default)]
struct Log {
    snaps: Cell<usize>,
    /// The `from` of every `trace`: the fastening points, with repeats.
    froms: RefCell<Vec<Point>>,
    searches: RefCell<Vec<EdgeSearch>>,
}

impl Log {
    /// The distinct points traces started from.
    fn starts(&self) -> usize {
        let mut seen: Vec<Point> = Vec::new();
        for &p in self.froms.borrow().iter() {
            if !seen.iter().any(|q| q.distance(p) < 1e-6) {
                seen.push(p);
            }
        }
        seen.len()
    }

    fn last_width(&self) -> f64 {
        self.searches.borrow().last().map_or(f64::NAN, |s| s.width)
    }
}

/// Edges lie on a circle of radius 80 round (200, 150): `snap` finds the nearest circle point
/// within the width, `trace` walks the shorter arc.
#[derive(Clone, Debug)]
struct Circle {
    log: Rc<Log>,
}

fn on_circle(p: Point) -> bool {
    ((p - CENTRE).hypot() - RADIUS).abs() < 0.5
}

impl EdgeTracer for Circle {
    fn snap(&mut self, at: Point, search: EdgeSearch) -> Option<Point> {
        self.log.snaps.set(self.log.snaps.get() + 1);
        self.log.searches.borrow_mut().push(search);
        let d = at - CENTRE;
        let len = d.hypot();
        ((len - RADIUS).abs() <= search.width && len > 0.0).then(|| CENTRE + d * (RADIUS / len))
    }

    fn trace(&mut self, from: Point, to: Point, _guide: &[Point], search: EdgeSearch) -> Vec<Point> {
        self.log.froms.borrow_mut().push(from);
        self.log.searches.borrow_mut().push(search);
        if !on_circle(from) || !on_circle(to) {
            return vec![from, to];
        }
        let angle = |p: Point| (p.y - CENTRE.y).atan2(p.x - CENTRE.x);
        let mut delta = angle(to) - angle(from);
        if delta > std::f64::consts::PI {
            delta -= std::f64::consts::TAU;
        } else if delta < -std::f64::consts::PI {
            delta += std::f64::consts::TAU;
        }
        let n = ((delta.abs() * RADIUS / 2.0).ceil() as usize).max(1);
        let mut points: Vec<Point> = (1..n)
            .map(|k| {
                let a = angle(from) + delta * k as f64 / n as f64;
                CENTRE + vec2_of(a)
            })
            .collect();
        points.insert(0, from);
        points.push(to);
        points
    }
}

fn vec2_of(angle: f64) -> Vec2 {
    Vec2::new(angle.cos(), angle.sin()) * RADIUS
}

fn on(angle: f64) -> Point {
    CENTRE + vec2_of(angle)
}

struct Rig {
    tool: MagneticLassoTool<Circle>,
    probe: Fake,
    log: Rc<Log>,
    id: Option<CanvasId>,
    focus: bool,
    took_key: Option<ToolKey>,
}

impl Rig {
    fn events(&mut self) -> Vec<SelectEvent> {
        self.tool.take_events()
    }
}

fn rig() -> Harness<'static, Rig> {
    let view = CanvasView::new(egui::Rect::from_min_size(pos2(0.0, 0.0), vec2(400.0, 300.0)), CENTRE, 1.0);
    let log = Rc::new(Log::default());
    let state = Rig { tool: MagneticLassoTool::new(Circle { log: log.clone() }), probe: Fake::empty(), log, id: None, focus: true, took_key: None };
    let mut h = Harness::builder().with_size(vec2(400.0, 300.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        move |ui, rig: &mut Rig| {
            let canvas_id = CanvasId::new(ui, "canvas");
            let canvas = canvas_id.interact(ui, view.rect, Sense::click_and_drag() | Sense::FOCUSABLE);
            if std::mem::take(&mut rig.focus) {
                canvas.request_focus();
            }
            let host = ToolHost::new(canvas_id).document(Size::new(400.0, 300.0));
            let r = host.show(ui, &view, &canvas, &mut SelectToolPass::new(&mut rig.tool, &rig.probe));
            rig.id = Some(r.id);
            rig.took_key = rig.took_key.or(r.took_key);
        },
        state,
    );
    h.step();
    h.step();
    h
}

fn input(h: &mut Harness<'_, Rig>, input: ToolInput) {
    let Some(id) = h.state().id else { panic!("a pass ran") };
    ToolHost::inject(&h.ctx, id, input);
    h.step();
}

/// A sample with a mouse's pen fields.
fn sample(at: Point, modifiers: Modifiers) -> (Point, f32, [f32; 2], Modifiers) {
    (at, 1.0, [0.0; 2], modifiers)
}

fn down(h: &mut Harness<'_, Rig>, at: Point, modifiers: Modifiers) {
    let (doc, pressure, tilt, modifiers) = sample(at, modifiers);
    input(h, ToolInput::Down { button: egui::PointerButton::Primary, doc, pressure, tilt, rotation: 0.0, modifiers });
}

fn up(h: &mut Harness<'_, Rig>, at: Point, modifiers: Modifiers) {
    input(h, ToolInput::Up { doc: at, modifiers });
}

/// A move with no button down, or a drag while one is.
fn glide(h: &mut Harness<'_, Rig>, at: Point, modifiers: Modifiers) {
    let (doc, pressure, tilt, modifiers) = sample(at, modifiers);
    input(h, ToolInput::Move { doc, pressure, tilt, rotation: 0.0, modifiers });
}

fn key_with(h: &mut Harness<'_, Rig>, key: Key, modifiers: Modifiers) {
    h.event(Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers });
    h.step();
}

fn press_key(h: &mut Harness<'_, Rig>, key: Key) {
    key_with(h, key, NONE);
}

/// The modifiers the canvas sees from the next pass on.
fn hold(h: &mut Harness<'_, Rig>, modifiers: Modifiers) {
    h.event(Event::ModifiersChanged(modifiers));
    h.step();
}

fn commit_points(events: &[SelectEvent]) -> (Vec<Point>, SelectionOperation) {
    let [SelectEvent::Commit { shape: SelectionShape::Polygon(points), operation }] = events else { panic!("one polygon commit: {events:?}") };
    (points.clone(), *operation)
}

// Points fasten every `spacing` along the edge at frequencies 0, 57 and 100, and a long jump is
// walked in at most 256 steps of `spacing / 2`.
#[test]
fn magnetic_fastens_every_spacing_and_walks_jumps() {
    let start = on(0.0);
    let fastened = |frequency: f32| {
        let mut h = rig();
        h.state_mut().tool.settings_mut().frequency = frequency;
        down(&mut h, start, NONE);
        up(&mut h, start, NONE);
        // One hover sample 2.5 rad (200 px of arc) along the circle.
        glide(&mut h, on(2.5), NONE);
        h.state().log.starts()
    };
    let (slow, normal, fast) = (fastened(0.0), fastened(57.0), fastened(100.0));
    assert!(slow < normal && normal < fast, "{slow} {normal} {fast}");
    // 200 px at 47 px: the start and four fastened points.
    assert!((4..=6).contains(&normal), "{normal}");

    let mut h = rig();
    h.state_mut().tool.settings_mut().frequency = 100.0;
    down(&mut h, start, NONE);
    up(&mut h, start, NONE);
    glide(&mut h, Point::new(5000.0, 150.0), NONE);
    assert_eq!(h.state().log.snaps.get(), 1 + 256, "the start, then 256 steps");
}

// ⌥-click adds a straight segment to the pointer; ⌥-drag draws freehand and fastens where it
// stops.
#[test]
fn magnetic_alt_click_straight_alt_drag_freehand() {
    let mut h = rig();
    let start = on(0.0);
    down(&mut h, start, NONE);
    up(&mut h, start, NONE);
    click_at(&mut h, Point::new(250.0, 60.0), Modifiers::ALT);
    down(&mut h, Point::new(220.0, 60.0), Modifiers::ALT);
    glide(&mut h, Point::new(210.0, 62.0), Modifiers::ALT);
    glide(&mut h, Point::new(210.1, 62.1), Modifiers::ALT);
    glide(&mut h, Point::new(200.0, 66.0), Modifiers::ALT);
    up(&mut h, Point::new(200.0, 66.0), Modifiers::ALT);
    key_with(&mut h, Key::Enter, Modifiers::ALT);
    let (points, operation) = commit_points(&h.state_mut().events());
    assert_eq!(operation, SelectionOperation::New);
    let expected = [start, Point::new(250.0, 60.0), Point::new(220.0, 60.0), Point::new(210.0, 62.0), Point::new(200.0, 66.0)];
    assert_eq!(points, expected, "0.5 px spacing drops the near sample");

    // ⌥ pressed during a drag anchors the pointer and draws freehand from it.
    let mut h = rig();
    down(&mut h, start, NONE);
    glide(&mut h, on(0.5), NONE);
    glide(&mut h, Point::new(270.0, 100.0), NONE);
    hold(&mut h, Modifiers::ALT);
    glide(&mut h, Point::new(260.0, 90.0), Modifiers::ALT);
    up(&mut h, Point::new(260.0, 90.0), Modifiers::ALT);
    key_with(&mut h, Key::Enter, Modifiers::ALT);
    let (points, _) = commit_points(&h.state_mut().events());
    assert_eq!(points.first(), Some(&start));
    assert_eq!(points[points.len() - 2..], [Point::new(270.0, 100.0), Point::new(260.0, 90.0)]);
}

fn click_at(h: &mut Harness<'_, Rig>, at: Point, modifiers: Modifiers) {
    down(h, at, modifiers);
    up(h, at, modifiers);
}

// Backspace removes fastening points back to the first; one more cancels with no event.
#[test]
fn magnetic_remove_last_point_and_cancel() {
    let mut h = rig();
    let start = on(0.0);
    down(&mut h, start, NONE);
    up(&mut h, start, NONE);
    click_at(&mut h, on(1.0), NONE);
    click_at(&mut h, on(2.0), NONE);
    let last_start = |h: &Harness<'_, Rig>| h.state().log.froms.borrow().last().copied();
    // Each Backspace traces from the fastening point before: the pointer stays where it was.
    press_key(&mut h, Key::Backspace);
    assert!(last_start(&h).is_some_and(|p| p.distance(on(1.0)) < 1e-6));
    assert!(matches!(h.state().tool.phase(), SelectPhase::Outline { .. }));
    press_key(&mut h, Key::Backspace);
    assert!(last_start(&h).is_some_and(|p| p.distance(start) < 1e-6), "back to the first point");
    assert!(matches!(h.state().tool.phase(), SelectPhase::Outline { .. }), "the first point stays");
    press_key(&mut h, Key::Backspace);
    assert_eq!(h.state().tool.phase(), SelectPhase::Idle);
    assert!(h.state_mut().events().is_empty());
}

// Esc during a press drops the whole outline; a cancel for another reason (a tool switch) keeps it
// for the app's `reset`.
#[test]
fn magnetic_escape_during_press_drops_outline() {
    let mut h = rig();
    click_at(&mut h, on(0.0), NONE);
    canvas::press(&mut h, pos2(on(1.0).x as f32, on(1.0).y as f32), NONE);
    assert!(matches!(h.state().tool.phase(), SelectPhase::Outline { .. }));
    canvas::key(&mut h, Key::Escape, NONE);
    assert_eq!(h.state().tool.phase(), SelectPhase::Idle);
    canvas::release(&mut h, pos2(on(1.0).x as f32, on(1.0).y as f32), NONE);
    assert!(h.state_mut().events().is_empty());

    click_at(&mut h, on(0.0), NONE);
    canvas::press(&mut h, pos2(on(1.0).x as f32, on(1.0).y as f32), NONE);
    let (id, ctx) = (h.state().id.expect("a pass ran"), h.ctx.clone());
    let rig = h.state_mut();
    let _ = ToolHost::cancel(&ctx, id, &mut SelectToolPass::new(&mut rig.tool, &rig.probe), CancelReason::Removed);
    assert!(matches!(h.state().tool.phase(), SelectPhase::Outline { .. }), "a tool switch keeps the outline");
}

// `[` and `]` change the width by 1 within 1..=256, with or without an outline open.
#[test]
fn magnetic_bracket_keys_change_width_within_range() {
    let mut h = rig();
    press_key(&mut h, Key::OpenBracket);
    assert_eq!(h.state().tool.settings().width, 9.0);
    assert_eq!(h.state_mut().took_key.take().map(|k| k.key), Some(Key::OpenBracket));
    let start = on(0.0);
    down(&mut h, start, NONE);
    up(&mut h, start, NONE);
    press_key(&mut h, Key::CloseBracket);
    press_key(&mut h, Key::CloseBracket);
    assert_eq!(h.state().tool.settings().width, 11.0);
    h.state_mut().tool.settings_mut().width = 1.0;
    press_key(&mut h, Key::OpenBracket);
    assert_eq!(h.state().tool.settings().width, 1.0);
    h.state_mut().tool.settings_mut().width = 256.0;
    press_key(&mut h, Key::CloseBracket);
    assert_eq!(h.state().tool.settings().width, 256.0);
}

// Enter closes along the edges (the tracer's path), ⌥ Enter with a straight segment, a click on the
// first point closes too; each commits one `Polygon` with the latched operation.
#[test]
fn magnetic_closes_along_edges_or_straight() {
    let start = on(0.0);
    // Enter: along the circle, every point on it.
    let mut h = rig();
    h.state_mut().tool.operation(SelectionOperation::Add);
    down(&mut h, start, NONE);
    up(&mut h, start, NONE);
    click_at(&mut h, on(1.5), NONE);
    key_with(&mut h, Key::Enter, NONE);
    let (points, operation) = commit_points(&h.state_mut().events());
    assert_eq!(operation, SelectionOperation::Add);
    assert!(points.len() > 6 && points.iter().all(|&p| on_circle(p)), "{points:?}");
    assert_ne!(points.last(), points.first(), "the outline does not repeat its start");
    assert_eq!(h.state().tool.phase(), SelectPhase::Idle);

    // ⌥ Enter: the last point joins the first with a straight segment.
    let mut h = rig();
    down(&mut h, start, NONE);
    up(&mut h, start, NONE);
    click_at(&mut h, Point::new(250.0, 60.0), Modifiers::ALT);
    click_at(&mut h, Point::new(150.0, 60.0), Modifiers::ALT);
    key_with(&mut h, Key::Enter, Modifiers::ALT);
    assert_eq!(commit_points(&h.state_mut().events()).0, [start, Point::new(250.0, 60.0), Point::new(150.0, 60.0)]);

    // A click within the close radius of the first point.
    let mut h = rig();
    down(&mut h, start, NONE);
    up(&mut h, start, NONE);
    click_at(&mut h, on(1.5), NONE);
    click_at(&mut h, on(3.0), NONE);
    assert!(h.state_mut().events().is_empty());
    click_at(&mut h, start + Vec2::new(3.0, 3.0), NONE);
    let (points, _) = commit_points(&h.state_mut().events());
    assert!(points.len() > 6);
    assert_eq!(h.state().tool.phase(), SelectPhase::Idle);
}

// With `pressure_width` on, a pen sample at pressure 0.5 halves the searched width (minus one);
// a mouse sample never narrows it.
#[test]
fn magnetic_stylus_pressure_narrows_width() {
    let mut h = rig();
    h.state_mut().tool.settings_mut().pressure_width = true;
    let start = on(0.0);
    down(&mut h, start, NONE);
    up(&mut h, start, NONE);
    glide(&mut h, on(0.2), NONE);
    assert_eq!(h.state().log.last_width(), 10.0, "a mouse");
    input(&mut h, ToolInput::Move { doc: on(0.4), pressure: 0.5, tilt: [10.0, 0.0], rotation: 0.0, modifiers: NONE });
    assert_eq!(h.state().log.last_width(), 5.5, "1 + 9 × 0.5");
    h.state_mut().tool.settings_mut().pressure_width = false;
    input(&mut h, ToolInput::Move { doc: on(0.6), pressure: 0.5, tilt: [10.0, 0.0], rotation: 0.0, modifiers: NONE });
    assert_eq!(h.state().log.last_width(), 10.0, "the option is off");
}

// While the Magnetic Lasso's outline is open the tool's `node` is "Selection outline" with its
// point count (fastening points and the live trace) and bounds.
#[test]
fn magnetic_outline_being_drawn_is_one_accesskit_object() {
    let mut h = rig();
    h.run();
    assert!(h.query_by_label("Selection outline").is_none());
    let start = on(0.0);
    down(&mut h, start, NONE);
    up(&mut h, start, NONE);
    glide(&mut h, on(0.3), NONE);
    h.run();
    let SelectPhase::Outline { points } = h.state().tool.phase() else { panic!("open") };
    let node = h.get_by_label("Selection outline");
    assert_eq!(node.accesskit_node().role(), Role::GraphicsObject);
    assert_eq!(node.accesskit_node().value().as_deref(), Some(format!("{points} points").as_str()));
    h.state_mut().tool.reset();
    h.run();
    assert!(h.query_by_label("Selection outline").is_none());
}
