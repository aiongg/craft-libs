//! Tests of the Ruler tool on the canvas tool host (task TO-R2).

use egui::{Event, Key, Modifiers, PointerButton, Sense, pos2, vec2};
use egui_kittest::Harness;
use kurbo::Point;

use super::{RulerInput, RulerLine, RulerTool};
use craft_ui::canvas::{CanvasId, CanvasView, ToolHost, ToolInput};
use craft_ui::edit::EditEvent;
use craft_ui::theme::{Base, install};

type Line = Option<RulerLine>;

struct Host {
    tool: RulerTool,
    /// The app's stored line: every event applied as the app would.
    line: Line,
    /// The line before the running gesture began, for Cancel.
    saved: Line,
    view: CanvasView,
    canvas: Option<CanvasId>,
    events: Vec<EditEvent<Line>>,
}

fn harness(line: Line) -> Harness<'static, Host> {
    let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, vec2(400.0, 300.0));
    let host = Host {
        tool: RulerTool::new(),
        line,
        saved: None,
        view: CanvasView::new(rect, Point::new(200.0, 150.0), 1.0),
        canvas: None,
        events: Vec::new(),
    };
    let mut h = Harness::builder()
        .with_size(vec2(400.0, 300.0))
        .with_step_dt(0.05)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, host: &mut Host| {
                let id = CanvasId::new(ui, "ruler");
                host.canvas = Some(id);
                let canvas = id.interact(ui, host.view.rect, Sense::click_and_drag() | Sense::FOCUSABLE);
                let mut pass = host.tool.pass(RulerInput::new(host.line));
                let _ = ToolHost::new(id).show(ui, &host.view, &canvas, &mut pass);
                for event in pass.finish().events {
                    match &event {
                        EditEvent::Begin { value, .. } => {
                            host.saved = host.line;
                            if let Some(v) = value {
                                host.line = *v;
                            }
                        }
                        EditEvent::Update { value, .. } | EditEvent::Commit { value, .. } | EditEvent::Set { value, .. } => host.line = *value,
                        EditEvent::Cancel { .. } => host.line = host.saved,
                        _ => {}
                    }
                    host.events.push(event);
                }
            },
            host,
        );
    h.step();
    h
}

fn line(a: (f64, f64), b: (f64, f64)) -> Line {
    Some(RulerLine::new(Point::new(a.0, a.1), Point::new(b.0, b.1)))
}

fn input(h: &mut Harness<'_, Host>, input: impl FnOnce(Modifiers) -> ToolInput, m: Modifiers) {
    let Some(id) = h.state().canvas else { panic!("a pass ran") };
    ToolHost::inject(&h.ctx, id, input(m));
    h.step();
}

fn down(h: &mut Harness<'_, Host>, at: (f64, f64), m: Modifiers) {
    input(
        h,
        |modifiers| ToolInput::Down {
            button: egui::PointerButton::Primary,
            doc: Point::new(at.0, at.1),
            pressure: 1.0,
            tilt: [0.0; 2],
            rotation: 0.0,
            modifiers,
        },
        m,
    );
}

fn to(h: &mut Harness<'_, Host>, at: (f64, f64), m: Modifiers) {
    input(h, |modifiers| ToolInput::Move { doc: Point::new(at.0, at.1), pressure: 1.0, tilt: [0.0; 2], rotation: 0.0, modifiers }, m);
}

fn up(h: &mut Harness<'_, Host>, at: (f64, f64), m: Modifiers) {
    input(h, |modifiers| ToolInput::Up { doc: Point::new(at.0, at.1), modifiers }, m);
}

fn current(h: &Harness<'_, Host>) -> RulerLine {
    let Some(line) = h.state().line else { panic!("the app has a line") };
    line
}

fn near(a: Point, b: (f64, f64)) -> bool {
    a.distance(Point::new(b.0, b.1)) < 1e-6
}

fn phases(h: &Harness<'_, Host>) -> Vec<&'static str> {
    h.state()
        .events
        .iter()
        .map(|e| match e {
            EditEvent::Begin { .. } => "begin",
            EditEvent::Update { .. } => "update",
            EditEvent::Commit { .. } => "commit",
            EditEvent::Cancel { .. } => "cancel",
            EditEvent::Set { .. } => "set",
            _ => "other",
        })
        .collect()
}

// Through `ToolHost`: a press within 7 points of the end drags it; Alt at the far end swaps the ends and starts the protractor at the vertex; elsewhere a new line starts; the events are Begin, Update, Commit.
#[test]
fn a_press_near_an_end_grabs_it_and_alt_starts_the_protractor() {
    let mut h = harness(line((100.0, 100.0), (200.0, 100.0)));
    let none = Modifiers::NONE;

    down(&mut h, (203.0, 102.0), none);
    to(&mut h, (250.0, 120.0), none);
    up(&mut h, (250.0, 120.0), none);
    let l = current(&h);
    assert!(near(l.start, (100.0, 100.0)) && near(l.end, (250.0, 120.0)) && l.protractor.is_none(), "{l:?}");
    assert_eq!(phases(&h), ["begin", "update", "commit"]);

    // Alt at the far end (not at the start) swaps the ends; the protractor starts at the press.
    h.state_mut().events.clear();
    down(&mut h, (252.0, 120.0), Modifiers::ALT);
    let l = current(&h);
    assert!(near(l.start, (250.0, 120.0)) && near(l.end, (100.0, 100.0)), "swapped: {l:?}");
    assert_eq!(l.protractor, Some(Point::new(252.0, 120.0)));
    to(&mut h, (300.0, 200.0), Modifiers::ALT);
    up(&mut h, (300.0, 200.0), Modifiers::ALT);
    assert_eq!(current(&h).protractor, Some(Point::new(300.0, 200.0)));
    assert_eq!(phases(&h), ["begin", "update", "commit"]);

    // Away from every end a new line starts and replaces the old one.
    h.state_mut().events.clear();
    down(&mut h, (50.0, 250.0), none);
    to(&mut h, (90.0, 250.0), none);
    up(&mut h, (90.0, 250.0), none);
    let l = current(&h);
    assert!(near(l.start, (50.0, 250.0)) && near(l.end, (90.0, 250.0)) && l.protractor.is_none(), "{l:?}");
}

// Shift keeps a dragged end on 45° rays about the other end, and the protractor's end about the vertex.
#[test]
fn shift_constrains_to_45_degrees_about_the_fixed_end() {
    let mut h = harness(line((100.0, 100.0), (200.0, 100.0)));
    let shift = Modifiers::SHIFT;

    down(&mut h, (200.0, 100.0), Modifiers::NONE);
    to(&mut h, (260.0, 150.0), shift);
    assert!(near(current(&h).end, (260.0, 100.0)), "{:?}", current(&h));
    to(&mut h, (260.0, 230.0), shift);
    assert!(near(current(&h).end, (245.0, 245.0)), "{:?}", current(&h));
    up(&mut h, (260.0, 230.0), shift);

    // The start end is constrained about the end.
    down(&mut h, (100.0, 100.0), Modifiers::NONE);
    to(&mut h, (30.0, 180.0), shift);
    let l = current(&h);
    // About the end (245, 245): d = (-215, -65) is 197° → 180°, projected length 215.
    assert!(near(l.start, (30.0, 245.0)), "{l:?}");
    up(&mut h, (30.0, 180.0), shift);

    // The protractor arm is constrained about the vertex.
    let vertex = current(&h).start;
    down(&mut h, (vertex.x, vertex.y), Modifiers::ALT);
    to(&mut h, (vertex.x + 10.0, vertex.y + 90.0), shift);
    let Some(q) = current(&h).protractor else { panic!("a protractor") };
    assert!(near(q, (vertex.x, vertex.y + 90.0)), "{q:?} about {vertex:?}");
}

// A new line under 0.5 document units commits `None`; a short drag of an existing end does not clear.
#[test]
fn a_tiny_new_line_is_cleared_on_release() {
    let mut h = harness(None);
    let none = Modifiers::NONE;
    down(&mut h, (50.0, 50.0), none);
    to(&mut h, (50.2, 50.0), none);
    assert!(h.state().line.is_some(), "the line shows while dragging");
    up(&mut h, (50.2, 50.0), none);
    assert_eq!(h.state().line, None);
    assert!(matches!(h.state().events.last(), Some(EditEvent::Commit { value: None, .. })));

    h.state_mut().line = line((100.0, 100.0), (200.0, 100.0));
    down(&mut h, (200.0, 100.0), none);
    to(&mut h, (200.2, 100.0), none);
    up(&mut h, (200.2, 100.0), none);
    assert!(h.state().line.is_some(), "an existing line stays");
}

// Page Down picks Start, End, Protractor; arrows move the picked point 1 unit, Shift 10, as `Set`; Delete gives `Set(None)`.
#[test]
fn keys_move_the_picked_part_and_delete_clears() {
    let mut start = RulerLine::new(Point::new(100.0, 100.0), Point::new(200.0, 100.0));
    start.protractor = Some(Point::new(100.0, 150.0));
    let mut h = harness(Some(start));
    let key = |h: &mut Harness<'_, Host>, key: Key, m: Modifiers| {
        h.key_press_modifiers(m, key);
        h.run();
    };

    // The canvas is the first Tab stop, the parts group the second; its first part is picked.
    key(&mut h, Key::Tab, Modifiers::NONE);
    key(&mut h, Key::Tab, Modifiers::NONE);
    key(&mut h, Key::ArrowRight, Modifiers::NONE);
    assert!(near(current(&h).start, (101.0, 100.0)), "{:?}", current(&h));

    key(&mut h, Key::PageDown, Modifiers::NONE);
    key(&mut h, Key::ArrowDown, Modifiers::SHIFT);
    assert!(near(current(&h).end, (200.0, 110.0)), "{:?}", current(&h));

    key(&mut h, Key::PageDown, Modifiers::NONE);
    key(&mut h, Key::ArrowLeft, Modifiers::NONE);
    assert_eq!(current(&h).protractor, Some(Point::new(99.0, 150.0)));
    assert!(near(current(&h).start, (101.0, 100.0)) && near(current(&h).end, (200.0, 110.0)));
    assert!(h.state().events.iter().all(|e| matches!(e, EditEvent::Set { .. })), "{:?}", phases(&h));

    key(&mut h, Key::Delete, Modifiers::NONE);
    assert_eq!(h.state().line, None);
    assert!(matches!(h.state().events.last(), Some(EditEvent::Set { value: None, .. })));
}

// Esc during a drag gives Cancel and the app's line stays the press's.
#[test]
fn esc_cancels_a_drag() {
    let before = line((100.0, 100.0), (200.0, 100.0));
    let mut h = harness(before);
    h.event(Event::PointerMoved(pos2(200.0, 100.0)));
    h.step();
    h.event(Event::PointerButton { pos: pos2(200.0, 100.0), button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    h.step();
    h.event(Event::PointerMoved(pos2(260.0, 160.0)));
    h.step();
    assert!(near(current(&h).end, (260.0, 160.0)));
    h.key_press(Key::Escape);
    h.run();
    assert_eq!(h.state().line, before);
    assert_eq!(phases(&h).last(), Some(&"cancel"));
}
