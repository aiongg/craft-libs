//! Perspective Warp: the session and plane geometry, the planes on a canvas and the options bar.

use craft_behavior::kurbo::{Point, Rect};
use craft_behavior::{CanvasView, EditEvent, GestureId, Mixed};
use craft_imaging::ui::warp::perspective::{
    PerspectiveEdit, PerspectiveSession, PerspectiveWarp, Plane, PlaneCorner, PlaneMode, Straighten, linked_corners, straighten, unify,
};
use craft_ui::canvas::Canvas;
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::testing::{Fixture, pointer, trial};
use egui::Pos2;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

fn quad(x0: f64, y0: f64, x1: f64, y1: f64) -> [Point; 4] {
    [Point::new(x0, y0), Point::new(x1, y0), Point::new(x1, y1), Point::new(x0, y1)]
}

fn square(x0: f64, y0: f64, x1: f64, y1: f64) -> Plane {
    Plane::identity(quad(x0, y0, x1, y1))
}

/// Two planes side by side sharing the corners at x = 50.
fn two_planes() -> Vec<Plane> {
    vec![square(0.0, 0.0, 50.0, 100.0), square(50.0, 0.0, 100.0, 100.0)]
}

fn session() -> PerspectiveSession {
    PerspectiveSession::new(Rect::new(0.0, 0.0, 100.0, 100.0))
}

fn set(s: &mut PerspectiveSession, planes: Vec<Plane>) {
    let edit = PerspectiveEdit::replace(s.planes(), planes, "Planes");
    s.apply(edit);
}

fn drag(s: &mut PerspectiveSession, planes: Vec<Plane>) {
    let (id, before) = (GestureId::new(1), Mixed::Same(s.planes().to_vec()));
    s.apply(PerspectiveEdit::Planes(EditEvent::begin(id, before.clone(), Arc::from("Planes"), None)));
    s.apply(PerspectiveEdit::Planes(EditEvent::update(id, planes.clone())));
    s.apply(PerspectiveEdit::Planes(EditEvent::commit(id, before, planes)));
}

#[trial]
fn entering_warp_unifies_linked_corners() {
    let mut planes = two_planes();
    planes[1].source[0] = Point::new(50.2, 0.0);
    planes[1].target[0] = Point::new(51.0, 2.0);
    planes[0].target[1] = Point::new(49.0, 0.0);
    let mut copy = planes.clone();
    unify(&mut copy);
    assert_eq!(copy[0].target[1], Point::new(50.0, 1.0));
    let mut s = session();
    set(&mut s, planes);
    s.apply(PerspectiveEdit::Mode(PlaneMode::Warp));
    assert_eq!(s.mode(), PlaneMode::Warp);
    assert_eq!(s.planes()[0].source[1], Point::new(50.1, 0.0));
    assert_eq!(s.planes()[1].source[0], Point::new(50.1, 0.0));
    assert_eq!(s.planes()[0].target[1], Point::new(50.0, 1.0));
    assert_eq!(s.planes()[1].target[0], Point::new(50.0, 1.0));
}

#[trial]
fn straighten_level_and_both_move_the_corners() {
    let skewed = || {
        let mut planes = two_planes();
        planes[0].target = [Point::new(0.0, 0.0), Point::new(48.0, 4.0), Point::new(52.0, 100.0), Point::new(2.0, 96.0)];
        planes[1].target[0] = planes[0].target[1];
        planes[1].target[3] = planes[0].target[2];
        planes
    };
    let mut vertical = skewed();
    straighten(&mut vertical, Straighten::Vertical);
    assert_eq!(vertical[0].target[1].x, vertical[0].target[2].x);
    assert_eq!(vertical[0].target[1].y, 4.0);
    assert_eq!(vertical[1].target[0], vertical[0].target[1]);
    let one = || {
        let mut plane = square(0.0, 0.0, 50.0, 100.0);
        plane.target = [Point::new(0.0, 0.0), Point::new(48.0, 4.0), Point::new(52.0, 100.0), Point::new(2.0, 96.0)];
        vec![plane]
    };
    let mut horizontal = one();
    straighten(&mut horizontal, Straighten::Horizontal);
    assert_eq!(horizontal[0].target[0].y, horizontal[0].target[1].y);
    assert_eq!(horizontal[0].target[2].y, horizontal[0].target[3].y);
    assert_eq!(horizontal[0].target[1].x, 48.0);
    let mut both = one();
    straighten(&mut both, Straighten::Both);
    assert_eq!(both[0].target[0].y, both[0].target[1].y);
    assert_eq!(both[0].target[1].x, both[0].target[2].x);
    assert_eq!(both[0].target[0].x, both[0].target[3].x);
}

#[trial]
fn plane_edits_are_undo_steps() {
    let mut s = session();
    set(&mut s, two_planes());
    s.apply(PerspectiveEdit::Mode(PlaneMode::Warp));
    let mut moved = s.planes().to_vec();
    moved[0].target[0] = Point::new(5.0, 5.0);
    drag(&mut s, moved);
    assert!(s.undo());
    assert_eq!(s.planes()[0].target[0], Point::new(0.0, 0.0));
    assert_eq!(s.mode(), PlaneMode::Warp);
    assert!(s.undo());
    assert_eq!(s.mode(), PlaneMode::Layout);
    assert_eq!(s.planes().len(), 2);
    assert!(s.undo());
    assert!(s.planes().is_empty());
    assert!(!s.can_undo());
    assert!(s.redo());
    assert_eq!(s.planes().len(), 2);
}

/// What one pass reported about the planes edit.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Seen {
    Begin,
    Commit,
    Cancel,
    Set,
}

struct State {
    session: PerspectiveSession,
    seen: Vec<Seen>,
    apply: bool,
}

struct Harness {
    fixture: Fixture,
    state: Rc<RefCell<State>>,
    view: Rc<Cell<Option<CanvasView>>>,
}

fn harness(planes: Vec<Plane>, mode: PlaneMode) -> Harness {
    let mut session = PerspectiveSession::new(Rect::new(0.0, 0.0, 400.0, 300.0));
    if !planes.is_empty() {
        let edit = PerspectiveEdit::replace(&[], planes, "Planes");
        session.apply(edit);
    }
    session.apply(PerspectiveEdit::Mode(mode));
    let state = Rc::new(RefCell::new(State { session, seen: Vec::new(), apply: false }));
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
                    PerspectiveWarp::new("Planes", Bind::given(held.session.planes().to_vec()), held.session.mode(), held.session.generation())
                        .show_at(ui, body.rect)
                })
                .inner
        };
        let Some(output) = output else { return };
        let mut held = shown_state.borrow_mut();
        for edit in output.edits() {
            if let PerspectiveEdit::Planes(event) = &edit {
                match event {
                    EditEvent::Begin { .. } => held.seen.push(Seen::Begin),
                    EditEvent::Commit { .. } => held.seen.push(Seen::Commit),
                    EditEvent::Cancel { .. } => held.seen.push(Seen::Cancel),
                    EditEvent::Set { .. } => held.seen.push(Seen::Set),
                    _ => {}
                }
            }
            held.session.apply(edit);
        }
        held.apply |= output.apply;
    });
    fixture.settle();
    Harness { fixture, state, view }
}

impl Harness {
    fn at(&self, x: f64, y: f64) -> Pos2 {
        self.view.get().expect("shown").to_screen(Point::new(x, y))
    }

    /// Presses at `from`, moves to `to` in two steps and releases.
    fn drag(&mut self, from: (f64, f64), to: (f64, f64)) {
        let (a, b) = (self.at(from.0, from.1), self.at(to.0, to.1));
        self.fixture.frame(vec![pointer::moved(a)]);
        self.fixture.frame(vec![pointer::press(a)]);
        self.fixture.frame(vec![pointer::moved(a.lerp(b, 0.5))]);
        self.fixture.frame(vec![pointer::moved(b)]);
        self.fixture.frame(vec![pointer::release(b)]);
        self.fixture.frame(Vec::new());
    }

    fn planes(&self) -> Vec<Plane> {
        self.state.borrow().session.planes().to_vec()
    }
}

#[trial]
fn a_drag_in_layout_draws_a_plane_and_a_tiny_drag_adds_none() {
    let mut h = harness(vec![], PlaneMode::Layout);
    h.drag((50.0, 50.0), (60.0, 60.0));
    assert_eq!(h.planes(), vec![square(50.0, 50.0, 60.0, 60.0)]);

    h.drag((150.0, 150.0), (152.0, 153.0));
    assert_eq!(h.planes().len(), 1, "3 units is under the minimum");
    h.drag((150.0, 150.0), (160.0, 152.0));
    assert_eq!(h.planes().len(), 1, "so is 2 in one direction");
}

#[trial]
fn a_corner_dropped_near_another_planes_corner_snaps_and_links() {
    let mut h = harness(vec![square(100.0, 100.0, 200.0, 200.0), square(220.0, 100.0, 300.0, 200.0)], PlaneMode::Layout);
    h.drag((220.0, 100.0), (205.0, 102.0));
    let moved = h.planes();
    assert_eq!(moved[1].source[0], Point::new(200.0, 100.0), "snapped to the first plane's corner");
    assert_eq!(moved[1].target[0], moved[1].source[0], "Layout moves source and target together");
    let group = linked_corners(&moved).into_iter().find(|g| g.len() > 1).expect("linked");
    assert_eq!(group, vec![PlaneCorner::new(0, 1), PlaneCorner::new(1, 0)]);

    // Dragged on, the linked pair moves together.
    h.drag((200.0, 100.0), (200.0, 60.0));
    let moved = h.planes();
    assert_eq!((moved[0].source[1], moved[1].source[0]), (Point::new(200.0, 60.0), Point::new(200.0, 60.0)));
}

#[trial]
fn a_warp_drag_moves_the_target_of_linked_corners_and_keeps_the_layout() {
    let mut h = harness(vec![square(100.0, 100.0, 200.0, 200.0), square(200.0, 100.0, 300.0, 200.0)], PlaneMode::Warp);
    h.drag((200.0, 100.0), (210.0, 80.0));
    let moved = h.planes();
    assert_eq!((moved[0].target[1], moved[1].target[0]), (Point::new(210.0, 80.0), Point::new(210.0, 80.0)));
    assert_eq!((moved[0].source[1], moved[1].source[0]), (Point::new(200.0, 100.0), Point::new(200.0, 100.0)));

    // A press away from a corner draws nothing in Warp.
    let before = h.state.borrow().seen.len();
    h.drag((20.0, 20.0), (60.0, 60.0));
    assert_eq!(h.state.borrow().seen.len(), before);
    assert_eq!(h.planes().len(), 2);
}

#[trial]
fn replacing_the_planes_in_the_middle_of_a_corner_drag_ends_the_drag() {
    let mut h = harness(vec![square(100.0, 100.0, 200.0, 200.0)], PlaneMode::Layout);
    let from = h.at(100.0, 100.0);
    h.fixture.frame(vec![pointer::moved(from)]);
    h.fixture.frame(vec![pointer::press(from)]);
    h.fixture.frame(vec![pointer::moved(h.at(110.0, 110.0))]);
    h.fixture.frame(vec![pointer::moved(h.at(120.0, 120.0))]);
    h.fixture.settle();
    assert!(h.state.borrow().session.dragging());
    assert_eq!(h.planes()[0].source[0], Point::new(120.0, 120.0));

    h.state.borrow_mut().session.replace(vec![]);
    h.fixture.settle();
    assert!(!h.state.borrow().session.dragging());
    assert!(h.state.borrow().seen.contains(&Seen::Cancel), "the drag ends with a cancel");

    let before = h.state.borrow().seen.len();
    let away = h.at(140.0, 140.0);
    h.fixture.frame(vec![pointer::moved(away)]);
    h.fixture.frame(vec![pointer::release(away)]);
    h.fixture.settle();
    assert_eq!(h.state.borrow().seen.len(), before, "nothing after the replacement");
    assert!(h.planes().is_empty());
}
