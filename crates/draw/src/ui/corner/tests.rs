//! Tests of the corner widgets.

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Sense, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use kurbo::Point;

use super::{CornerEdit, CornerKind, CornerPoint, CornerRef, CornerTarget, CornerWidgets};
use craft_ui::canvas::{CanvasView, RulerUnits};
use craft_ui::edit::{EditEvent, EditPolicy, Gesture};
use craft_ui::localized::LocaleNumbers;
use craft_ui::theme::{Base, install};

/// A host with a 400 × 300 canvas showing document (0, 0)–(400, 300) at zoom 1, a rectangle
/// (key 1, 50–250 × 50–150, `individual`) and a square (key 2, 260–340 × 200–280) shown.
struct Host {
    targets: Vec<CornerTarget>,
    kinds: bool,
    view: Option<CanvasView>,
    events: Vec<EditEvent<CornerEdit>>,
    cycle: Option<(CornerRef, CornerKind)>,
    activate: Vec<CornerRef>,
    claimed: bool,
    hovered: bool,
    /// A text field's typing run beside the canvas, begun in its first pass.
    typing: bool,
    typing_began: bool,
    /// Passes run, and the one in which the typing run committed.
    passes: usize,
    committed: Option<usize>,
}

fn quad(key: u64, (x0, y0, x1, y1): (f64, f64, f64, f64), radius: f64, individual: bool) -> CornerTarget {
    let p = [Point::new(x0, y0), Point::new(x1, y0), Point::new(x1, y1), Point::new(x0, y1)];
    let corners = (0..4).map(|i| CornerPoint { vertex: p[i], prev: p[(i + 3) % 4], next: p[(i + 1) % 4], radius, kind: CornerKind::Round }).collect();
    CornerTarget { key, corners, individual }
}

fn apply(targets: &mut [CornerTarget], edit: &CornerEdit) {
    for r in &edit.targets {
        if let Some(t) = targets.iter_mut().find(|t| t.key == r.key) {
            for (c, radius) in t.corners.iter_mut().zip(&r.radii) {
                c.radius = *radius;
            }
        }
    }
}

fn harness(radius: f64, kinds: bool) -> Harness<'static, Host> {
    let host = Host {
        targets: vec![quad(1, (50.0, 50.0, 250.0, 150.0), radius, true), quad(2, (260.0, 200.0, 340.0, 280.0), radius, false)],
        kinds,
        view: None,
        events: Vec::new(),
        cycle: None,
        activate: Vec::new(),
        claimed: false,
        hovered: false,
        typing: false,
        typing_began: false,
        passes: 0,
        committed: None,
    };
    // A small step keeps two presses in one spot within the double-click delay.
    let mut h = Harness::<Host>::builder()
        .with_size([480.0, 400.0])
        .with_step_dt(0.02)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, host: &mut Host| {
                host.passes += 1;
                if host.typing {
                    let field = ui.make_persistent_id("typing field");
                    let response = ui.interact(egui::Rect::from_min_size(egui::pos2(420.0, 320.0), vec2(20.0, 20.0)), field, Sense::click());
                    let first = !host.typing_began;
                    host.typing_began = true;
                    if first {
                        response.request_focus();
                    }
                    let mut run = Gesture::load(ui, field, &0, EditPolicy::default());
                    if first {
                        run.begin();
                    }
                    if matches!(run.finish(), Some(EditEvent::Commit { .. })) {
                        host.committed = Some(host.passes);
                    }
                }
                let (rect, canvas) = ui.allocate_exact_size(vec2(400.0, 300.0), Sense::click_and_drag() - Sense::FOCUSABLE);
                let view = CanvasView::new(rect, Point::new(200.0, 150.0), 1.0);
                host.view = Some(view);
                let px = RulerUnits::pixels(LocaleNumbers::from_tag("en"));
                let targets = host.targets.clone();
                let r = CornerWidgets::new("corners", &targets, &px).kinds(host.kinds).show(ui, &canvas, &view);
                for e in r.event.iter() {
                    if let Some(edit) = e.value() {
                        apply(&mut host.targets, edit);
                    }
                }
                host.events.extend(r.event.clone());
                host.cycle = host.cycle.or(r.cycle_kind);
                host.activate.extend(r.activate);
                host.claimed |= r.claimed;
                host.hovered |= r.hovered;
            },
            host,
        );
    h.run();
    h
}

fn screen(h: &Harness<'_, Host>, x: f64, y: f64) -> Pos2 {
    h.state().view.expect("drawn").to_screen(Point::new(x, y))
}

fn button(pos: Pos2, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers }
}

fn press(h: &mut Harness<'_, Host>, at: Pos2, modifiers: Modifiers) {
    h.event(Event::ModifiersChanged(modifiers));
    h.event(Event::PointerMoved(at));
    h.event(button(at, true, modifiers));
    h.run();
}

fn move_to(h: &mut Harness<'_, Host>, at: Pos2) {
    h.event(Event::PointerMoved(at));
    h.run();
}

fn release(h: &mut Harness<'_, Host>, at: Pos2, modifiers: Modifiers) {
    h.event(button(at, false, modifiers));
    h.event(Event::ModifiersChanged(Modifiers::NONE));
    h.run();
}

fn kinds(h: &Harness<'_, Host>) -> Vec<&'static str> {
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

fn last_value(h: &Harness<'_, Host>) -> CornerEdit {
    let Some(value) = h.state().events.iter().rev().find_map(|e| e.value()) else { panic!("no event carried a value") };
    value.clone()
}

fn descendants(node: egui_kittest::kittest::AccessKitNode<'_>) -> Vec<egui_kittest::kittest::AccessKitNode<'_>> {
    node.children().flat_map(|child| std::iter::once(child).chain(descendants(child))).collect()
}

fn radii(edit: &CornerEdit, key: u64) -> Vec<f64> {
    edit.targets.iter().find(|t| t.key == key).map(|t| t.radii.clone()).unwrap_or_default()
}

// Dragging a rectangle's widget reports Begin, Update and Commit with every selected target at the
// same radius; with ⌥ only the dragged corner of the rectangle changes and a star's all.
#[test]
fn corner_drag_reports_every_target_and_alt_one_corner() {
    for (modifiers, rect) in [(Modifiers::NONE, vec![20.0; 4]), (Modifiers::ALT, vec![20.0, 0.0, 0.0, 0.0])] {
        let mut h = harness(0.0, false);
        // The top left corner's widget sits 10 points inside it; a drag along the diagonal moves
        // the arc's centre by 1 per radius unit.
        let (from, to) = (screen(&h, 60.0, 60.0), screen(&h, 80.0, 80.0));
        press(&mut h, from, modifiers);
        assert!(h.state().events.is_empty(), "no event before the click slop");
        move_to(&mut h, from + vec2(10.0, 10.0));
        move_to(&mut h, to);
        release(&mut h, to, modifiers);
        let k = kinds(&h);
        assert_eq!(k.first(), Some(&"begin"), "{k:?}");
        assert_eq!(k.last(), Some(&"commit"), "{k:?}");
        let edit = last_value(&h);
        assert_eq!(radii(&edit, 1), rect, "{modifiers:?}");
        assert_eq!(radii(&edit, 2), vec![20.0; 4], "a target without individual always changes all");
        assert!(h.state().hovered);
    }
}

// A press and release without movement reports no event and is claimed.
#[test]
fn corner_click_without_movement_changes_nothing() {
    let mut h = harness(0.0, false);
    let at = screen(&h, 60.0, 60.0);
    press(&mut h, at, Modifiers::NONE);
    release(&mut h, at, Modifiers::NONE);
    assert!(h.state().events.is_empty());
    assert!(h.state().claimed);
    assert!(h.state().activate.is_empty());

    // A press away from every widget is not taken.
    let mut h = harness(0.0, false);
    let away = screen(&h, 150.0, 100.0);
    press(&mut h, away, Modifiers::NONE);
    release(&mut h, away, Modifiers::NONE);
    assert!(!h.state().claimed);
}

// With `kinds(true)` an ⌥-click reports `cycle_kind` with the next kind; a double-click reports
// `activate`.
#[test]
fn corner_alt_click_cycles_the_kind_and_double_click_activates() {
    let corner = CornerRef { target: 1, corner: 0 };
    let mut h = harness(0.0, true);
    let at = screen(&h, 60.0, 60.0);
    press(&mut h, at, Modifiers::ALT);
    release(&mut h, at, Modifiers::ALT);
    assert_eq!(h.state().cycle, Some((corner, CornerKind::InvertedRound)));
    assert!(h.state().events.is_empty());

    let mut h = harness(0.0, false);
    let at = screen(&h, 60.0, 60.0);
    press(&mut h, at, Modifiers::ALT);
    release(&mut h, at, Modifiers::ALT);
    assert_eq!(h.state().cycle, None, "kinds is off");

    let mut h = harness(0.0, false);
    let at = screen(&h, 60.0, 60.0);
    press(&mut h, at, Modifiers::NONE);
    release(&mut h, at, Modifiers::NONE);
    assert!(h.state().activate.is_empty());
    press(&mut h, at, Modifiers::NONE);
    release(&mut h, at, Modifiers::NONE);
    assert_eq!(h.state().activate, [corner]);
}

// With focus: Page Down picks the next widget, the arrows report Sets of ±1 (⇧ 10, ⌥ one corner),
// Enter reports `activate`.
#[test]
fn corner_keyboard_steps_the_radius() {
    let mut h = harness(0.0, false);
    h.key_press(Key::Tab);
    h.run();
    // AccessKit reports the group's picked part as the focused node (its active descendant).
    let focused = h.query_by(|n| n.is_focused()).map(|n| (n.accesskit_node().role(), n.accesskit_node().label().unwrap_or_default()));
    assert_eq!(focused, Some((Role::GraphicsObject, "Corner 1 of 8".to_owned())));
    h.key_press(Key::PageDown);
    h.run();
    h.key_press(Key::ArrowUp);
    h.run();
    assert_eq!(kinds(&h), ["set"], "{:?}", h.state().events);
    let edit = last_value(&h);
    assert_eq!(radii(&edit, 1), vec![1.0; 4]);
    assert_eq!(radii(&edit, 2), vec![1.0; 4]);

    h.key_press_modifiers(Modifiers::SHIFT, Key::ArrowRight);
    h.run();
    assert_eq!(radii(&last_value(&h), 1), vec![11.0; 4]);

    h.key_press_modifiers(Modifiers::ALT, Key::ArrowUp);
    h.run();
    let edit = last_value(&h);
    assert_eq!(radii(&edit, 1), vec![11.0, 12.0, 11.0, 11.0]);
    assert_eq!(radii(&edit, 2), vec![12.0; 4]);

    h.key_press(Key::ArrowDown);
    h.run();
    assert_eq!(radii(&last_value(&h), 1), vec![11.0; 4]);

    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().activate, [CornerRef { target: 1, corner: 1 }]);
}

// Esc during a drag reports Cancel and the rest of the press changes nothing.
#[test]
fn corner_esc_cancels_the_drag() {
    let mut h = harness(0.0, false);
    let (from, to) = (screen(&h, 60.0, 60.0), screen(&h, 90.0, 90.0));
    press(&mut h, from, Modifiers::NONE);
    move_to(&mut h, from + vec2(10.0, 10.0));
    h.key_press(Key::Escape);
    h.run();
    move_to(&mut h, to);
    release(&mut h, to, Modifiers::NONE);
    let k = kinds(&h);
    assert_eq!(k.last(), Some(&"cancel"), "{k:?}");
    assert!(!k.contains(&"commit"), "{k:?}");
    let after_cancel = k.iter().rposition(|k| *k == "cancel").map_or(0, |i| i + 1);
    assert_eq!(k.len(), after_cancel);
}

// The group is named "Corner radius"; each widget is an object "Corner i of n" valued "radius 12
// px"; the readout shows "R: 25.0 px" while dragging.
#[test]
fn corner_accesskit_group_and_values() {
    let mut h = harness(12.0, false);
    let group = h.get_by_label("Corner radius");
    assert_eq!(group.accesskit_node().role(), Role::Group);
    let objects: Vec<(String, String)> = group
        .accesskit_node()
        .children()
        .flat_map(|child| std::iter::once(child).chain(descendants(child)))
        .filter(|n| n.role() == Role::GraphicsObject)
        .map(|n| (n.label().unwrap_or_default(), n.value().unwrap_or_default()))
        .collect();
    assert_eq!(objects.len(), 8);
    assert_eq!(objects[0], ("Corner 1 of 8".to_owned(), "radius 12 px".to_owned()));
    assert_eq!(objects[7], ("Corner 8 of 8".to_owned(), "radius 12 px".to_owned()));

    // The rectangle's top left widget sits at the centre of its arc.
    let from = screen(&h, 62.0, 62.0);
    press(&mut h, from, Modifiers::NONE);
    move_to(&mut h, from + vec2(5.0, 5.0));
    move_to(&mut h, from + vec2(13.0, 13.0));
    h.get_by_label("R: 25.0 px");
}

// A press on a widget during another control's typing run waits for the run's Commit, then the
// drag begins from the press; a click released meanwhile still reports once the run has ended.
#[test]
fn corner_press_during_a_typing_run_waits_for_the_runs_commit() {
    let mut h = harness(0.0, false);
    h.state_mut().typing = true;
    h.run();
    let from = screen(&h, 60.0, 60.0);
    press(&mut h, from, Modifiers::NONE);
    assert!(h.state().claimed, "the widgets hold the press while it waits");
    assert!(h.state().events.is_empty(), "nothing before the run's Commit");
    for i in 0..5 {
        move_to(&mut h, from + vec2(10.0, 10.0 + i as f32));
        let waiting = h.state().committed.is_none_or(|c| c >= h.state().passes);
        assert!(!waiting || h.state().events.is_empty(), "{:?}", h.state().events);
    }
    release(&mut h, from + vec2(10.0, 14.0), Modifiers::NONE);
    assert!(h.state().committed.is_some(), "the run ended");
    let k = kinds(&h);
    assert_eq!(k.first(), Some(&"begin"), "{k:?}");
    assert_eq!(k.last(), Some(&"commit"), "{k:?}");

    let mut h = harness(0.0, false);
    h.state_mut().typing = true;
    h.run();
    let corner = CornerRef { target: 1, corner: 0 };
    press(&mut h, from, Modifiers::NONE);
    release(&mut h, from, Modifiers::NONE);
    press(&mut h, from, Modifiers::NONE);
    release(&mut h, from, Modifiers::NONE);
    for _ in 0..6 {
        h.run();
    }
    assert!(h.state().events.is_empty(), "a click changes nothing");
    assert_eq!(h.state().activate, [corner], "and its double-click activates once");
}
