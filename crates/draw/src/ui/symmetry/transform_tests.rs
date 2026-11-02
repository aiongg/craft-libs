//! Tests of Transform Symmetry on the canvas tool host (task TO-S2).

use egui::{CursorIcon, Event, Key, Modifiers, PointerButton, Pos2, Sense, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use kurbo::{Point, Size};

use super::{Symmetry, SymmetryEvent, SymmetryInput, SymmetryMode, SymmetryTransform};
use craft_ui::canvas::{CanvasId, CanvasView, ToolHost, ToolHostResponse};
use craft_ui::edit::EditEvent;
use craft_ui::focus::key_owner::register_canvas;
use craft_ui::theme::{Base, install};

struct Host {
    session: SymmetryTransform,
    valid: bool,
    view: CanvasView,
    events: Vec<SymmetryEvent>,
    responses: Vec<ToolHostResponse>,
    focus: bool,
}

const SIZE: Size = Size::new(400.0, 300.0);

/// A vertical symmetry at (200, 150): the rotation handle sits at (200, 255).
fn start() -> Symmetry {
    Symmetry::new(SymmetryMode::Vertical, Point::new(200.0, 150.0), 0.0).unwrap_or_else(|| panic!("a valid symmetry"))
}

fn harness() -> Harness<'static, Host> {
    let rect = egui::Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0));
    let host = Host {
        session: SymmetryTransform::begin(start()),
        valid: true,
        view: CanvasView::new(rect, Point::new(200.0, 150.0), 1.0),
        events: Vec::new(),
        responses: Vec::new(),
        focus: false,
    };
    let mut h = Harness::builder()
        .with_size(vec2(400.0, 300.0))
        .with_step_dt(0.05)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, host: &mut Host| {
                let id = CanvasId::new(ui, "symmetry");
                let canvas = id.interact(ui, host.view.rect, Sense::click_and_drag() | Sense::FOCUSABLE);
                register_canvas(ui.ctx(), canvas.id);
                if std::mem::take(&mut host.focus) {
                    canvas.request_focus();
                }
                let mut pass = host.session.pass(SymmetryInput::new(SIZE, host.valid));
                let response = ToolHost::new(id).document(SIZE).show(ui, &host.view, &canvas, &mut pass);
                host.events.extend(pass.finish().events);
                host.responses.push(response);
            },
            host,
        );
    h.step();
    h
}

fn focused(h: &mut Harness<'_, Host>) {
    h.state_mut().focus = true;
    h.step();
}

fn button(at: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos: at, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

fn move_to(h: &mut Harness<'_, Host>, at: Pos2, modifiers: Modifiers) {
    h.event_modifiers(Event::PointerMoved(at), modifiers);
    h.step();
}

/// Presses at `from`, moves through the middle to `to` (holding `modifiers`) and releases there.
fn drag(h: &mut Harness<'_, Host>, from: Pos2, to: Pos2, modifiers: Modifiers) {
    move_to(h, from, Modifiers::NONE);
    h.event(button(from, true));
    h.step();
    move_to(h, from.lerp(to, 0.5), modifiers);
    move_to(h, to, modifiers);
    h.event_modifiers(button(to, false), modifiers);
    h.step();
}

fn key(h: &mut Harness<'_, Host>, key: Key, modifiers: Modifiers) {
    h.key_press_modifiers(modifiers, key);
    h.run();
}

fn preview(h: &Harness<'_, Host>) -> Symmetry {
    h.state().session.preview()
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

fn gesture_events(h: &Harness<'_, Host>) -> Vec<EditEvent<Symmetry>> {
    h.state().events.iter().filter_map(|e| if let SymmetryEvent::Preview(p) = e { Some(p.clone()) } else { None }).collect()
}

// Through `ToolHost`: a drag from the centre moves it, from the handle turns the axes (Shift in absolute 15° steps), from elsewhere is claimed and changes nothing; events are `Preview` gestures.
#[test]
fn centre_moves_handle_turns_with_shift_steps_elsewhere_does_nothing() {
    let mut h = harness();
    drag(&mut h, pos2(200.0, 150.0), pos2(230.0, 170.0), Modifiers::NONE);
    assert_eq!(preview(&h).center, Point::new(230.0, 170.0));
    let events = gesture_events(&h);
    assert!(matches!(events.first(), Some(EditEvent::Begin { .. })), "{events:?}");
    assert!(matches!(events.last(), Some(EditEvent::Commit { changed: true, .. })), "{events:?}");

    // The handle is now at (230, 275): a quarter turn counter-clockwise puts it east of the centre.
    drag(&mut h, pos2(230.0, 275.0), pos2(335.0, 170.0), Modifiers::NONE);
    assert!(near(preview(&h).rotation, 270.0), "{:?}", preview(&h));

    // The handle is now at (335, 170); 53° toward the south turns to 323°, which rounds to 330° in 15° steps.
    let turned = f64::from(53.0_f32).to_radians();
    let to = pos2(230.0 + (105.0 * turned.cos()) as f32, 170.0 + (105.0 * turned.sin()) as f32);
    drag(&mut h, pos2(335.0, 170.0), to, Modifiers::SHIFT);
    let rotation = preview(&h).rotation;
    assert!(near(rotation.rem_euclid(15.0), 0.0) && near(rotation, 330.0), "{rotation}");

    let before = preview(&h);
    h.state_mut().events.clear();
    drag(&mut h, pos2(50.0, 50.0), pos2(60.0, 70.0), Modifiers::NONE);
    assert_eq!(preview(&h), before);
    assert!(h.state().events.is_empty(), "{:?}", h.state().events);
    assert!(h.state().responses.iter().all(|r| r.ignored_press.is_none()), "the tool claims the press");
}

// Enter gives Apply of the preview, Esc gives Discard, an input with `valid` false gives Discard once and takes no press.
#[test]
fn enter_applies_escape_discards_invalid_discards() {
    let mut h = harness();
    drag(&mut h, pos2(200.0, 150.0), pos2(210.0, 150.0), Modifiers::NONE);
    focused(&mut h);
    h.state_mut().events.clear();
    key(&mut h, Key::Enter, Modifiers::NONE);
    let moved = Symmetry::new(SymmetryMode::Vertical, Point::new(210.0, 150.0), 0.0).unwrap_or_else(|| panic!("a valid symmetry"));
    assert_eq!(h.state().events, [SymmetryEvent::Apply(moved)]);

    let mut h = harness();
    focused(&mut h);
    key(&mut h, Key::Escape, Modifiers::NONE);
    assert_eq!(h.state().events, [SymmetryEvent::Discard]);

    let mut h = harness();
    h.state_mut().valid = false;
    h.run();
    drag(&mut h, pos2(200.0, 150.0), pos2(260.0, 150.0), Modifiers::NONE);
    assert_eq!(h.state().events, [SymmetryEvent::Discard]);
    assert_eq!(preview(&h), start());
}

// Move over the centre, Alias over the handle, the arrow elsewhere.
#[test]
fn cursor_follows_the_part_under_the_pointer() {
    let mut h = harness();
    let cursor = |h: &Harness<'_, Host>| h.state().responses.last().map(|r| r.cursor);
    move_to(&mut h, pos2(203.0, 150.0), Modifiers::NONE);
    assert_eq!(cursor(&h), Some(CursorIcon::Move));
    move_to(&mut h, pos2(200.0, 258.0), Modifiers::NONE);
    assert_eq!(cursor(&h), Some(CursorIcon::Alias));
    move_to(&mut h, pos2(80.0, 60.0), Modifiers::NONE);
    assert_eq!(cursor(&h), Some(CursorIcon::Default));
}

// With the parts group focused and Center picked the arrows move it 1, Shift 10; with the handle picked Right turns 1° clockwise, Shift 15°, Left the other way; each is a `Preview(Set)`.
#[test]
fn keys_move_the_center_and_turn_the_handle() {
    let mut h = harness();
    // A claimed press released on a part gives the parts group the keyboard.
    drag(&mut h, pos2(200.0, 150.0), pos2(200.0, 150.0), Modifiers::NONE);
    assert_eq!(h.state().session.picked(), Some(0));
    h.state_mut().events.clear();
    key(&mut h, Key::ArrowRight, Modifiers::NONE);
    key(&mut h, Key::ArrowDown, Modifiers::SHIFT);
    assert_eq!(preview(&h).center, Point::new(201.0, 160.0));
    let sets = gesture_events(&h);
    assert_eq!(sets.len(), 2, "{sets:?}");
    assert!(sets.iter().all(|e| matches!(e, EditEvent::Set { .. })), "{sets:?}");

    // The handle is at (201, 265).
    drag(&mut h, pos2(201.0, 265.0), pos2(201.0, 265.0), Modifiers::NONE);
    assert_eq!(h.state().session.picked(), Some(1));
    key(&mut h, Key::ArrowRight, Modifiers::NONE);
    assert!(near(preview(&h).rotation, 1.0), "{:?}", preview(&h));
    key(&mut h, Key::ArrowRight, Modifiers::SHIFT);
    assert!(near(preview(&h).rotation, 16.0), "{:?}", preview(&h));
    key(&mut h, Key::ArrowLeft, Modifiers::NONE);
    assert!(near(preview(&h).rotation, 15.0), "{:?}", preview(&h));
    key(&mut h, Key::ArrowUp, Modifiers::NONE);
    assert!(near(preview(&h).rotation, 15.0), "Up and Down do not turn: {:?}", preview(&h));
}

// With the handle picked but the canvas (not the parts group) focused, the arrows move the whole
// centre, as Crop moves its whole frame.
#[test]
fn arrows_move_the_center_while_the_canvas_has_the_keyboard() {
    let mut h = harness();
    drag(&mut h, pos2(200.0, 255.0), pos2(200.0, 255.0), Modifiers::NONE);
    assert_eq!(h.state().session.picked(), Some(1));
    focused(&mut h);
    key(&mut h, Key::ArrowRight, Modifiers::NONE);
    assert_eq!(preview(&h).center, Point::new(201.0, 150.0));
    assert!(near(preview(&h).rotation, 0.0), "{:?}", preview(&h));
}

// The session opens from a menu, so nothing has the keyboard: Enter and Esc still end it.
#[test]
fn enter_and_escape_work_before_the_canvas_is_focused() {
    let mut h = harness();
    assert!(h.ctx.memory(|m| m.focused()).is_none());
    key(&mut h, Key::Enter, Modifiers::NONE);
    assert_eq!(h.state().events, [SymmetryEvent::Apply(start())]);

    let mut h = harness();
    key(&mut h, Key::Escape, Modifiers::NONE);
    assert_eq!(h.state().events, [SymmetryEvent::Discard]);
}

// Esc during a drag ends the drag only; the session goes on.
#[test]
fn escape_during_a_drag_cancels_the_drag_not_the_session() {
    let mut h = harness();
    move_to(&mut h, pos2(200.0, 150.0), Modifiers::NONE);
    h.event(button(pos2(200.0, 150.0), true));
    h.step();
    move_to(&mut h, pos2(230.0, 150.0), Modifiers::NONE);
    key(&mut h, Key::Escape, Modifiers::NONE);
    assert_eq!(preview(&h), start());
    assert!(!h.state().events.iter().any(|e| matches!(e, SymmetryEvent::Discard | SymmetryEvent::Apply(_))), "{:?}", h.state().events);
}

// Only the picked part is marked selected for a screen reader.
#[test]
fn the_picked_part_is_selected() {
    let mut h = harness();
    drag(&mut h, pos2(200.0, 255.0), pos2(200.0, 255.0), Modifiers::NONE);
    h.run();
    let selected = |h: &Harness<'_, Host>, name: &str| h.get_by_label(name).accesskit_node().data().is_selected();
    assert_eq!(selected(&h, "Rotation handle"), Some(true));
    assert_ne!(selected(&h, "Center"), Some(true));
}
