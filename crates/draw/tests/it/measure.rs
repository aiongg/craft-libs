//! The Ruler: its line model, the options bar, the handles and the tool, and the distances between boxes.

use craft_behavior::kurbo::{Point, Rect};
use craft_behavior::{Axis, CancelReason, CanvasView, EditEvent};
use craft_draw::ui::measure::{RulerBar, RulerLine, RulerOverlay, RulerPart, RulerTool, box_distances};
use craft_ui::bars::{Bar, BarKind};
use craft_ui::canvas::{CanvasTool, ToolAction, ToolContext, ToolKey, ToolPath, ToolPointer};
use craft_ui::contract::{Bind, Role, Show as _, ShowContainer as _};
use craft_ui::primitives::Draw;
use craft_ui::testing::{Fixture, ToolFixture, trial};
use egui::{Key, Modifiers, Pos2, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

/// X, Y, W, H, A and L1 of a line, and A (0 to 180) and L2 with a protractor, equal the engine's ruler info (y up for
/// the angle, -180 to 180).
#[trial]
fn reading_matches_the_engines_ruler_info() {
    // Down-right on screen is below the x axis with y up: -45 degrees.
    let r = RulerLine::new(Point::new(10.0, 20.0), Point::new(13.0, 24.0)).reading();
    assert_eq!((r.x, r.y, r.w, r.h, r.l1, r.l2), (10.0, 20.0, 3.0, 4.0, 5.0, None));
    assert!(near(r.angle, -(4.0f64).atan2(3.0).to_degrees()));
    // Up-left: 135 degrees.
    let r = RulerLine::new(Point::new(0.0, 0.0), Point::new(-1.0, -1.0)).reading();
    assert!(near(r.angle, 135.0));
    // Protractor: the angle between the arms, 0 to 180, and the second arm's length.
    let line = RulerLine::new(Point::new(0.0, 0.0), Point::new(10.0, 0.0)).with_protractor(Point::new(0.0, -10.0));
    let r = line.reading();
    assert!(near(r.angle, 90.0) && near(r.l1, 10.0) && r.l2.is_some_and(|l| near(l, 10.0)));
    let wide = RulerLine::new(line.start, line.end).with_protractor(Point::new(-5.0, 5.0));
    assert!(near(wide.reading().angle, 135.0));
    let skew = RulerLine::new(Point::new(0.0, 0.0), Point::new(10.0, -1.0)).with_protractor(Point::new(10.0, 1.0));
    assert!((0.0..=180.0).contains(&skew.reading().angle));
}

/// `scaled` multiplies X, Y, W, H, L1 and L2 and keeps A.
#[trial]
fn scaled_multiplies_lengths_and_positions_not_the_angle() {
    let line = RulerLine::new(Point::new(1.0, 2.0), Point::new(4.0, 6.0)).with_protractor(Point::new(1.0, 12.0));
    let (r, s) = (line.reading(), line.reading().scaled(2.0));
    assert_eq!((s.x, s.y, s.w, s.h, s.l1), (2.0, 4.0, 6.0, 8.0, 10.0));
    assert_eq!(s.l2, r.l2.map(|l| l * 2.0));
    assert_eq!(s.angle, r.angle);
}

/// `part_at` within the reach prefers the protractor's end, then the end, then the start; outside the reach it is
/// `None`.
#[trial]
fn a_press_finds_the_protractor_then_the_end_then_the_start() {
    let line = RulerLine::new(Point::new(0.0, 0.0), Point::new(1.0, 0.0)).with_protractor(Point::new(0.5, 0.0));
    assert_eq!(line.part_at(Point::new(0.5, 0.0), 2.0), Some(RulerPart::Protractor));
    assert_eq!(line.part_at(Point::new(0.5, 0.0), 0.2), Some(RulerPart::Protractor));
    let line = RulerLine::new(line.start, line.end);
    assert_eq!(line.part_at(Point::new(0.5, 0.0), 2.0), Some(RulerPart::End));
    assert_eq!(line.part_at(Point::new(-1.5, 0.0), 2.0), Some(RulerPart::Start));
    assert_eq!(line.part_at(Point::new(50.0, 50.0), 2.0), None);
}

/// Boxes apart on one axis give one gap, overlapping boxes the distances between like edges, diagonal boxes a line
/// through the first box's centre with dashed extensions of the second box's edges, a box inside another four
/// distances; zero and non-finite distances are dropped; labels come from the closure with the axis.
#[trial]
fn apart_overlapping_diagonal_and_inside() {
    let run = |a: Rect, b: Rect| {
        let (lines, measures) = box_distances(a, b, |len, axis| format!("{len}{}", if axis == Axis::X { "x" } else { "y" }));
        assert!(lines.iter().all(|l| l.extension));
        let measures: Vec<_> = measures.into_iter().map(|m| (m.a, m.b, m.label.unwrap_or_default())).collect();
        (measures, lines.into_iter().map(|l| (l.a, l.b)).collect::<Vec<_>>())
    };
    let p = Point::new;
    let (m, e) = run(Rect::new(0.0, 0.0, 40.0, 40.0), Rect::new(100.0, 20.0, 150.0, 80.0));
    assert_eq!(
        m,
        vec![
            (p(40.0, 30.0), p(100.0, 30.0), "60x".into()),
            (p(20.0, 0.0), p(20.0, 20.0), "20y".into()),
            (p(20.0, 40.0), p(20.0, 80.0), "40y".into())
        ]
    );
    assert_eq!(e, vec![(p(100.0, 20.0), p(20.0, 20.0)), (p(100.0, 80.0), p(20.0, 80.0))]);

    let (m, e) = run(Rect::new(100.0, 0.0, 140.0, 20.0), Rect::new(20.0, 70.0, 70.0, 90.0));
    assert_eq!(m, vec![(p(70.0, 10.0), p(100.0, 10.0), "30x".into()), (p(120.0, 20.0), p(120.0, 70.0), "50y".into())]);
    assert_eq!(e, vec![(p(70.0, 70.0), p(70.0, 10.0)), (p(70.0, 70.0), p(120.0, 70.0))]);

    let (m, e) = run(Rect::new(40.0, 40.0, 90.0, 80.0), Rect::new(0.0, 0.0, 400.0, 300.0));
    assert_eq!(m.len(), 4);
    assert_eq!((m[0].0, m[0].1), (p(0.0, 60.0), p(40.0, 60.0)));
    assert_eq!((m[3].0, m[3].1), (p(65.0, 80.0), p(65.0, 300.0)));
    assert!(e.is_empty());

    let canvas = Rect::new(0.0, 0.0, 400.0, 300.0);
    assert!(run(canvas, canvas).0.is_empty());
    assert!(run(Rect::new(f64::NAN, 0.0, 1.0, 1.0), canvas).0.is_empty());
}

fn bar_rig(line: Option<RulerLine>, scale: bool, factor: f64) -> Fixture {
    let mut fixture = Fixture::new(move |ui| {
        let mut use_scale = scale;
        Bar::new("Options", BarKind::Options).show(ui, |ui, _| {
            RulerBar::new(line, Bind::bound(&mut use_scale)).factor(factor).show(ui);
        });
    });
    fixture.room(Vec2::new(1200.0, 120.0)).settle();
    fixture
}

/// The value of the readout field named `label`: the field's group node carries the text its body shows.
fn readout(fixture: &Fixture, label: &str) -> Option<String> {
    let tree = fixture.tree();
    let group = fixture.by_role(Role::GROUP).into_iter().find(|group| tree.name_of(group.node) == Some(label))?;
    tree.value_of(group.node).map(str::to_owned)
}

/// Use Measurement Scale multiplies the lengths and positions in the readouts, not the angle.
#[trial]
fn scale_multiplies_lengths_not_the_angle() {
    let line = RulerLine::new(Point::new(1.0, 2.0), Point::new(4.0, 6.0));
    let plain = bar_rig(Some(line), false, 2.0);
    let scaled = bar_rig(Some(line), true, 2.0);
    assert!(readout(&plain, "L1").is_some());
    assert_ne!(readout(&plain, "L1"), readout(&scaled, "L1"));
    assert_eq!(readout(&plain, "A"), readout(&scaled, "A"));
}

/// Straighten Layer and Clear are disabled without a line.
#[trial]
fn buttons_are_disabled_without_a_line() {
    let none = bar_rig(None, false, 1.0);
    let disabled = |fixture: &Fixture, label: &str| fixture.tree().is_disabled_key(fixture.by_label(label).expect(label).node);
    assert!(disabled(&none, "Straighten Layer") && disabled(&none, "Clear"));
    let some = bar_rig(Some(RulerLine::new(Point::new(0.0, 0.0), Point::new(3.0, 4.0))), false, 1.0);
    assert!(!disabled(&some, "Straighten Layer") && !disabled(&some, "Clear"));
}

/// The readouts write lengths in the canvas's ruler units: document pixels by default.
#[trial]
fn readouts_show_the_ruler_units() {
    let rig = bar_rig(Some(RulerLine::new(Point::new(3.0, 4.0), Point::new(6.0, 8.0))), false, 1.0);
    let value = readout(&rig, "L1");
    assert!(value.as_ref().is_some_and(|v| v.contains("5.00") && v.contains("px")), "{value:?}");
}

/// The tool for the fixture: it builds a pass over the shared line for every call and applies the events the pass
/// reported, as the app would.
struct Driven {
    tool: RulerTool,
    line: Rc<RefCell<Option<RulerLine>>>,
    log: Rc<RefCell<Vec<&'static str>>>,
}

impl Driven {
    fn sync(&mut self) {
        for event in self.tool.take_events() {
            let (phase, value) = match &event {
                EditEvent::Begin { .. } => ("begin", None),
                EditEvent::Update { value, .. } => ("update", Some(*value)),
                EditEvent::Commit { after, .. } => ("commit", Some(*after)),
                EditEvent::Set { after, .. } => ("set", Some(*after)),
                EditEvent::Cancel { before, .. } => (
                    "cancel",
                    before.as_ref().and_then(|b| match b {
                        craft_behavior::Mixed::Same(v) => Some(*v),
                        _ => None,
                    }),
                ),
                _ => ("other", None),
            };
            if let Some(value) = value {
                *self.line.borrow_mut() = value;
            }
            self.log.borrow_mut().push(phase);
        }
    }

    fn with_pass<R>(&mut self, call: impl FnOnce(&mut dyn CanvasTool) -> R) -> R {
        let line = *self.line.borrow();
        let result = call(&mut self.tool.pass(line));
        self.sync();
        result
    }
}

impl CanvasTool for Driven {
    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        self.with_pass(|pass| pass.press(pointer, cx))
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        self.with_pass(|pass| pass.drag(pointer, cx));
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        self.with_pass(|pass| pass.release(pointer, cx));
    }

    fn cancel(&mut self, reason: CancelReason, cx: &mut ToolContext<'_>) {
        self.with_pass(|pass| pass.cancel(reason, cx));
    }

    fn key(&mut self, key: &ToolKey, cx: &mut ToolContext<'_>) -> bool {
        self.with_pass(|pass| pass.key(key, cx))
    }

    fn paint(&self, _: &Draw<'_>, _: &ToolContext<'_>) {}

    fn command_form(&self) -> ToolPath {
        ToolPath::Command("the Ruler Line... command")
    }
}

struct Shown {
    fixture: ToolFixture<Driven>,
    line: Rc<RefCell<Option<RulerLine>>>,
    log: Rc<RefCell<Vec<&'static str>>>,
}

fn shown(start: Option<RulerLine>) -> Shown {
    let line = Rc::new(RefCell::new(start));
    let log = Rc::new(RefCell::new(Vec::new()));
    let driven = Driven { tool: RulerTool::new(), line: Rc::clone(&line), log: Rc::clone(&log) };
    let body_line = Rc::clone(&line);
    let body_log = Rc::clone(&log);
    let fixture = ToolFixture::with_view(
        "Ruler",
        driven,
        |room| CanvasView::new(room, Point::new(f64::from(room.width()) / 2.0, f64::from(room.height()) / 2.0), 1.0),
        move |ui, body| {
            let mut line = *body_line.borrow();
            let out = RulerOverlay::new("Ruler", Bind::bound(&mut line)).show_at(ui, body.rect);
            if let Some(event) = &out.event {
                body_log.borrow_mut().push(match event {
                    EditEvent::Begin { .. } => "begin",
                    EditEvent::Update { .. } => "update",
                    EditEvent::Commit { .. } => "commit",
                    EditEvent::Set { .. } => "set",
                    _ => "other",
                });
            }
            *body_line.borrow_mut() = line;
        },
    );
    Shown { fixture, line, log }
}

impl Shown {
    /// The screen point of the document point `at`.
    fn screen(&self, at: (f64, f64)) -> Pos2 {
        let origin = self.fixture.canvas().rect.min;
        Pos2::new(origin.x + at.0 as f32, origin.y + at.1 as f32)
    }

    fn line(&self) -> RulerLine {
        self.line.borrow().expect("the app has a line")
    }
}

fn line(a: (f64, f64), b: (f64, f64)) -> Option<RulerLine> {
    Some(RulerLine::new(Point::new(a.0, a.1), Point::new(b.0, b.1)))
}

/// A press near an end grabs it; Alt at the far end swaps the ends and starts the protractor at the press; away from
/// every end a new line replaces the old one.
#[trial]
fn a_press_near_an_end_grabs_it_and_alt_starts_the_protractor() {
    let mut s = shown(line((100.0, 100.0), (200.0, 100.0)));
    let (from, to) = (s.screen((203.0, 102.0)), s.screen((250.0, 120.0)));
    s.fixture.press_at(from).move_to(to).release_at(to);
    let l = s.line();
    assert!(l.start.distance(Point::new(100.0, 100.0)) < 1.0 && l.end.distance(Point::new(250.0, 120.0)) < 1.0 && l.protractor.is_none(), "{l:?}");

    // Alt at the far end (not at the start) swaps the ends; the protractor starts at the press.
    s.fixture.fixture().hold(Modifiers::ALT);
    let (from, to) = (s.screen((252.0, 120.0)), s.screen((300.0, 200.0)));
    s.fixture.press_at(from).move_to(to).release_at(to);
    let l = s.line();
    assert!(l.start.distance(Point::new(250.0, 120.0)) < 1.0 && l.end.distance(Point::new(100.0, 100.0)) < 1.0, "swapped: {l:?}");
    assert!(l.protractor.is_some_and(|q| q.distance(Point::new(300.0, 200.0)) < 1.0), "{l:?}");

    s.fixture.fixture().hold(Modifiers::NONE);
    let (from, to) = (s.screen((50.0, 250.0)), s.screen((90.0, 250.0)));
    s.fixture.press_at(from).move_to(to).release_at(to);
    let l = s.line();
    assert!(l.start.distance(Point::new(50.0, 250.0)) < 1.0 && l.end.distance(Point::new(90.0, 250.0)) < 1.0 && l.protractor.is_none(), "{l:?}");
}

/// Shift moves a dragged end onto the nearest 45° ray about the fixed end, keeping the pointer's distance from it.
#[trial]
fn shift_keeps_the_pointer_distance_on_the_45_degree_ray() {
    let mut s = shown(line((100.0, 100.0), (200.0, 100.0)));
    let fixed = Point::new(100.0, 100.0);
    s.fixture.fixture().hold(Modifiers::SHIFT);
    let (from, to) = (s.screen((200.0, 100.0)), s.screen((260.0, 230.0)));
    s.fixture.press_at(from).move_to(to);
    // d = (160, 130): 39° snaps to 45°, the length stays 206.15 (a projection would give 205.1).
    let end = s.line().end;
    let length = 160.0_f64.hypot(130.0);
    assert!((end.distance(fixed) - length).abs() < 1.0, "{end:?}");
    assert!(((end.x - 100.0) - (end.y - 100.0)).abs() < 1.0, "{end:?}");
    s.fixture.release_at(to);
    assert!((s.line().end.distance(fixed) - length).abs() < 1.0);
}

/// A new line under 0.5 document units is cleared on release; a short drag of an existing end does not clear.
#[trial]
fn a_tiny_new_line_is_cleared_on_release() {
    let mut s = shown(None);
    let at = s.screen((50.0, 50.0));
    s.fixture.press_at(at).release_at(at);
    assert_eq!(*s.line.borrow(), None);
    assert!(s.log.borrow().contains(&"commit"));

    *s.line.borrow_mut() = line((100.0, 100.0), (200.0, 100.0));
    let at = s.screen((200.0, 100.0));
    s.fixture.press_at(at).release_at(at);
    assert!(s.line.borrow().is_some(), "an existing line stays");
}

/// Page Down picks Start, End, Protractor; arrows move the picked point 1 unit and Shift 10; Delete clears the line.
#[trial]
fn keys_move_the_picked_part_and_delete_clears() {
    let arm = RulerLine::new(Point::new(100.0, 100.0), Point::new(200.0, 100.0)).with_protractor(Point::new(100.0, 150.0));
    let mut s = shown(Some(arm));
    let key = |s: &mut Shown, key: Key, mods: Modifiers| {
        s.fixture.fixture().key(key, mods).settle();
    };
    // The canvas is the first Tab stop, the points group the second; its first point is picked.
    key(&mut s, Key::Tab, Modifiers::NONE);
    key(&mut s, Key::Tab, Modifiers::NONE);
    key(&mut s, Key::ArrowRight, Modifiers::NONE);
    assert!(s.line().start.distance(Point::new(101.0, 100.0)) < 1e-6, "{:?}", s.line());

    key(&mut s, Key::PageDown, Modifiers::NONE);
    key(&mut s, Key::ArrowDown, Modifiers::SHIFT);
    assert!(s.line().end.distance(Point::new(200.0, 110.0)) < 1e-6, "{:?}", s.line());

    key(&mut s, Key::PageDown, Modifiers::NONE);
    key(&mut s, Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(s.line().protractor, Some(Point::new(99.0, 150.0)));
    assert!(s.log.borrow().iter().all(|phase| *phase == "set"), "{:?}", s.log.borrow());

    key(&mut s, Key::Delete, Modifiers::NONE);
    assert_eq!(*s.line.borrow(), None);
}

/// With the canvas focused (not the points group), arrows move the whole line; with the points group focused they move
/// the picked point.
#[trial]
fn arrows_move_the_whole_line_unless_the_parts_group_has_the_keyboard() {
    let mut s = shown(line((100.0, 100.0), (200.0, 100.0)));
    // The canvas is the first Tab stop.
    s.fixture.fixture().key(Key::Tab, Modifiers::NONE).settle();
    s.fixture.fixture().key(Key::ArrowRight, Modifiers::NONE).settle();
    let l = s.line();
    assert!(l.start.distance(Point::new(101.0, 100.0)) < 1e-6 && l.end.distance(Point::new(201.0, 100.0)) < 1e-6, "{l:?}");

    // The points group is the second Tab stop: the arrows move the picked point only.
    s.fixture.fixture().key(Key::Tab, Modifiers::NONE).settle();
    s.fixture.fixture().key(Key::ArrowDown, Modifiers::NONE).settle();
    let l = s.line();
    assert!(l.start.distance(Point::new(101.0, 101.0)) < 1e-6 && l.end.distance(Point::new(201.0, 100.0)) < 1e-6, "{l:?}");
}

/// The tool owes the canvas a command form and a clean gesture: one begin, one commit, and Escape cancels.
#[trial]
fn the_ruler_tool_passes_the_tool_checks() {
    shown(None).fixture.check();
}
