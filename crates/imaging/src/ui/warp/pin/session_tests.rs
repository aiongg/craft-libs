//! Tests of the pin session, on a fake solver that records its calls.

use egui::Modifiers;
use kurbo::Point;

use super::*;
use craft_ui::edit::{CancelReason, EditEvent, GestureId};

#[derive(Debug, PartialEq)]
enum Call {
    Rebind(Vec<Pin>),
    Solve(Vec<Pin>),
    Remesh(PinWarpOptions),
}

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
        PinMeshView { rest: &self.rest, deformed: &self.deformed, triangles: &self.triangles, order: &self.order }
    }
}

fn pin(x: f64, y: f64) -> Pin {
    Pin { rest: Point::new(x, y), at: Point::new(x, y), angle: None, depth: 0 }
}

fn moved(mut p: Pin, dx: f64) -> Pin {
    p.at.x += dx;
    p
}

fn session() -> PinWarpSession {
    PinWarpSession::new(&PinWarpSettings::default())
}

fn drag(s: &mut PinWarpSession, f: &mut Fake, start: Option<Vec<Pin>>, ends: Vec<Pin>, now: f64) {
    let id = GestureId::next();
    s.apply(PinEdit::Pins(start.map_or_else(|| EditEvent::begin(id), |v| EditEvent::begin_with(id, v))), f, now).unwrap();
    s.apply(PinEdit::Pins(EditEvent::update(id, ends.clone(), Modifiers::NONE)), f, now).unwrap();
    s.apply(PinEdit::Pins(EditEvent::commit(id, ends, true, Modifiers::NONE)), f, now).unwrap();
    s.settle(now, false);
}

fn set(s: &mut PinWarpSession, f: &mut Fake, pins: Vec<Pin>, now: f64) {
    s.apply(PinEdit::Pins(EditEvent::set(pins, Modifiers::NONE)), f, now).unwrap();
    s.settle(now, false);
}

// `rest_of` finds the rest position under a deformed point from the top of the draw order and
// None off the mesh.
#[test]
fn a_click_on_the_mesh_places_a_pin_at_the_rest_position_under_it() {
    let rest = [Point::new(0.0, 0.0), Point::new(10.0, 0.0), Point::new(0.0, 10.0)];
    // The deformed triangle is the rest one shifted by (100, 0) and drawn twice: the later entry
    // of the order is on top and wins.
    let deformed = [Point::new(100.0, 0.0), Point::new(110.0, 0.0), Point::new(100.0, 10.0)];
    let rest2 = [Point::new(50.0, 50.0), Point::new(60.0, 50.0), Point::new(50.0, 60.0)];
    let rest_all = [rest, rest2].concat();
    let deformed_all = [deformed, deformed].concat();
    let triangles = [[0, 1, 2], [3, 4, 5]];
    let view = |order: &[u32]| {
        let order: Vec<u32> = order.to_vec();
        let leaked: &'static [u32] = Box::leak(order.into_boxed_slice());
        PinMeshView { rest: &rest_all, deformed: &deformed_all, triangles: &triangles, order: leaked }
    };
    let under = Point::new(102.0, 3.0);
    let top_last = view(&[0, 1]).rest_of(under).unwrap();
    assert!((top_last.x - 52.0).abs() < 1e-9 && (top_last.y - 53.0).abs() < 1e-9);
    let top_first = view(&[1, 0]).rest_of(under).unwrap();
    assert!((top_first.x - 2.0).abs() < 1e-9 && (top_first.y - 3.0).abs() < 1e-9);
    assert_eq!(view(&[0, 1]).rest_of(Point::new(0.0, 0.0)), None, "off the mesh");
}

// A Begin whose value adds a pin, Updates, Commit: one undo step, one rebind then solves.
#[test]
fn placing_a_pin_and_dragging_it_on_is_one_step() {
    let (mut s, mut f) = (session(), Fake::default());
    let placed = vec![pin(5.0, 5.0)];
    drag(&mut s, &mut f, Some(placed.clone()), vec![moved(placed[0], 20.0)], 0.0);
    assert!(matches!(f.calls[0], Call::Rebind(_)));
    assert!(f.calls[1..].iter().all(|c| matches!(c, Call::Solve(_))));
    assert_eq!(s.selected(), &[0], "the placed pin is selected");
    assert!(s.can_undo());
    assert!(s.undo(&mut f));
    assert!(s.pins().is_empty(), "placement and drag undo together");
    assert!(!s.can_undo());
}

// A drag of an existing pin is one step; Cancel puts the pins back and records nothing.
#[test]
fn dragging_a_pin_is_one_step_and_esc_puts_it_back() {
    let (mut s, mut f) = (session(), Fake::default());
    let start = vec![pin(5.0, 5.0)];
    set(&mut s, &mut f, start.clone(), 0.0);
    drag(&mut s, &mut f, None, vec![moved(start[0], 9.0)], 1.0);
    assert_eq!(s.pins()[0].at.x, 14.0);
    assert!(s.undo(&mut f));
    assert_eq!(s.pins(), &start[..]);

    let id = GestureId::next();
    s.apply(PinEdit::Pins(EditEvent::begin(id)), &mut f, 2.0).unwrap();
    s.apply(PinEdit::Pins(EditEvent::update(id, vec![moved(start[0], 30.0)], Modifiers::NONE)), &mut f, 2.0).unwrap();
    assert!(s.dragging());
    assert!(!s.undo(&mut f), "undo waits for the drag");
    s.apply(PinEdit::Pins(EditEvent::cancel(id, CancelReason::Escape)), &mut f, 2.0).unwrap();
    s.settle(2.0, false);
    assert!(!s.dragging());
    assert_eq!(s.pins(), &start[..]);
    assert!(s.can_redo(), "the cancelled drag recorded nothing, so the undone step is still there");
}

// A Set that drops a pin is one step and rebinds; the selection loses the deleted index.
#[test]
fn alt_click_deletes_a_pin_as_one_step() {
    let (mut s, mut f) = (session(), Fake::default());
    let pins = vec![pin(1.0, 1.0), pin(2.0, 2.0), pin(3.0, 3.0)];
    set(&mut s, &mut f, pins.clone(), 0.0);
    s.apply(PinEdit::Select(vec![0, 2, 9]), &mut f, 0.0).unwrap();
    assert_eq!(s.selected(), &[0, 2], "out-of-range indices are dropped");
    f.calls.clear();
    set(&mut s, &mut f, vec![pins[0], pins[2]], 1.0);
    assert_eq!(f.calls.len(), 1);
    assert!(matches!(f.calls[0], Call::Rebind(_)));
    assert_eq!(s.selected(), &[0, 1]);
    set(&mut s, &mut f, vec![pins[2]], 2.0);
    assert_eq!(s.selected(), &[0], "pin 0 was deleted and pin 2 slid down");
    assert!(s.undo(&mut f));
    assert_eq!(s.pins().len(), 2);
}

// A density the solver refuses restores the previous density and expansion and returns its text.
#[test]
fn a_refused_remesh_restores_density_and_expansion_and_reports_the_text() {
    let (mut s, mut f) = (session(), Fake { refuse: true, ..Fake::default() });
    let before = *s.options();
    let wanted = PinWarpOptions { density: PinDensity::More, expansion: 40.0, ..before };
    assert_eq!(s.apply(PinEdit::Options(wanted), &mut f, 0.0), Err("no pixels".to_string()));
    assert_eq!(*s.options(), before);

    f.refuse = false;
    f.calls.clear();
    assert_eq!(s.apply(PinEdit::Options(PinWarpOptions { mode: PinMode::Rigid, ..before }), &mut f, 1.0), Ok(()));
    assert!(matches!(f.calls.as_slice(), [Call::Solve(_)]), "a mode change needs no remesh");
    f.calls.clear();
    s.apply(PinEdit::Options(wanted), &mut f, 2.0).unwrap();
    assert!(matches!(f.calls.as_slice(), [Call::Remesh(_), Call::Rebind(_)]));
    assert_eq!(s.options().density, PinDensity::More);
}

// The same pins reached by two pointer paths call `solve` with the same pins (no warm start).
#[test]
fn solving_starts_from_the_rest_mesh_whatever_the_pointer_history() {
    let base = vec![pin(5.0, 5.0)];
    let target = vec![moved(base[0], 12.0)];
    let last_solve = |path: &[f64]| {
        let (mut s, mut f) = (session(), Fake::default());
        set(&mut s, &mut f, base.clone(), 0.0);
        let id = GestureId::next();
        s.apply(PinEdit::Pins(EditEvent::begin(id)), &mut f, 1.0).unwrap();
        for dx in path {
            s.apply(PinEdit::Pins(EditEvent::update(id, vec![moved(base[0], *dx)], Modifiers::NONE)), &mut f, 1.0).unwrap();
        }
        s.apply(PinEdit::Pins(EditEvent::commit(id, target.clone(), true, Modifiers::NONE)), &mut f, 1.0).unwrap();
        f.calls.pop()
    };
    assert_eq!(last_solve(&[3.0, 40.0, -7.0]), last_solve(&[12.0]));
    assert_eq!(last_solve(&[12.0]), Some(Call::Solve(target.clone())));
}

// Undo and redo restore the pins, rebind, and keep selected indices that still exist.
#[test]
fn undo_and_redo_restore_the_pins_and_keep_the_selection_when_it_still_exists() {
    let (mut s, mut f) = (session(), Fake::default());
    let two = vec![pin(1.0, 1.0), pin(2.0, 2.0)];
    set(&mut s, &mut f, vec![two[0]], 0.0);
    set(&mut s, &mut f, two.clone(), 1.0);
    s.apply(PinEdit::Select(vec![1]), &mut f, 1.0).unwrap();
    f.calls.clear();
    assert!(s.undo(&mut f));
    assert_eq!(s.pins(), &two[..1]);
    assert!(s.selected().is_empty(), "pin 1 no longer exists");
    assert!(matches!(f.calls.as_slice(), [Call::Rebind(p)] if p.len() == 1));
    s.apply(PinEdit::Select(vec![0]), &mut f, 2.0).unwrap();
    assert!(s.redo(&mut f));
    assert_eq!(s.pins(), &two[..]);
    assert_eq!(s.selected(), &[0]);
    assert!(!s.redo(&mut f));
}

// Untouched pins (and a turn of 360°) give no result; a moved pin gives the pins.
#[test]
fn an_untouched_session_has_no_result() {
    let (mut s, mut f) = (session(), Fake::default());
    assert!(s.result().is_none());
    set(&mut s, &mut f, vec![pin(1.0, 1.0)], 0.0);
    assert!(s.result().is_none(), "a pin where it attached");
    let turned = Pin { angle: Some(360.0), ..pin(1.0, 1.0) };
    set(&mut s, &mut f, vec![turned], 1.0);
    assert!(s.result().is_none(), "a full turn is no turn");
    set(&mut s, &mut f, vec![Pin { angle: Some(30.0), ..pin(1.0, 1.0) }], 2.0);
    assert_eq!(s.result().map(<[Pin]>::len), Some(1));
    set(&mut s, &mut f, vec![moved(pin(1.0, 1.0), 4.0)], 3.0);
    assert_eq!(s.result().map(<[Pin]>::len), Some(1));
}

// Arrow nudges within the pause are one step.
#[test]
fn pin_nudges_within_the_pause_fold_into_one_step() {
    let (mut s, mut f) = (PinWarpSession::new(&PinWarpSettings::default()).nudge_pause(Some(1.0)), Fake::default());
    let base = vec![pin(5.0, 5.0)];
    set(&mut s, &mut f, base.clone(), 0.0);
    for (i, dx) in [1.0, 2.0, 3.0].into_iter().enumerate() {
        let now = 10.0 + i as f64 * 0.3;
        s.apply(PinEdit::Pins(EditEvent::set(vec![moved(base[0], dx)], Modifiers::NONE)), &mut f, now).unwrap();
        s.settle(now, false);
    }
    s.settle(20.0, false);
    assert!(s.undo(&mut f));
    assert_eq!(s.pins(), &base[..], "three nudges are one step");
}

// Edges are listed once with the smaller index first, and the centre is the rest bounds' middle.
#[test]
fn edges_are_unique_and_the_centre_is_the_middle_of_the_rest_bounds() {
    let rest = [Point::new(0.0, 0.0), Point::new(10.0, 0.0), Point::new(0.0, 20.0), Point::new(10.0, 20.0)];
    let triangles = [[0, 1, 2], [2, 1, 3]];
    let view = PinMeshView { rest: &rest, deformed: &rest, triangles: &triangles, order: &[0, 1] };
    assert_eq!(view.edges(), vec![[0, 1], [0, 2], [1, 2], [1, 3], [2, 3]]);
    assert_eq!(view.centre(), Some(Point::new(5.0, 10.0)));
    let empty = PinMeshView { rest: &[], deformed: &[], triangles: &[], order: &[] };
    assert_eq!(empty.centre(), None);
}

// The turn at a rest point is the rotation of its triangle; a stretch is not a turn, and a point
// off the mesh has none.
#[test]
fn turn_at_reads_the_rotation_of_the_triangle_under_a_rest_point() {
    let rest = vec![Point::new(0.0, 0.0), Point::new(100.0, 0.0), Point::new(0.0, 100.0)];
    let (sin, cos) = 25f64.to_radians().sin_cos();
    // Stretched 2 × along x, then rotated 25°.
    let deformed: Vec<Point> = rest.iter().map(|p| Point::new(2.0 * p.x * cos - p.y * sin, 2.0 * p.x * sin + p.y * cos)).collect();
    let fake = Fake { rest, deformed, triangles: vec![[0, 1, 2]], order: vec![0], ..Fake::default() };
    let turn = fake.mesh().turn_at(Point::new(20.0, 20.0)).expect("on the mesh");
    assert!((turn - 25.0).abs() < 1e-9, "{turn}");
    assert!(fake.mesh().turn_at(Point::new(90.0, 90.0)).is_none());
}
