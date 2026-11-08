//! Puppet Warp: the session on a recording solver, the pins on a canvas and the options bar.

use craft_behavior::kurbo::Point;
use craft_behavior::{CancelReason, CanvasView, EditEvent, GestureId, Mixed};
use craft_imaging::ui::warp::pin::{
    Pin, PinDensity, PinEdit, PinMeshView, PinMode, PinSolver, PinWarp, PinWarpBar, PinWarpCommands, PinWarpOptions, PinWarpSession, PinWarpSettings,
};
use craft_ui::canvas::Canvas;
use craft_ui::contract::{Bind, Role, Show as _, ShowContainer as _};
use craft_ui::testing::{Fixture, pointer, trial};
use egui::{Key, Modifiers, Pos2, Vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

/// A room wide enough for the whole bar.
const WIDE_BAR: Vec2 = Vec2::new(1700.0, 80.0);
/// The expansion and the rotation the bar tests type.
const TYPED_EXPANSION: f64 = 12.0;
const TYPED_ROTATION: f64 = 45.0;

#[derive(Debug, PartialEq)]
enum Call {
    Rebind(Vec<Pin>),
    Solve(Vec<Pin>),
    Remesh(PinWarpOptions),
}

/// A solver that records its calls over a mesh of two triangles.
#[derive(Default)]
struct Fake {
    calls: Vec<Call>,
    refuse: bool,
    rest: Vec<Point>,
    deformed: Vec<Point>,
    triangles: Vec<[u32; 3]>,
    order: Vec<u32>,
}

impl PinSolver for Fake {
    fn rebind(&mut self, pins: &[Pin], _: &PinWarpOptions) {
        self.calls.push(Call::Rebind(pins.to_vec()));
    }
    fn solve(&mut self, pins: &[Pin], _: &PinWarpOptions) {
        self.calls.push(Call::Solve(pins.to_vec()));
    }
    fn remesh(&mut self, options: &PinWarpOptions) -> Result<(), String> {
        self.calls.push(Call::Remesh(*options));
        if self.refuse { Err("no pixels".into()) } else { Ok(()) }
    }
    fn mesh(&self) -> PinMeshView<'_> {
        PinMeshView::new(&self.rest, &self.deformed, &self.triangles, &self.order)
    }
}

impl Fake {
    /// A rectangle mesh of two triangles from (40, 40) to (360, 260), undeformed.
    fn flat() -> Self {
        let rest = vec![Point::new(40.0, 40.0), Point::new(360.0, 40.0), Point::new(360.0, 260.0), Point::new(40.0, 260.0)];
        Fake { deformed: rest.clone(), rest, triangles: vec![[0, 1, 2], [0, 2, 3]], order: vec![0, 1], ..Fake::default() }
    }
}

fn pin(x: f64, y: f64) -> Pin {
    Pin::new(Point::new(x, y), Point::new(x, y))
}

fn moved(mut pin: Pin, dx: f64) -> Pin {
    pin.at.x += dx;
    pin
}

fn turned(mut pin: Pin, degrees: f64) -> Pin {
    pin.angle = Some(degrees);
    pin
}

fn session() -> PinWarpSession {
    PinWarpSession::new(&PinWarpSettings::default())
}

fn label() -> Arc<str> {
    Arc::from("Pins")
}

/// A drag from the session's pins to `ends` as `Begin`, `Update` and `Commit`.
fn drag(s: &mut PinWarpSession, f: &mut Fake, ends: Vec<Pin>, id: u64) {
    let (id, before) = (GestureId::new(id), Mixed::Same(s.pins().to_vec()));
    s.apply(PinEdit::Pins(EditEvent::begin(id, before.clone(), label(), None)), f).unwrap();
    s.apply(PinEdit::Pins(EditEvent::update(id, ends.clone())), f).unwrap();
    s.apply(PinEdit::Pins(EditEvent::commit(id, before, ends)), f).unwrap();
}

fn set(s: &mut PinWarpSession, f: &mut Fake, pins: Vec<Pin>) {
    let edit = PinEdit::replace(s.pins(), pins, "Pins");
    s.apply(edit, f).unwrap();
}

#[trial]
fn a_click_on_the_mesh_places_a_pin_at_the_rest_position_under_it() {
    let rest = [Point::new(0.0, 0.0), Point::new(10.0, 0.0), Point::new(0.0, 10.0)];
    // The deformed triangle is the rest one shifted by (100, 0) and drawn twice: the later entry of the order is on
    // top and wins.
    let deformed = [Point::new(100.0, 0.0), Point::new(110.0, 0.0), Point::new(100.0, 10.0)];
    let rest2 = [Point::new(50.0, 50.0), Point::new(60.0, 50.0), Point::new(50.0, 60.0)];
    let rest_all = [rest, rest2].concat();
    let deformed_all = [deformed, deformed].concat();
    let triangles = [[0, 1, 2], [3, 4, 5]];
    let view = |order: &'static [u32]| PinMeshView::new(&rest_all, &deformed_all, &triangles, order);
    let under = Point::new(102.0, 3.0);
    let top_last = view(&[0, 1]).rest_of(under).unwrap();
    assert!((top_last.x - 52.0).abs() < 1e-9 && (top_last.y - 53.0).abs() < 1e-9);
    let top_first = view(&[1, 0]).rest_of(under).unwrap();
    assert!((top_first.x - 2.0).abs() < 1e-9 && (top_first.y - 3.0).abs() < 1e-9);
    assert_eq!(view(&[0, 1]).rest_of(Point::new(0.0, 0.0)), None, "off the mesh");
}

#[trial]
fn placing_a_pin_and_dragging_it_on_is_one_step() {
    let (mut s, mut f) = (session(), Fake::default());
    let placed = vec![pin(5.0, 5.0)];
    drag(&mut s, &mut f, vec![moved(placed[0], 20.0)], 1);
    assert!(matches!(f.calls[0], Call::Rebind(_)));
    assert!(f.calls[1..].iter().all(|c| matches!(c, Call::Solve(_))));
    assert_eq!(s.selected(), &[0], "the placed pin is selected");
    assert!(s.can_undo());
    assert!(s.undo(&mut f));
    assert!(s.pins().is_empty(), "placement and drag undo together");
    assert!(!s.can_undo());
}

#[trial]
fn dragging_a_pin_is_one_step_and_esc_puts_it_back() {
    let (mut s, mut f) = (session(), Fake::default());
    let start = vec![pin(5.0, 5.0)];
    set(&mut s, &mut f, start.clone());
    drag(&mut s, &mut f, vec![moved(start[0], 9.0)], 1);
    assert_eq!(s.pins()[0].at.x, 14.0);
    assert!(s.undo(&mut f));
    assert_eq!(s.pins(), &start[..]);

    let id = GestureId::new(2);
    s.apply(PinEdit::Pins(EditEvent::begin(id, Mixed::Same(start.clone()), label(), None)), &mut f).unwrap();
    s.apply(PinEdit::Pins(EditEvent::update(id, vec![moved(start[0], 30.0)])), &mut f).unwrap();
    assert!(s.dragging());
    assert!(!s.undo(&mut f), "undo waits for the drag");
    s.apply(PinEdit::Pins(EditEvent::cancel(id, CancelReason::Escape, Some(Mixed::Same(start.clone())))), &mut f).unwrap();
    assert!(!s.dragging());
    assert_eq!(s.pins(), &start[..]);
    assert!(s.can_redo(), "the cancelled drag recorded nothing, so the undone step is still there");
}

#[trial]
fn alt_click_deletes_a_pin_as_one_step() {
    let (mut s, mut f) = (session(), Fake::default());
    let pins = vec![pin(1.0, 1.0), pin(2.0, 2.0), pin(3.0, 3.0)];
    set(&mut s, &mut f, pins.clone());
    s.apply(PinEdit::Select(vec![0, 2, 9]), &mut f).unwrap();
    assert_eq!(s.selected(), &[0, 2], "out-of-range indices are dropped");
    f.calls.clear();
    set(&mut s, &mut f, vec![pins[0], pins[2]]);
    assert_eq!(f.calls.len(), 1);
    assert!(matches!(f.calls[0], Call::Rebind(_)));
    assert_eq!(s.selected(), &[0, 1]);
    set(&mut s, &mut f, vec![pins[2]]);
    assert_eq!(s.selected(), &[0], "pin 0 was deleted and pin 2 slid down");
    assert!(s.undo(&mut f));
    assert_eq!(s.pins().len(), 2);
}

#[trial]
fn a_refused_remesh_restores_density_and_expansion_and_reports_the_text() {
    let (mut s, mut f) = (session(), Fake { refuse: true, ..Fake::default() });
    let before = *s.options();
    let mut wanted = before;
    wanted.density = PinDensity::More;
    wanted.expansion = 40.0;
    assert_eq!(s.apply(PinEdit::Options(wanted), &mut f), Err("no pixels".to_string()));
    assert_eq!(*s.options(), before);

    f.refuse = false;
    f.calls.clear();
    let mut rigid = before;
    rigid.mode = PinMode::Rigid;
    assert_eq!(s.apply(PinEdit::Options(rigid), &mut f), Ok(()));
    assert!(matches!(f.calls.as_slice(), [Call::Solve(_)]), "a mode change needs no remesh");
    f.calls.clear();
    s.apply(PinEdit::Options(wanted), &mut f).unwrap();
    assert!(matches!(f.calls.as_slice(), [Call::Remesh(_), Call::Rebind(_)]));
    assert_eq!(s.options().density, PinDensity::More);
}

#[trial]
fn solving_starts_from_the_rest_mesh_whatever_the_pointer_history() {
    let base = vec![pin(5.0, 5.0)];
    let target = vec![moved(base[0], 12.0)];
    let last_solve = |path: &[f64]| {
        let (mut s, mut f) = (session(), Fake::default());
        set(&mut s, &mut f, base.clone());
        let (id, before) = (GestureId::new(1), Mixed::Same(base.clone()));
        s.apply(PinEdit::Pins(EditEvent::begin(id, before.clone(), label(), None)), &mut f).unwrap();
        for dx in path {
            s.apply(PinEdit::Pins(EditEvent::update(id, vec![moved(base[0], *dx)])), &mut f).unwrap();
        }
        s.apply(PinEdit::Pins(EditEvent::commit(id, before, target.clone())), &mut f).unwrap();
        f.calls.pop()
    };
    assert_eq!(last_solve(&[3.0, 40.0, -7.0]), last_solve(&[12.0]));
    assert_eq!(last_solve(&[12.0]), Some(Call::Solve(target.clone())));
}

#[trial]
fn undo_and_redo_restore_the_pins_and_keep_the_selection_when_it_still_exists() {
    let (mut s, mut f) = (session(), Fake::default());
    let two = vec![pin(1.0, 1.0), pin(2.0, 2.0)];
    set(&mut s, &mut f, vec![two[0]]);
    set(&mut s, &mut f, two.clone());
    s.apply(PinEdit::Select(vec![1]), &mut f).unwrap();
    f.calls.clear();
    assert!(s.undo(&mut f));
    assert_eq!(s.pins(), &two[..1]);
    assert!(s.selected().is_empty(), "pin 1 no longer exists");
    assert!(matches!(f.calls.as_slice(), [Call::Rebind(p)] if p.len() == 1));
    s.apply(PinEdit::Select(vec![0]), &mut f).unwrap();
    assert!(s.redo(&mut f));
    assert_eq!(s.pins(), &two[..]);
    assert_eq!(s.selected(), &[0]);
    assert!(!s.redo(&mut f));
}

#[trial]
fn an_untouched_session_has_no_result() {
    let (mut s, mut f) = (session(), Fake::default());
    assert!(s.result().is_none());
    set(&mut s, &mut f, vec![pin(1.0, 1.0)]);
    assert!(s.result().is_none(), "a pin where it attached");
    set(&mut s, &mut f, vec![turned(pin(1.0, 1.0), 360.0)]);
    assert!(s.result().is_none(), "a full turn is no turn");
    set(&mut s, &mut f, vec![turned(pin(1.0, 1.0), 30.0)]);
    assert_eq!(s.result().map(<[Pin]>::len), Some(1));
    set(&mut s, &mut f, vec![moved(pin(1.0, 1.0), 4.0)]);
    assert_eq!(s.result().map(<[Pin]>::len), Some(1));
}

#[trial]
fn edges_are_unique_and_the_centre_is_the_middle_of_the_rest_bounds() {
    let rest = [Point::new(0.0, 0.0), Point::new(10.0, 0.0), Point::new(0.0, 20.0), Point::new(10.0, 20.0)];
    let triangles = [[0, 1, 2], [2, 1, 3]];
    let view = PinMeshView::new(&rest, &rest, &triangles, &[0, 1]);
    assert_eq!(view.edges(), vec![[0, 1], [0, 2], [1, 2], [1, 3], [2, 3]]);
    assert_eq!(view.centre(), Some(Point::new(5.0, 10.0)));
    let empty = PinMeshView::new(&[], &[], &[], &[]);
    assert_eq!(empty.centre(), None);
}

#[trial]
fn turn_at_reads_the_rotation_of_the_triangle_under_a_rest_point() {
    let rest = vec![Point::new(0.0, 0.0), Point::new(100.0, 0.0), Point::new(0.0, 100.0)];
    let (sin, cos) = 25f64.to_radians().sin_cos();
    // Stretched 2 x along x, then rotated 25 degrees.
    let deformed: Vec<Point> = rest.iter().map(|p| Point::new(2.0 * p.x * cos - p.y * sin, 2.0 * p.x * sin + p.y * cos)).collect();
    let fake = Fake { rest, deformed, triangles: vec![[0, 1, 2]], order: vec![0], ..Fake::default() };
    let turn = fake.mesh().turn_at(Point::new(20.0, 20.0)).expect("on the mesh");
    assert!((turn - 25.0).abs() < 1e-9, "{turn}");
    assert!(fake.mesh().turn_at(Point::new(90.0, 90.0)).is_none());
}

/// What one pass reported about the pin edit.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Seen {
    Begin,
    Commit,
    Cancel,
    Set,
}

struct State {
    session: PinWarpSession,
    solver: Fake,
    seen: Vec<Seen>,
    edits: usize,
    apply: bool,
    discard: bool,
    pointer: Option<Point>,
}

struct Harness {
    fixture: Fixture,
    state: Rc<RefCell<State>>,
    view: Rc<Cell<Option<CanvasView>>>,
}

fn harness(pins: Vec<Pin>, selected: Vec<usize>) -> Harness {
    let mut solver = Fake::flat();
    let mut session = PinWarpSession::from_pins(pins, &PinWarpSettings::default());
    session.apply(PinEdit::Select(selected), &mut solver).expect("selects");
    let state = Rc::new(RefCell::new(State { session, solver, seen: Vec::new(), edits: 0, apply: false, discard: false, pointer: None }));
    let view = Rc::new(Cell::new(None));
    let (shown_state, shown_view) = (Rc::clone(&state), Rc::clone(&view));
    let mut fixture = Fixture::new(move |ui| {
        let room = ui.available_rect_before_wrap();
        let canvas_view = CanvasView::new(room, Point::new(f64::from(room.width()) / 2.0, f64::from(room.height()) / 2.0), 1.0);
        shown_view.set(Some(canvas_view));
        let output = {
            let held = shown_state.borrow();
            Canvas::new("Document")
                .view(canvas_view)
                .show(ui, |ui, body| {
                    PinWarp::new("Pins", Bind::given(held.session.pins().to_vec()), held.solver.mesh())
                        .selected_pins(held.session.selected())
                        .show_mesh(true)
                        .show_at(ui, body.rect)
                })
                .inner
        };
        let Some(output) = output else { return };
        let mut held = shown_state.borrow_mut();
        let State { session, solver, seen, edits, apply, discard, pointer } = &mut *held;
        for edit in output.edits() {
            if let PinEdit::Pins(event) = &edit {
                match event {
                    EditEvent::Begin { .. } => seen.push(Seen::Begin),
                    EditEvent::Commit { .. } => seen.push(Seen::Commit),
                    EditEvent::Cancel { .. } => seen.push(Seen::Cancel),
                    EditEvent::Set { .. } => seen.push(Seen::Set),
                    _ => {}
                }
            }
            *edits += 1;
            session.apply(edit, solver).expect("applies");
        }
        *apply |= output.apply;
        *discard |= output.discard;
        *pointer = output.pointer;
    });
    fixture.settle();
    Harness { fixture, state, view }
}

impl Harness {
    fn at(&self, x: f64, y: f64) -> Pos2 {
        self.view.get().expect("shown").to_screen(Point::new(x, y))
    }

    /// Presses at `from`, moves to `to` in two steps and releases, with `mods` held.
    fn drag(&mut self, from: (f64, f64), to: (f64, f64), mods: Modifiers) {
        let (a, b) = (self.at(from.0, from.1), self.at(to.0, to.1));
        self.fixture.hold(mods);
        self.fixture.frame(vec![pointer::moved(a)]);
        self.fixture.frame(vec![pointer::press(a)]);
        self.fixture.frame(vec![pointer::moved(a.lerp(b, 0.5))]);
        self.fixture.frame(vec![pointer::moved(b)]);
        self.fixture.frame(vec![pointer::release(b)]);
        self.fixture.frame(Vec::new());
        self.fixture.hold(Modifiers::NONE);
    }

    fn click(&mut self, at: (f64, f64), mods: Modifiers) {
        self.drag(at, at, mods);
    }

    fn key(&mut self, key: Key, mods: Modifiers) {
        self.fixture.key(key, mods);
        self.fixture.settle();
    }

    fn selected(&self) -> Vec<usize> {
        self.state.borrow().session.selected().to_vec()
    }

    fn places(&self) -> Vec<Point> {
        self.state.borrow().session.pins().iter().map(|p| p.at).collect()
    }

    fn seen(&self) -> Vec<Seen> {
        self.state.borrow().seen.clone()
    }
}

#[trial]
fn a_click_on_the_mesh_places_a_selected_pin_and_its_drag_moves_it() {
    let mut h = harness(vec![pin(100.0, 100.0)], vec![]);
    h.drag((200.0, 150.0), (230.0, 170.0), Modifiers::NONE);
    let pins = h.state.borrow().session.pins().to_vec();
    assert_eq!(pins.len(), 2);
    assert_eq!(pins[1].rest, Point::new(200.0, 150.0), "attached where it was placed");
    assert_eq!(pins[1].at, Point::new(230.0, 170.0), "and moved by the drag");
    assert_eq!(h.selected(), vec![1]);
    assert_eq!(h.seen(), [Seen::Begin, Seen::Commit], "placement and drag are one gesture");

    let edits = h.state.borrow().edits;
    let off = h.at(5.0, 5.0);
    h.fixture.click_at(off);
    assert_eq!(h.state.borrow().edits, edits, "a click off the mesh does nothing");
    assert_eq!(h.state.borrow().session.pins().len(), 2);
}

#[trial]
fn shift_click_toggles_the_selection_and_a_drag_moves_every_selected_pin() {
    let mut h = harness(vec![pin(100.0, 100.0), pin(200.0, 100.0), pin(300.0, 100.0)], vec![0]);
    h.click((200.0, 100.0), Modifiers::SHIFT);
    assert_eq!(h.selected(), vec![0, 1]);
    h.click((300.0, 100.0), Modifiers::SHIFT);
    h.click((300.0, 100.0), Modifiers::SHIFT);
    assert_eq!(h.selected(), vec![0, 1], "the second shift-click takes it out");

    h.state.borrow_mut().seen.clear();
    h.drag((100.0, 100.0), (130.0, 120.0), Modifiers::NONE);
    assert_eq!(h.places(), vec![Point::new(130.0, 120.0), Point::new(230.0, 120.0), Point::new(300.0, 100.0)]);
    assert_eq!(h.seen(), [Seen::Begin, Seen::Commit], "one gesture for both pins");
    assert_eq!(h.selected(), vec![0, 1]);

    h.click((300.0, 100.0), Modifiers::NONE);
    assert_eq!(h.selected(), vec![2], "a plain press on an unselected pin selects only it");
}

/// The radius of the ring a pin's fixed rotation is set on, from the tokens.
fn turn_ring() -> f64 {
    f64::from(craft_ui::testing::style::tokens(craft_theme::Mode::Light, craft_theme::Contrast::Standard).size.handle.ring)
}

/// How far inside or outside the turn ring a press is placed.
const RING_MARGIN: f64 = 1.0;

#[trial]
fn alt_drag_near_a_selected_pin_turns_it_and_fixes_its_rotation() {
    let ring = turn_ring();
    let (centre, quarter_turn) = (Point::new(200.0, 150.0), 90.0);
    let mut h = harness(vec![pin(centre.x, centre.y)], vec![0]);
    h.drag((centre.x + ring - RING_MARGIN, centre.y), (centre.x, 190.0), Modifiers::ALT);
    let turned = h.state.borrow().session.pins()[0];
    assert!((turned.angle.expect("fixed") - quarter_turn).abs() < 1e-9, "{:?}", turned.angle);
    assert_eq!(turned.at, centre, "the pin stays where it is");
    assert_eq!(h.seen(), [Seen::Begin, Seen::Commit]);

    h.state.borrow_mut().seen.clear();
    h.drag((centre.x + ring + RING_MARGIN, centre.y), (centre.x, 190.0), Modifiers::ALT);
    assert!(h.seen().is_empty(), "just outside the turn ring the press turns nothing: {:?}", h.seen());

    h.drag((300.0, 150.0), (300.0, 190.0), Modifiers::ALT);
    assert!(h.seen().is_empty(), "far from the pin the press turns nothing: {:?}", h.seen());
}

#[trial]
fn tab_onto_a_pin_selects_it() {
    let mut h = harness(vec![pin(100.0, 100.0), pin(200.0, 100.0)], vec![]);
    h.key(Key::Tab, Modifiers::NONE);
    assert_eq!(h.selected(), vec![0]);
    h.key(Key::Tab, Modifiers::NONE);
    assert_eq!(h.selected(), vec![1], "the pin Tab reaches is the selection");
}

#[trial]
fn alt_click_deletes_a_pin_and_esc_cancels_a_drag_or_discards() {
    let mut h = harness(vec![pin(100.0, 100.0), pin(200.0, 100.0)], vec![]);
    h.click((100.0, 100.0), Modifiers::ALT);
    assert_eq!(h.places(), vec![Point::new(200.0, 100.0)]);
    assert_eq!(h.seen(), [Seen::Set]);

    let from = h.at(200.0, 100.0);
    h.fixture.frame(vec![pointer::moved(from)]);
    h.fixture.frame(vec![pointer::press(from)]);
    h.fixture.frame(vec![pointer::moved(h.at(220.0, 100.0))]);
    h.fixture.frame(vec![pointer::moved(h.at(240.0, 100.0))]);
    h.fixture.settle();
    assert_eq!(h.places(), vec![Point::new(240.0, 100.0)]);
    h.key(Key::Escape, Modifiers::NONE);
    assert_eq!(h.places(), vec![Point::new(200.0, 100.0)], "cancelled");
    assert!(h.seen().contains(&Seen::Cancel));
    assert!(!h.state.borrow().discard, "the Esc ended the drag");
    h.fixture.frame(vec![pointer::release(h.at(240.0, 100.0))]);
    h.fixture.settle();
    h.key(Key::Escape, Modifiers::NONE);
    assert!(h.state.borrow().discard, "Esc with no drag discards the session");
}

#[trial]
fn delete_removes_the_selected_pins_and_select_all_selects_every_pin() {
    let mut h = harness(vec![pin(100.0, 100.0), pin(200.0, 100.0), pin(300.0, 100.0)], vec![]);
    h.click((100.0, 100.0), Modifiers::NONE);
    h.key(Key::A, Modifiers::COMMAND);
    assert_eq!(h.selected(), vec![0, 1, 2]);
    h.key(Key::Delete, Modifiers::NONE);
    assert!(h.state.borrow().session.pins().is_empty());

    let mut h = harness(vec![pin(100.0, 100.0), pin(200.0, 100.0)], vec![]);
    h.click((200.0, 100.0), Modifiers::NONE);
    h.key(Key::Enter, Modifiers::NONE);
    assert!(h.state.borrow().apply, "Enter applies");
    h.key(Key::Backspace, Modifiers::NONE);
    assert_eq!(h.places(), vec![Point::new(100.0, 100.0)]);
}

#[trial]
fn the_keyboard_places_picks_moves_and_removes_pins() {
    let mut h = harness(vec![pin(100.0, 100.0), pin(200.0, 100.0)], vec![]);
    h.click((100.0, 100.0), Modifiers::NONE);
    h.key(Key::PageDown, Modifiers::NONE);
    assert_eq!(h.selected(), vec![1]);
    h.key(Key::ArrowRight, Modifiers::NONE);
    h.key(Key::ArrowDown, Modifiers::SHIFT);
    assert_eq!(h.places(), vec![Point::new(100.0, 100.0), Point::new(201.0, 110.0)]);

    let add = |h: &Harness| {
        let mut held = h.state.borrow_mut();
        let State { session, solver, pointer, .. } = &mut *held;
        let (at, mesh) = (*pointer, solver.mesh());
        let rest_mesh = PinMeshView::new(mesh.rest, mesh.deformed, mesh.triangles, mesh.order);
        PinWarpCommands::AddPin.apply(session, &mut Fake::default(), rest_mesh, at).expect("applies");
    };
    // The pointer is on the mesh: the pin goes under it.
    h.fixture.frame(vec![pointer::moved(h.at(50.0, 60.0))]);
    h.fixture.settle();
    add(&h);
    assert_eq!(h.places()[2], Point::new(50.0, 60.0));
    assert_eq!(h.state.borrow().session.pins()[2].rest, Point::new(50.0, 60.0));
    assert_eq!(h.selected(), vec![2]);

    // The pointer is off the canvas: the mesh centre.
    h.fixture.frame(vec![pointer::gone()]);
    h.fixture.settle();
    add(&h);
    assert_eq!(h.places()[3], Point::new(200.0, 150.0));
}

#[trial]
fn alt_drag_turns_a_fixed_pin_by_the_swept_angle() {
    let mut h = harness(vec![turned(pin(200.0, 150.0), 30.0)], vec![0]);
    let near = turn_ring() - RING_MARGIN;
    h.drag((200.0, 150.0 + near), (200.0 - near, 150.0), Modifiers::ALT);
    let angle = h.state.borrow().session.pins()[0].angle.expect("fixed");
    assert!((angle - 120.0).abs() < 1e-9, "{angle}");
}

#[trial]
fn alt_drag_turns_an_auto_pin_from_the_meshs_turn() {
    let mut h = harness(vec![pin(200.0, 150.0)], vec![0]);
    let (sin, cos) = 20f64.to_radians().sin_cos();
    let centre = Point::new(200.0, 150.0);
    {
        let mut held = h.state.borrow_mut();
        held.solver.deformed = held
            .solver
            .rest
            .iter()
            .map(|p| {
                let (x, y) = (p.x - centre.x, p.y - centre.y);
                Point::new(centre.x + x * cos - y * sin, centre.y + x * sin + y * cos)
            })
            .collect();
    }
    h.fixture.settle();
    let near = turn_ring() - RING_MARGIN;
    h.drag((200.0, 150.0 + near), (200.0 - near, 150.0), Modifiers::ALT);
    let angle = h.state.borrow().session.pins()[0].angle.expect("fixed");
    assert!((angle - 110.0).abs() < 1e-9, "{angle}");
}

#[trial]
fn page_keys_cycle_through_every_pin() {
    let mut h = harness(vec![pin(100.0, 100.0), pin(200.0, 100.0)], vec![]);
    h.click((100.0, 100.0), Modifiers::NONE);
    let mut picks = Vec::new();
    for key in [Key::PageDown, Key::PageDown, Key::PageDown, Key::PageUp] {
        h.key(key, Modifiers::NONE);
        picks.push(h.selected());
    }
    assert_eq!(picks, [vec![1], vec![0], vec![1], vec![0]], "the pins wrap");
}

#[trial]
fn insert_places_no_pin_where_the_mesh_has_no_rest_position() {
    let mut s = PinWarpSession::new(&PinWarpSettings::default());
    let mut f = Fake::flat();
    f.triangles.clear();
    let mesh = PinMeshView::new(&f.rest, &f.deformed, &f.triangles, &f.order);
    PinWarpCommands::AddPin.apply(&mut s, &mut Fake::default(), mesh, None).expect("applies");
    assert!(s.pins().is_empty(), "not even at the mesh's centre");
}

struct BarState {
    session: PinWarpSession,
    notice: Option<String>,
    edits: Vec<PinEdit>,
    apply: bool,
    discard: bool,
}

fn bar(selected: bool, notice: Option<&str>) -> (Fixture, Rc<RefCell<BarState>>) {
    let mut solver = Fake::default();
    let mut session = PinWarpSession::from_pins(vec![pin(10.0, 0.0), pin(20.0, 0.0)], &PinWarpSettings::default());
    if selected {
        session.apply(PinEdit::Select(vec![1]), &mut solver).expect("selects");
    }
    let state = Rc::new(RefCell::new(BarState { session, notice: notice.map(str::to_owned), edits: Vec::new(), apply: false, discard: false }));
    let shown = Rc::clone(&state);
    let mut fixture = Fixture::new(move |ui| {
        let mut held = shown.borrow_mut();
        let output = PinWarpBar::new(&held.session).notice(held.notice.as_deref()).show(ui);
        held.apply |= output.apply;
        held.discard |= output.discard;
        held.edits.extend(output.edits);
    });
    fixture.room(WIDE_BAR);
    fixture.settle();
    (fixture, state)
}

fn depths(edit: &PinEdit) -> Vec<i32> {
    match edit {
        PinEdit::Pins(event) => event.value().map_or_else(Vec::new, |pins| pins.iter().map(|p| p.depth).collect()),
        _ => Vec::new(),
    }
}

#[trial]
fn pins_depth_buttons_change_the_selected_pins_depth() {
    let (fixture, _) = bar(false, None);
    assert!(fixture.by_label("Bring the selected pin forward").is_none(), "the depth buttons show with a selected pin");

    let (mut fixture, state) = bar(true, None);
    let forward = fixture.by_label("Bring the selected pin forward").expect("shown");
    fixture.click(&forward);
    let backward = fixture.by_label("Send the selected pin backward").expect("shown");
    fixture.click(&backward);
    let reported: Vec<Vec<i32>> = state.borrow().edits.iter().map(depths).collect();
    assert_eq!(reported, vec![vec![0, 1], vec![0, -1]]);
}

#[trial]
fn the_options_report_edits_and_a_refused_remesh_shows_its_notice() {
    let (mut fixture, state) = bar(false, Some("Could not use Puppet Warp"));
    assert!(fixture.by_label("Could not use Puppet Warp").is_some(), "the notice shows its text");
    let combos = fixture.by_role(Role::COMBO_BOX);
    fixture.click(&combos[0]);
    let rigid = fixture.by_label("Rigid").expect("the mode list is open");
    fixture.click(&rigid);
    let combos = fixture.by_role(Role::COMBO_BOX);
    fixture.click(&combos[1]);
    let more = fixture.by_label("More Points").expect("the density list is open");
    fixture.click(&more);
    let field = fixture.by_role(Role::SPIN_BUTTON).into_iter().next().expect("the expansion field");
    fixture.click(&field);
    fixture.key(Key::A, Modifiers::COMMAND);
    fixture.type_text(&TYPED_EXPANSION.to_string());
    fixture.key(Key::Enter, Modifiers::NONE);
    let mesh = fixture.by_label("Show Mesh").expect("shown");
    fixture.click(&mesh);
    let edits = state.borrow().edits.clone();
    assert!(matches!(&edits[0], PinEdit::Options(o) if o.mode == PinMode::Rigid), "{edits:?}");
    assert!(matches!(&edits[1], PinEdit::Options(o) if o.density == PinDensity::More), "{edits:?}");
    assert!(matches!(&edits[2], PinEdit::Options(o) if o.expansion == TYPED_EXPANSION), "{edits:?}");
    assert_eq!(edits.len(), 4, "{edits:?}");
    assert_eq!(edits[3], PinEdit::ShowMesh(false));
}

#[trial]
fn rotation_and_remove_all_report_pin_lists() {
    let (mut fixture, state) = bar(true, None);
    let combos = fixture.by_role(Role::COMBO_BOX);
    fixture.click(&combos[2]);
    let fixed = fixture.by_label("Fixed").expect("the rotation list is open");
    fixture.click(&fixed);
    let angle = |edit: &PinEdit| match edit {
        PinEdit::Pins(event) => event.value().and_then(|pins| pins[1].angle),
        _ => None,
    };
    assert_eq!(angle(&state.borrow().edits[0]), Some(0.0));

    // The app applies the edit; the degrees field then shows.
    let edits = std::mem::take(&mut state.borrow_mut().edits);
    for edit in edits {
        let mut held = state.borrow_mut();
        held.session.apply(edit, &mut Fake::default()).expect("applies");
    }
    fixture.settle();
    let degrees = fixture.by_role(Role::SPIN_BUTTON).into_iter().last().expect("the rotation field");
    fixture.click(&degrees);
    fixture.key(Key::A, Modifiers::COMMAND);
    fixture.type_text(&TYPED_ROTATION.to_string());
    fixture.key(Key::Enter, Modifiers::NONE);
    let committed = state.borrow().edits.iter().rev().find_map(angle);
    assert_eq!(committed, Some(TYPED_ROTATION), "{:?}", state.borrow().edits);

    state.borrow_mut().edits.clear();
    let remove = fixture.by_label("Remove All Pins").expect("shown");
    fixture.click(&remove);
    assert!(matches!(&state.borrow().edits[..], [PinEdit::Pins(EditEvent::Set { after, .. })] if after.is_empty()), "{:?}", state.borrow().edits);

    let commit = fixture.by_label("Commit Puppet Warp").expect("shown");
    fixture.click(&commit);
    assert!(state.borrow().apply);
    let cancel = fixture.by_label("Cancel Puppet Warp").expect("shown");
    fixture.click(&cancel);
    assert!(state.borrow().discard);
}
