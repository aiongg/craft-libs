//! Tests of the perspective overlay.

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Sense, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use kurbo::{Point, Rect};

use super::*;
use craft_ui::canvas::CanvasView;
use craft_ui::edit::EditEvent;
use craft_ui::theme::{Base, install};

struct Host {
    session: PerspectiveSession,
    view: Option<CanvasView>,
    edits: Vec<PerspectiveEdit>,
}

fn square(x0: f64, y0: f64, x1: f64, y1: f64) -> Plane {
    Plane::identity([Point::new(x0, y0), Point::new(x1, y0), Point::new(x1, y1), Point::new(x0, y1)])
}

fn harness(planes: Vec<Plane>, mode: PlaneMode) -> Harness<'static, Host> {
    let mut session = PerspectiveSession::new(Rect::new(0.0, 0.0, 400.0, 300.0));
    if !planes.is_empty() {
        session.apply(PerspectiveEdit::Planes(EditEvent::set(planes, Modifiers::NONE)), 0.0);
    }
    session.apply(PerspectiveEdit::Mode(mode), 0.0);
    let host = Host { session, view: None, edits: Vec::new() };
    let mut h = Harness::builder().with_size([480.0, 400.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut Host| {
            ui.add_space(40.0);
            let (rect, canvas) = ui.allocate_exact_size(vec2(400.0, 300.0), Sense::click_and_drag() - Sense::FOCUSABLE);
            let view = CanvasView::new(rect, Point::new(200.0, 150.0), 1.0);
            host.view = Some(view);
            let r = PerspectiveWarp::new("planes", &host.session).show(ui, &canvas, &view);
            let now = ui.input(|i| i.time);
            for edit in &r.edits {
                host.session.apply(edit.clone(), now);
            }
            host.session.settle(now, ui.input(|i| i.pointer.any_down()));
            host.edits.extend(r.edits);
        },
        host,
    );
    h.run_steps(3);
    h
}

fn at(h: &Harness<'_, Host>, x: f64, y: f64) -> Pos2 {
    h.state().view.expect("drawn").to_screen(Point::new(x, y))
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

fn drag(h: &mut Harness<'_, Host>, from: (f64, f64), to: (f64, f64)) {
    let (from, to) = (at(h, from.0, from.1), at(h, to.0, to.1));
    h.event(Event::PointerMoved(from));
    h.event(button(from, true));
    h.event(Event::PointerMoved(from.lerp(to, 0.5)));
    h.event(Event::PointerMoved(to));
    h.event(button(to, false));
    h.run_steps(2);
}

fn planes(h: &Harness<'_, Host>) -> Vec<Plane> {
    h.state().session.planes().to_vec()
}

// A Layout drag of 10 × 10 adds a plane; a drag under 4 units each way adds none.
#[test]
fn a_drag_in_layout_draws_a_plane_and_a_tiny_drag_adds_none() {
    let mut h = harness(vec![], PlaneMode::Layout);
    drag(&mut h, (50.0, 50.0), (60.0, 60.0));
    assert_eq!(planes(&h), vec![square(50.0, 50.0, 60.0, 60.0)]);

    drag(&mut h, (150.0, 150.0), (152.0, 153.0));
    assert_eq!(planes(&h).len(), 1, "3 units is under the minimum");
    drag(&mut h, (150.0, 150.0), (160.0, 152.0));
    assert_eq!(planes(&h).len(), 1, "so is 2 in one direction");
}

// A corner dropped within `transform.hit` of another plane's corner takes its position, which
// links them.
#[test]
fn a_corner_dropped_near_another_planes_corner_snaps_and_links() {
    let mut h = harness(vec![square(100.0, 100.0, 200.0, 200.0), square(220.0, 100.0, 300.0, 200.0)], PlaneMode::Layout);
    drag(&mut h, (220.0, 100.0), (205.0, 102.0));
    let moved = planes(&h);
    assert_eq!(moved[1].source[0], Point::new(200.0, 100.0), "snapped to the first plane's corner");
    assert_eq!(moved[1].target[0], moved[1].source[0], "Layout moves source and target together");
    let group = linked_corners(&moved).into_iter().find(|g| g.len() > 1).expect("linked");
    assert_eq!(group, vec![PlaneCorner { plane: 0, corner: 1 }, PlaneCorner { plane: 1, corner: 0 }]);

    // Dragged on, the linked pair moves together.
    drag(&mut h, (200.0, 100.0), (200.0, 60.0));
    let moved = planes(&h);
    assert_eq!((moved[0].source[1], moved[1].source[0]), (Point::new(200.0, 60.0), Point::new(200.0, 60.0)));
}

// In Warp the target moves, the layout stays, and linked corners go together.
#[test]
fn a_warp_drag_moves_the_target_of_linked_corners_and_keeps_the_layout() {
    let mut h = harness(vec![square(100.0, 100.0, 200.0, 200.0), square(200.0, 100.0, 300.0, 200.0)], PlaneMode::Warp);
    drag(&mut h, (200.0, 100.0), (210.0, 80.0));
    let moved = planes(&h);
    assert_eq!((moved[0].target[1], moved[1].target[0]), (Point::new(210.0, 80.0), Point::new(210.0, 80.0)));
    assert_eq!((moved[0].source[1], moved[1].source[0]), (Point::new(200.0, 100.0), Point::new(200.0, 100.0)));

    // A press away from a corner draws nothing in Warp.
    let before = h.state().edits.len();
    drag(&mut h, (20.0, 20.0), (60.0, 60.0));
    assert_eq!(h.state().edits.len(), before);
}

// When the generation changes in the middle of a corner drag, the drag ends with Cancel and the
// next move reports nothing.
#[test]
fn replacing_the_planes_in_the_middle_of_a_corner_drag_ends_the_drag() {
    let mut h = harness(vec![square(100.0, 100.0, 200.0, 200.0)], PlaneMode::Layout);
    let from = at(&h, 100.0, 100.0);
    h.event(Event::PointerMoved(from));
    h.event(button(from, true));
    h.event(Event::PointerMoved(from + vec2(20.0, 20.0)));
    h.run_steps(2);
    assert!(h.state().session.dragging());
    assert_eq!(planes(&h)[0].source[0], Point::new(120.0, 120.0));

    h.state_mut().session.replace(vec![]);
    h.run_steps(2);
    assert!(!h.state().session.dragging());
    assert!(h.state().edits.iter().any(|e| matches!(e, PerspectiveEdit::Planes(EditEvent::Cancel { .. }))));

    let before = h.state().edits.len();
    h.event(Event::PointerMoved(from + vec2(40.0, 40.0)));
    h.run_steps(2);
    h.event(button(from + vec2(40.0, 40.0), false));
    h.run_steps(2);
    assert_eq!(h.state().edits.len(), before, "nothing after the replacement");
    assert!(h.state().session.planes().is_empty());
}

// Insert adds the middle-half plane, Page keys pick corners, arrows move them, Delete removes the
// picked corner's plane, and so does Backspace (a Mac laptop's delete key).
#[test]
fn the_keyboard_adds_a_plane_picks_corners_and_removes_a_plane() {
    let mut h = harness(vec![square(100.0, 100.0, 200.0, 200.0)], PlaneMode::Layout);
    drag(&mut h, (100.0, 100.0), (100.0, 100.0));
    h.key_press(Key::PageDown);
    h.run_steps(2);
    h.key_press(Key::ArrowRight);
    h.run_steps(2);
    h.key_press_modifiers(Modifiers::SHIFT, Key::ArrowDown);
    h.run_steps(2);
    let moved = planes(&h)[0];
    assert_eq!(moved.source[1], Point::new(201.0, 110.0));
    assert_eq!(moved.target[1], moved.source[1]);
    assert_eq!(moved.source[0], Point::new(100.0, 100.0), "only the picked corner");

    h.key_press(Key::Insert);
    h.run_steps(2);
    assert_eq!(planes(&h).len(), 2);
    assert_eq!(planes(&h)[1], h.state().session.inserted_plane());
    assert_eq!(planes(&h)[1], square(100.0, 75.0, 300.0, 225.0));

    h.key_press(Key::Delete);
    h.run_steps(2);
    assert_eq!(planes(&h).len(), 1, "the new plane held the picked corner");

    h.key_press(Key::Insert);
    h.run_steps(2);
    assert_eq!(planes(&h).len(), 2);
    h.key_press(Key::Backspace);
    h.run_steps(2);
    assert_eq!(planes(&h).len(), 1, "Backspace removes the picked corner's plane as Delete does");
    h.key_press(Key::Enter);
    h.run_steps(2);
}

// AccessKit: each corner named "Plane 2, corner 3" with value "x, y", "linked" when linked.
#[test]
fn every_corner_is_named_and_valued_for_assistive_technology() {
    let h = harness(vec![square(100.0, 100.0, 200.0, 200.0), square(200.0, 100.0, 300.0, 200.0)], PlaneMode::Layout);
    assert!(h.query_by_role_and_label(Role::Group, "Perspective Warp").is_some());
    let third = h.get_by_label("Plane 2, corner 3");
    assert_eq!(third.accesskit_node().role(), Role::GraphicsObject);
    assert_eq!(third.accesskit_node().value().as_deref(), Some("300.0 px, 200.0 px"));
    assert_eq!(h.get_by_label("Plane 2, corner 1").accesskit_node().value().as_deref(), Some("200.0 px, 100.0 px, linked"));
    assert_eq!(h.get_by_label("Plane 1, corner 1").accesskit_node().value().as_deref(), Some("100.0 px, 100.0 px"));
}
