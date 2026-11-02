//! Tests of the Polygonal Lasso on the tool host (task SE-F9), on the rig of `lasso_tests`.

use craft_ui::canvas::SelectionOperation;
use craft_ui::canvas::kurbo::{Point, Rect};
use craft_ui::edit::CancelReason;
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
use craft_ui::testing::canvas;
use egui::{Key, Modifiers, PointerButton, pos2};
use egui_kittest::Harness;

use super::lasso_tests::{Fake, NONE, Rig, cancel_with, click, down, drag_to, press_key, rig, tags, up};
use super::{DragKind, SelectEvent, SelectPhase, SelectionShape};

// ⇧ turns a segment to 45° steps from the last vertex, not for the first vertex and not with ⌥.
#[test]
fn polygon_shift_constrains_to_45_degrees() {
    let mut h = rig(true, Fake::empty());
    h.state_mut().focus = true;
    h.step();
    click(&mut h, (50.0, 50.0), Modifiers::SHIFT);
    // 11° from the last vertex rounds to 0°, keeping the distance √(100² + 20²).
    click(&mut h, (150.0, 70.0), Modifiers::SHIFT);
    click(&mut h, (140.0, 170.0), Modifiers::SHIFT | Modifiers::ALT);
    press_key(&mut h, Key::Enter);
    let events = h.state_mut().events();
    let [SelectEvent::Commit { shape: SelectionShape::Polygon(points), operation }] = events.as_slice() else { panic!("one commit: {events:?}") };
    assert_eq!(*operation, SelectionOperation::Add);
    let turned = Point::new(50.0 + 100.0_f64.hypot(20.0), 50.0);
    assert_eq!(points.len(), 3);
    assert_eq!(points[0], Point::new(50.0, 50.0), "the first vertex is never turned");
    assert!((points[1] - turned).hypot() < 1e-9, "{:?}", points[1]);
    assert_eq!(points[2], Point::new(140.0, 170.0), "⇧ with ⌥ keeps the point");
}

// A plain press inside the selection places a vertex; only ⌘ drags it, and not while an outline
// is open.
#[test]
fn polygon_click_driven_needs_command_to_drag() {
    let mut h = rig(true, Fake::selected(Rect::new(100.0, 100.0, 200.0, 200.0)));
    click(&mut h, (150.0, 150.0), NONE);
    assert_eq!(h.state().phase(), SelectPhase::Outline { points: 1 });
    // ⌘ with an outline open places a vertex too.
    click(&mut h, (180.0, 120.0), Modifiers::COMMAND);
    assert_eq!(h.state().phase(), SelectPhase::Outline { points: 2 });
    assert!(h.state_mut().events().is_empty());
    h.state_mut().polygon.reset();

    down(&mut h, (150.0, 150.0), Modifiers::COMMAND);
    assert_eq!(h.state().phase(), SelectPhase::Dragging(DragKind::Pixels { copy: false }));
    drag_to(&mut h, (155.0, 150.0), Modifiers::COMMAND);
    up(&mut h, (155.0, 150.0), Modifiers::COMMAND);
    assert_eq!(tags(&h.state_mut().events()), ["begin", "update 5,0", "commit 5,0 changed"]);

    // ⌘ outside the selection is the Move tool's.
    assert!(!h.state().ignored);
    click(&mut h, (300.0, 250.0), Modifiers::COMMAND);
    assert!(h.state().ignored);
    assert_eq!(h.state().phase(), SelectPhase::Idle);
}

// A secondary click while an outline is open (the tool takes the button then), Backspace and
// Delete remove the last vertex; the last one cancels with no event. With no outline open the
// secondary button starts nothing.
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
#[test]
fn polygon_secondary_click_removes_last_vertex_then_cancels() {
    let mut h = rig(true, Fake::empty());
    h.state_mut().focus = true;
    h.step();
    let secondary = |h: &mut Harness<'_, Rig>| {
        canvas::press_button(h, pos2(10.0, 10.0), PointerButton::Secondary, NONE);
        canvas::release_button(h, pos2(10.0, 10.0), PointerButton::Secondary, NONE);
    };
    secondary(&mut h);
    assert_eq!(h.state().phase(), SelectPhase::Idle, "nothing open");
    for at in [(50.0, 50.0), (150.0, 50.0), (150.0, 150.0)] {
        canvas::press(&mut h, pos2(at.0 as f32, at.1 as f32), NONE);
        canvas::release(&mut h, pos2(at.0 as f32, at.1 as f32), NONE);
    }
    secondary(&mut h);
    assert_eq!(h.state().phase(), SelectPhase::Outline { points: 2 });
    press_key(&mut h, Key::Backspace);
    assert_eq!(h.state().phase(), SelectPhase::Outline { points: 1 });
    secondary(&mut h);
    assert_eq!(h.state().phase(), SelectPhase::Idle);
    assert!(h.state_mut().events().is_empty());
}

// Esc during a press drops the whole outline; a cancel for another reason (a tool switch) keeps it
// for the app's `reset`.
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
#[test]
fn polygon_escape_during_press_drops_outline() {
    let mut h = rig(true, Fake::empty());
    h.state_mut().focus = true;
    h.step();
    for at in [(50.0, 50.0), (150.0, 50.0), (150.0, 150.0)] {
        click(&mut h, at, NONE);
    }
    canvas::press(&mut h, pos2(100.0, 150.0), NONE);
    assert_eq!(h.state().phase(), SelectPhase::Outline { points: 4 });
    canvas::key(&mut h, Key::Escape, NONE);
    assert_eq!(h.state().phase(), SelectPhase::Idle);
    canvas::release(&mut h, pos2(100.0, 150.0), NONE);
    assert!(h.state_mut().events().is_empty());

    for at in [(50.0, 50.0), (150.0, 50.0)] {
        click(&mut h, at, NONE);
    }
    canvas::press(&mut h, pos2(100.0, 150.0), NONE);
    cancel_with(&mut h, CancelReason::Removed);
    assert_eq!(h.state().phase(), SelectPhase::Outline { points: 3 }, "a tool switch keeps the outline");
}

// `replaces_selection` is true only while a New outline is open.
#[test]
fn polygon_new_outline_replaces_selection() {
    let mut h = rig(true, Fake::selected(Rect::new(100.0, 100.0, 200.0, 200.0)));
    assert!(!h.state().polygon.replaces_selection());
    click(&mut h, (20.0, 20.0), NONE);
    assert!(h.state().polygon.replaces_selection());
    h.state_mut().polygon.reset();
    assert!(!h.state().polygon.replaces_selection());
    click(&mut h, (20.0, 20.0), Modifiers::SHIFT);
    assert_eq!(h.state().phase(), SelectPhase::Outline { points: 1 });
    assert!(!h.state().polygon.replaces_selection(), "Add keeps the selection visible");
}
