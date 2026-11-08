//! Perspective Warp: the session and plane geometry, the planes on a canvas and the options bar.

use craft_behavior::kurbo::{Point, Rect};
use craft_behavior::{CanvasView, EditEvent, GestureId, Mixed};
use craft_imaging::ui::warp::perspective::{
    LINK_DISTANCE, PerspectiveBar, PerspectiveCommands, PerspectiveEdit, PerspectiveSession, PerspectiveWarp, Plane, PlaneCorner, PlaneMode,
    Straighten, linked_corners, straighten, unify,
};
use craft_ui::canvas::Canvas;
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::testing::{Fixture, pointer, trial};
use egui::{Key, Modifiers, Pos2, Vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

/// A room wide enough for the whole bar.
const WIDE_BAR: Vec2 = Vec2::new(1200.0, 80.0);

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
fn corners_within_the_link_distance_are_linked() {
    let mut planes = two_planes();
    planes[1].source[0].x += LINK_DISTANCE * 0.9;
    let groups = linked_corners(&planes);
    assert_eq!(groups.len(), 6);
    let shared = groups.iter().find(|g| g.contains(&PlaneCorner::new(0, 1))).unwrap();
    assert_eq!(shared, &vec![PlaneCorner::new(0, 1), PlaneCorner::new(1, 0)]);
    planes[1].source[0].x += 1.0;
    assert_eq!(linked_corners(&planes).len(), 7);
}

#[trial]
fn linked_corners_move_together_in_warp_and_layout_is_kept() {
    let mut s = session();
    set(&mut s, two_planes());
    s.apply(PerspectiveEdit::Mode(PlaneMode::Warp));
    let mut moved = s.planes().to_vec();
    for c in [(0, 1), (1, 0)] {
        moved[c.0].target[c.1] = Point::new(55.0, 5.0);
    }
    // The overlay's edit may carry stale sources; Warp keeps the session's.
    moved[0].source[1] = Point::new(1.0, 1.0);
    drag(&mut s, moved);
    assert_eq!(s.planes()[0].target[1], Point::new(55.0, 5.0));
    assert_eq!(s.planes()[1].target[0], Point::new(55.0, 5.0));
    assert_eq!(s.planes()[0].source, quad(0.0, 0.0, 50.0, 100.0));
    assert_eq!(s.planes()[1].source, quad(50.0, 0.0, 100.0, 100.0));
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
fn warp_needs_at_least_one_plane() {
    let mut s = session();
    s.apply(PerspectiveEdit::Mode(PlaneMode::Warp));
    assert_eq!(s.mode(), PlaneMode::Layout);
    assert!(!s.can_undo());
}

#[trial]
fn replacing_the_planes_bumps_the_generation_and_identity_gives_no_result() {
    let mut s = session();
    assert_eq!(s.result(), None);
    let inserted = s.inserted_plane();
    set(&mut s, vec![inserted]);
    assert_eq!(s.inserted_plane().source, quad(25.0, 25.0, 75.0, 75.0));
    assert_eq!(s.result(), None);
    let (id, before) = (GestureId::new(1), Mixed::Same(s.planes().to_vec()));
    s.apply(PerspectiveEdit::Planes(EditEvent::begin(id, before, Arc::from("Planes"), None)));
    assert!(s.dragging());
    let generation = s.generation();
    let mut planes = two_planes();
    planes[0].target[0].x = 3.0;
    s.replace(planes);
    assert!(!s.dragging());
    assert_eq!(s.generation(), generation + 1);
    assert_eq!(s.planes().len(), 2);
    assert_eq!(s.result().map(<[Plane]>::len), Some(2));
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

    fn key(&mut self, key: Key, mods: Modifiers) {
        self.fixture.key(key, mods);
        self.fixture.settle();
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

#[trial]
fn the_keyboard_adds_a_plane_picks_corners_and_removes_a_plane() {
    let mut h = harness(vec![square(100.0, 100.0, 200.0, 200.0)], PlaneMode::Layout);
    h.drag((100.0, 100.0), (100.0, 100.0));
    h.key(Key::PageDown, Modifiers::NONE);
    h.key(Key::ArrowRight, Modifiers::NONE);
    h.key(Key::ArrowDown, Modifiers::SHIFT);
    let moved = h.planes()[0];
    assert_eq!(moved.source[1], Point::new(201.0, 110.0));
    assert_eq!(moved.target[1], moved.source[1]);
    assert_eq!(moved.source[0], Point::new(100.0, 100.0), "only the picked corner");

    PerspectiveCommands::AddPlane.apply(&mut h.state.borrow_mut().session);
    h.fixture.settle();
    assert_eq!(h.planes().len(), 2);
    assert_eq!(h.planes()[1], h.state.borrow().session.inserted_plane());
    assert_eq!(h.planes()[1], square(100.0, 75.0, 300.0, 225.0));

    h.key(Key::Delete, Modifiers::NONE);
    assert_eq!(h.planes(), vec![square(100.0, 75.0, 300.0, 225.0)], "the picked corner's plane is gone");

    PerspectiveCommands::AddPlane.apply(&mut h.state.borrow_mut().session);
    h.fixture.settle();
    assert_eq!(h.planes().len(), 2);
    h.key(Key::Backspace, Modifiers::NONE);
    assert_eq!(h.planes().len(), 1, "Backspace removes the picked corner's plane as Delete does");
    h.key(Key::Enter, Modifiers::NONE);
    assert!(h.state.borrow().apply, "Enter applies");
}

struct BarState {
    session: PerspectiveSession,
    edits: Vec<PerspectiveEdit>,
    apply: bool,
    discard: bool,
}

fn bar(count: usize, mode: PlaneMode) -> (Fixture, Rc<RefCell<BarState>>) {
    let mut session = PerspectiveSession::new(Rect::new(0.0, 0.0, 100.0, 100.0));
    if count > 0 {
        let planes = (0..count).map(|i| square(20.0 * i as f64, 0.0, 20.0 * i as f64 + 10.0, 10.0)).collect();
        session.apply(PerspectiveEdit::replace(&[], planes, "Planes"));
    }
    session.apply(PerspectiveEdit::Mode(mode));
    let state = Rc::new(RefCell::new(BarState { session, edits: Vec::new(), apply: false, discard: false }));
    let shown = Rc::clone(&state);
    let mut fixture = Fixture::new(move |ui| {
        let mut held = shown.borrow_mut();
        let output = PerspectiveBar::new(&held.session).show(ui);
        held.apply |= output.apply;
        held.discard |= output.discard;
        held.edits.extend(output.edits);
    });
    fixture.room(WIDE_BAR);
    fixture.settle();
    (fixture, state)
}

#[trial]
fn the_bar_shows_each_modes_controls() {
    let (layout, _) = bar(2, PlaneMode::Layout);
    assert!(layout.by_label("2 planes").is_some(), "the count reads through the count word");
    assert!(layout.by_label("Remove Planes").is_some());
    assert!(layout.by_label("Straighten").is_none());

    let (mut warp, state) = bar(2, PlaneMode::Warp);
    for name in ["Straighten", "Level", "Both"] {
        assert!(warp.by_label(name).is_some(), "{name}");
    }
    assert!(warp.by_label("Remove Planes").is_none());
    let level = warp.by_label("Level").expect("shown");
    warp.click(&level);
    assert_eq!(state.borrow().edits, vec![PerspectiveEdit::Straighten(Straighten::Horizontal)]);

    let (mut empty, state) = bar(0, PlaneMode::Layout);
    assert!(empty.by_label("0 planes").is_some());
    let tab = empty.by_label("Warp").expect("the Warp tab shows");
    empty.click(&tab);
    assert!(state.borrow().edits.is_empty(), "the Warp tab is disabled with no plane");
}

#[trial]
fn commit_and_cancel_report_through_the_output() {
    let (mut fixture, state) = bar(1, PlaneMode::Layout);
    let commit = fixture.by_label("Commit Perspective Warp").expect("shown");
    fixture.click(&commit);
    assert!(state.borrow().apply && !state.borrow().discard);
    let cancel = fixture.by_label("Cancel Perspective Warp").expect("shown");
    fixture.click(&cancel);
    assert!(state.borrow().discard);
}
