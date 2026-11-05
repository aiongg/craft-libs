//! Tests of the perspective session and plane geometry (task TR-T7).

use egui::Modifiers;
use kurbo::{Point, Rect};

use craft_ui::edit::{EditEvent, GestureId};

use super::session::{
    LINK_DISTANCE, PerspectiveEdit, PerspectiveSession, Plane, PlaneCorner, PlaneMode, Straighten, linked_corners, straighten, unify,
};

fn quad(x0: f64, y0: f64, x1: f64, y1: f64) -> [Point; 4] {
    [Point::new(x0, y0), Point::new(x1, y0), Point::new(x1, y1), Point::new(x0, y1)]
}

/// Two planes side by side sharing the corners at x = 50.
fn two_planes() -> Vec<Plane> {
    vec![Plane::identity(quad(0.0, 0.0, 50.0, 100.0)), Plane::identity(quad(50.0, 0.0, 100.0, 100.0))]
}

fn session() -> PerspectiveSession {
    PerspectiveSession::new(Rect::new(0.0, 0.0, 100.0, 100.0))
}

fn set(s: &mut PerspectiveSession, planes: Vec<Plane>, now: f64) {
    s.apply(PerspectiveEdit::Planes(EditEvent::set(planes, Modifiers::NONE)), now);
}

fn drag(s: &mut PerspectiveSession, planes: Vec<Plane>) {
    let id = GestureId::next();
    s.apply(PerspectiveEdit::Planes(EditEvent::begin(id)), 0.0);
    s.apply(PerspectiveEdit::Planes(EditEvent::update(id, planes.clone(), Modifiers::NONE)), 0.0);
    s.apply(PerspectiveEdit::Planes(EditEvent::commit(id, planes, true, Modifiers::NONE)), 0.0);
}

// Corners within 0.5 in the source are one linked group; others are alone.
#[test]
fn corners_within_the_link_distance_are_linked() {
    let mut planes = two_planes();
    planes[1].source[0].x += LINK_DISTANCE * 0.9;
    let groups = linked_corners(&planes);
    assert_eq!(groups.len(), 6);
    let shared = groups.iter().find(|g| g.contains(&PlaneCorner { plane: 0, corner: 1 })).unwrap();
    assert_eq!(shared, &vec![PlaneCorner { plane: 0, corner: 1 }, PlaneCorner { plane: 1, corner: 0 }]);
    planes[1].source[0].x += 1.0;
    assert_eq!(linked_corners(&planes).len(), 7);
}

// A Warp corner drag moves the linked targets together and keeps the sources.
#[test]
fn linked_corners_move_together_in_warp_and_layout_is_kept() {
    let mut s = session();
    set(&mut s, two_planes(), 0.0);
    s.apply(PerspectiveEdit::Mode(PlaneMode::Warp), 0.0);
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

// Mode(Warp) unifies linked corners to their means.
#[test]
fn entering_warp_unifies_linked_corners() {
    let mut planes = two_planes();
    planes[1].source[0] = Point::new(50.2, 0.0);
    planes[1].target[0] = Point::new(51.0, 2.0);
    planes[0].target[1] = Point::new(49.0, 0.0);
    let mut copy = planes.clone();
    unify(&mut copy);
    assert_eq!(copy[0].target[1], Point::new(50.0, 1.0));
    let mut s = session();
    set(&mut s, planes, 0.0);
    s.apply(PerspectiveEdit::Mode(PlaneMode::Warp), 0.0);
    assert_eq!(s.mode(), PlaneMode::Warp);
    assert_eq!(s.planes()[0].source[1], Point::new(50.1, 0.0));
    assert_eq!(s.planes()[1].source[0], Point::new(50.1, 0.0));
    assert_eq!(s.planes()[0].target[1], Point::new(50.0, 1.0));
    assert_eq!(s.planes()[1].target[0], Point::new(50.0, 1.0));
}

// Straighten levels near-vertical edges, Level near-horizontal ones, Both both, moving linked
// corners together.
#[test]
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
        let mut plane = Plane::identity(quad(0.0, 0.0, 50.0, 100.0));
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

// Mode(Warp) with no planes changes nothing.
#[test]
fn warp_needs_at_least_one_plane() {
    let mut s = session();
    s.apply(PerspectiveEdit::Mode(PlaneMode::Warp), 0.0);
    assert_eq!(s.mode(), PlaneMode::Layout);
    assert!(!s.can_undo());
}

// `replace` bumps the generation and ends a running drag; untouched planes give no result.
#[test]
fn replacing_the_planes_bumps_the_generation_and_identity_gives_no_result() {
    let mut s = session();
    assert_eq!(s.result(), None);
    let inserted = s.inserted_plane();
    set(&mut s, vec![inserted], 0.0);
    assert_eq!(s.inserted_plane().source, quad(25.0, 25.0, 75.0, 75.0));
    assert_eq!(s.result(), None);
    let id = GestureId::next();
    s.apply(PerspectiveEdit::Planes(EditEvent::begin(id)), 0.0);
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

// A drag is one undo step; undo restores planes and mode together.
#[test]
fn plane_edits_are_undo_steps() {
    let mut s = session();
    set(&mut s, two_planes(), 0.0);
    s.settle(1.0, false);
    s.apply(PerspectiveEdit::Mode(PlaneMode::Warp), 1.0);
    let mut moved = s.planes().to_vec();
    moved[0].target[0] = Point::new(5.0, 5.0);
    drag(&mut s, moved);
    s.settle(2.0, false);
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
