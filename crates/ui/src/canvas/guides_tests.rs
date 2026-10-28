use egui::accesskit::Role;
use egui::{Color32, Event, Key, Modifiers, PointerButton, Pos2, Sense, Shape, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use kurbo::{Point, Rect};

use super::{Guide, GuideHost, GuidePlace, Guides, Orientation};
use crate::canvas::{CanvasView, RulerUnits};
use crate::edit::{CancelReason, EditEvent};
use crate::localized::LocaleNumbers;
use crate::theme::{Base, current, install};

/// A host with a 400 × 300 canvas showing document (0, 0)–(400, 300) at zoom 1, turned by
/// `rotation`.
struct Host {
    guides: GuideHost,
    rotation: f64,
    zero: Point,
    view: Option<CanvasView>,
    events: Vec<(Option<u64>, EditEvent<GuidePlace>)>,
    clicked: Vec<(u64, Modifiers, Point)>,
    /// A ruler drag to hand to the guides in the next pass.
    ruler: Option<Orientation>,
}

fn harness(guides: Vec<Guide>, rotation: f64) -> Harness<'static, Host> {
    let host = Host { guides: GuideHost::new(guides), rotation, zero: Point::ZERO, view: None, events: Vec::new(), clicked: Vec::new(), ruler: None };
    Harness::<Host>::builder().with_size([480.0, 400.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut Host| {
            ui.add_space(40.0);
            // Not a Tab stop, so Tab goes to the guides.
            let (rect, canvas) = ui.allocate_exact_size(vec2(400.0, 300.0), Sense::click_and_drag() - Sense::FOCUSABLE);
            let view = CanvasView { rotation: host.rotation, ..CanvasView::new(rect, Point::new(200.0, 150.0), 1.0) };
            host.view = Some(view);
            let px = RulerUnits::pixels(LocaleNumbers::from_tag("en"));
            let response = Guides::new("guides", &host.guides.guides)
                .selected(host.guides.selected)
                .interactive(true)
                .bounds(Rect::new(0.0, 0.0, 400.0, 300.0))
                .units(&px, &px)
                .zero(host.zero)
                .show(ui, &canvas, &view);
            if let Some(orientation) = host.ruler.take() {
                Guides::drag_from_ruler(ui.ctx(), response.id, orientation);
            }
            host.events.extend(response.event.clone());
            host.clicked.extend(response.clicked);
            host.guides.apply(&response);
        },
        host,
    )
}

fn view(h: &Harness<'_, Host>) -> CanvasView {
    h.state().view.expect("drawn")
}

fn screen(h: &Harness<'_, Host>, p: Point) -> Pos2 {
    view(h).to_screen(p)
}

fn button(pos: Pos2, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers }
}

/// Moves to `from`, presses, moves to `to` and releases there, with `modifiers` held.
fn drag(h: &mut Harness<'_, Host>, from: Pos2, to: Pos2, modifiers: Modifiers) {
    h.event(Event::ModifiersChanged(modifiers));
    h.event(Event::PointerMoved(from));
    h.event(button(from, true, modifiers));
    h.event(Event::PointerMoved(from.lerp(to, 0.5)));
    h.event(Event::PointerMoved(to));
    h.event(button(to, false, modifiers));
    h.event(Event::ModifiersChanged(Modifiers::NONE));
    h.run();
}

fn click(h: &mut Harness<'_, Host>, at: Pos2) {
    drag(h, at, at, Modifiers::NONE);
}

/// The events' kinds and values, without gesture ids.
fn events(h: &Harness<'_, Host>) -> Vec<(Option<u64>, &'static str, Option<GuidePlace>)> {
    h.state()
        .events
        .iter()
        .map(|(key, e)| {
            let kind = match e {
                EditEvent::Begin { .. } => "begin",
                EditEvent::Update { .. } => "update",
                EditEvent::Commit { .. } => "commit",
                EditEvent::Cancel { .. } => "cancel",
                EditEvent::Set { .. } => "set",
            };
            (*key, kind, e.value().copied())
        })
        .collect()
}

fn last(h: &Harness<'_, Host>) -> Option<(Option<u64>, &'static str, Option<GuidePlace>)> {
    events(h).last().copied()
}

fn place(orientation: Orientation, position: f64) -> Option<GuidePlace> {
    Some(GuidePlace { orientation, position: Some(position) })
}

fn positions(h: &Harness<'_, Host>) -> Vec<(u64, Orientation, f64)> {
    h.state().guides.guides.iter().map(|g| (g.key, g.orientation, g.position)).collect()
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-3
}

#[test]
fn a_guide_takes_the_pointer_within_4_points_in_a_rotated_view() {
    for (offset, takes) in [(3.9, true), (-3.9, true), (4.1, false)] {
        let mut h = harness(vec![Guide::new(1, Orientation::Vertical, 200.0)], 30.0);
        h.run();
        let v = view(&h);
        // Across the guide on screen: document x as the view draws it.
        let across = v.map_vec(kurbo::Vec2::new(1.0, 0.0)).normalized();
        let from = v.to_screen(Point::new(200.0, 100.0)) + across * offset;
        drag(&mut h, from, from + across * 20.0, Modifiers::NONE);
        let moved = events(&h).iter().any(|(key, kind, _)| *key == Some(1) && *kind == "commit");
        assert_eq!(moved, takes, "{offset} pt from the guide");
        if takes {
            let (_, _, position) = positions(&h)[0];
            assert!(near(position, 200.0 + offset as f64 + 20.0), "the guide goes to the pointer: {position}");
        } else {
            assert_eq!(positions(&h), [(1, Orientation::Vertical, 200.0)]);
        }
    }
}

#[test]
fn a_drag_commits_a_drop_on_a_ruler_removes_and_a_click_selects() {
    let mut h = harness(vec![Guide::new(1, Orientation::Vertical, 100.0), Guide::new(2, Orientation::Horizontal, 100.0)], 0.0);
    h.run();
    let (from, to) = (screen(&h, Point::new(100.0, 50.0)), screen(&h, Point::new(150.0, 50.0)));
    drag(&mut h, from, to, Modifiers::NONE);
    let kinds: Vec<_> = events(&h).iter().map(|(key, kind, _)| (*key, *kind)).collect();
    assert_eq!(kinds.first(), Some(&(Some(1), "begin")));
    assert_eq!(last(&h), Some((Some(1), "commit", place(Orientation::Vertical, 150.0))));
    assert_eq!(h.state().guides.selected, Some(1), "a moved guide is selected");

    // A click: no event, the guide is selected and reported.
    let before = events(&h).len();
    let at = screen(&h, Point::new(300.0, 100.0));
    click(&mut h, at);
    assert_eq!(events(&h).len(), before, "a click is no event");
    assert_eq!(h.state().clicked.last().map(|c| c.0), Some(2));
    assert_eq!(h.state().guides.selected, Some(2));

    // Dropped on the top ruler (above the canvas): removed, though it moved only sideways.
    let top = view(&h).rect.top();
    let from = screen(&h, Point::new(300.0, 100.0));
    drag(&mut h, from, Pos2::new(from.x, top - 8.0), Modifiers::NONE);
    assert_eq!(last(&h), Some((Some(2), "commit", Some(GuidePlace { orientation: Orientation::Horizontal, position: None }))));
    assert_eq!(positions(&h), [(1, Orientation::Vertical, 150.0)]);

    // Dropped outside the bounds on the canvas: removed too.
    let mut h = harness(vec![Guide::new(1, Orientation::Vertical, 100.0)], 0.0);
    h.run();
    h.state_mut().guides.selected = Some(1);
    let (from, to) = (screen(&h, Point::new(100.0, 50.0)), screen(&h, Point::new(-20.0, 50.0)));
    drag(&mut h, from, to, Modifiers::NONE);
    assert!(positions(&h).is_empty(), "outside the document's x range");

    // A press elsewhere on the canvas deselects.
    let mut h = harness(vec![Guide::new(1, Orientation::Vertical, 100.0)], 0.0);
    h.state_mut().guides.selected = Some(1);
    h.run();
    let at = screen(&h, Point::new(300.0, 200.0));
    click(&mut h, at);
    assert_eq!(h.state().guides.selected, None);
}

#[test]
fn esc_cancels_a_guide_drag() {
    let mut h = harness(vec![Guide::new(1, Orientation::Vertical, 100.0)], 0.0);
    h.run();
    let from = screen(&h, Point::new(100.0, 50.0));
    h.event(Event::PointerMoved(from));
    h.event(button(from, true, Modifiers::NONE));
    h.event(Event::PointerMoved(from + vec2(40.0, 0.0)));
    h.run();
    assert_eq!(positions(&h), [(1, Orientation::Vertical, 140.0)], "the host shows the drag");
    h.key_press(Key::Escape);
    h.run();
    assert!(matches!(h.state().events.last(), Some((Some(1), EditEvent::Cancel { reason: CancelReason::Escape, .. }))));
    assert_eq!(positions(&h), [(1, Orientation::Vertical, 100.0)]);
    let count = events(&h).len();
    h.event(Event::PointerMoved(from + vec2(60.0, 0.0)));
    h.event(button(from + vec2(60.0, 0.0), false, Modifiers::NONE));
    h.run();
    assert_eq!(events(&h).len(), count, "the rest of the press does nothing");
    assert_eq!(positions(&h), [(1, Orientation::Vertical, 100.0)]);
}

/// Presses over the top ruler and hands the drag to the guides as the ruler would.
fn press_ruler(h: &mut Harness<'_, Host>, orientation: Orientation) -> Pos2 {
    let ruler = Pos2::new(view(h).rect.center().x, view(h).rect.top() - 8.0);
    h.event(Event::PointerMoved(ruler));
    h.event(button(ruler, true, Modifiers::NONE));
    h.state_mut().ruler = Some(orientation);
    h.run();
    ruler
}

#[test]
fn alt_turns_a_guide_out_of_a_ruler() {
    let mut h = harness(Vec::new(), 0.0);
    h.run();
    press_ruler(&mut h, Orientation::Horizontal);
    assert_eq!(events(&h).first().copied(), Some((None, "begin", None)), "the drag begins at the handoff");
    let to = screen(&h, Point::new(120.0, 80.0));
    h.event(Event::PointerMoved(to));
    h.run();
    assert_eq!(last(&h), Some((None, "update", place(Orientation::Horizontal, 80.0))));
    h.event(Event::ModifiersChanged(Modifiers::ALT));
    h.event(Event::PointerMoved(to + vec2(0.0, 1.0)));
    h.run();
    assert_eq!(last(&h), Some((None, "update", place(Orientation::Vertical, 120.0))), "Alt turns it live");
    h.event(button(to, false, Modifiers::ALT));
    h.event(Event::ModifiersChanged(Modifiers::NONE));
    h.run();
    assert_eq!(last(&h), Some((None, "commit", place(Orientation::Vertical, 120.0))));
    assert_eq!(positions(&h), [(1, Orientation::Vertical, 120.0)]);

    // Released back over the ruler: cancelled, nothing added.
    let ruler = press_ruler(&mut h, Orientation::Vertical);
    h.event(Event::PointerMoved(to));
    h.event(Event::PointerMoved(ruler));
    h.event(button(ruler, false, Modifiers::NONE));
    h.run();
    assert_eq!(last(&h).map(|e| e.1), Some("cancel"));
    assert_eq!(positions(&h), [(1, Orientation::Vertical, 120.0)]);
}

#[test]
fn shift_snaps_a_guide_to_the_ruler_ticks_from_the_zero_point() {
    // At zoom 1 pixels are labelled every 100 with ticks every 10, counted from x = 5.
    let mut h = harness(vec![Guide::new(1, Orientation::Vertical, 100.0)], 0.0);
    h.state_mut().zero = Point::new(5.0, 0.0);
    h.run();
    let (from, to) = (screen(&h, Point::new(100.0, 50.0)), screen(&h, Point::new(143.0, 50.0)));
    drag(&mut h, from, to, Modifiers::SHIFT);
    assert_eq!(last(&h), Some((Some(1), "commit", place(Orientation::Vertical, 145.0))));
    let (from, to) = (screen(&h, Point::new(145.0, 50.0)), screen(&h, Point::new(163.0, 50.0)));
    drag(&mut h, from, to, Modifiers::NONE);
    assert_eq!(last(&h), Some((Some(1), "commit", place(Orientation::Vertical, 163.0))), "no snap without Shift");
}

#[test]
fn the_arrow_keys_nudge_the_selected_guide_across_its_axis() {
    let mut h = harness(vec![Guide::new(1, Orientation::Horizontal, 80.0), Guide::new(2, Orientation::Vertical, 100.0)], 0.0);
    h.run();
    h.key_press(Key::Tab);
    h.run();
    assert_eq!(h.state().guides.selected, Some(2), "focus selects the first guide: vertical ones come first");
    h.key_press(Key::ArrowRight);
    h.run();
    assert_eq!(last(&h), Some((Some(2), "set", place(Orientation::Vertical, 101.0))));
    let count = events(&h).len();
    h.key_press(Key::ArrowDown);
    h.run();
    assert_eq!(events(&h).len(), count, "a vertical guide doesn't move up or down");
    h.key_press_modifiers(Modifiers::SHIFT, Key::ArrowLeft);
    h.run();
    assert_eq!(last(&h), Some((Some(2), "set", place(Orientation::Vertical, 91.0))));

    h.key_press(Key::PageDown);
    h.run();
    assert_eq!(h.state().guides.selected, Some(1));
    h.key_press(Key::ArrowDown);
    h.run();
    assert_eq!(last(&h), Some((Some(1), "set", place(Orientation::Horizontal, 81.0))));
    h.key_press(Key::PageUp);
    h.run();
    assert_eq!(h.state().guides.selected, Some(2));
    // Kept inside the bounds.
    for _ in 0..12 {
        h.key_press_modifiers(Modifiers::SHIFT, Key::ArrowLeft);
        h.run();
    }
    assert_eq!(positions(&h), [(1, Orientation::Horizontal, 81.0), (2, Orientation::Vertical, 0.0)]);
}

#[test]
fn delete_removes_the_selected_guide() {
    let mut h = harness(vec![Guide::new(1, Orientation::Vertical, 100.0), Guide::new(2, Orientation::Vertical, 200.0)], 0.0);
    h.run();
    h.key_press(Key::Tab);
    h.run();
    h.key_press(Key::Delete);
    h.run();
    assert_eq!(last(&h), Some((Some(1), "set", Some(GuidePlace { orientation: Orientation::Vertical, position: None }))));
    assert_eq!(positions(&h), [(2, Orientation::Vertical, 200.0)]);
    h.key_press(Key::PageDown);
    h.run();
    h.key_press(Key::Backspace);
    h.run();
    assert!(positions(&h).is_empty());
}

/// The shapes of the last frame in `color`.
fn painted(h: &Harness<'_, Host>, color: Color32) -> usize {
    fn count(shape: &Shape, color: Color32) -> usize {
        match shape {
            Shape::Vec(shapes) => shapes.iter().map(|s| count(s, color)).sum(),
            Shape::Mesh(mesh) => usize::from(!mesh.vertices.is_empty() && mesh.vertices.iter().all(|v| v.color == color)),
            Shape::LineSegment { stroke, .. } => usize::from(stroke.color == color),
            _ => 0,
        }
    }
    h.output().shapes.iter().map(|c| count(&c.shape, color)).sum()
}

#[test]
fn the_guides_are_one_tab_stop_with_focus_lines_after_keyboard_input_only() {
    let mut h = harness(vec![Guide::new(1, Orientation::Vertical, 100.0), Guide::new(2, Orientation::Horizontal, 80.0)], 0.0);
    h.run();
    h.key_press(Key::Tab);
    h.run();
    // The focus reaches the guides' group; AccessKit reports its active descendant.
    assert_eq!(focused(&h), Some((Role::GraphicsObject, "Vertical guide".to_owned())));
    let group = h.get_by_label("Guides");
    let stops = descendants(group.accesskit_node()).into_iter().filter(|n| n.data().supports_action(egui::accesskit::Action::Focus)).count();
    assert_eq!(stops, 0, "the guides are one Tab stop, not one each");
    let focus = super::Guides::style(&current(&h.ctx)).focus.color;
    assert_eq!(painted(&h, focus), 2, "two lines beside the selected guide");
    let at = screen(&h, Point::new(100.0, 200.0));
    click(&mut h, at);
    assert_eq!(focused(&h), Some((Role::GraphicsObject, "Vertical guide".to_owned())), "a press on a guide keeps the focus on the guides");
    assert_eq!(painted(&h, focus), 0, "no focus lines after the pointer");
    h.key_press(Key::PageDown);
    h.run();
    assert_eq!(painted(&h, focus), 2);
}

#[test]
fn guides_are_graphics_objects_in_a_group() {
    let mut h =
        harness(vec![Guide::new(1, Orientation::Vertical, 100.0), Guide { locked: true, ..Guide::new(2, Orientation::Horizontal, 80.5) }], 0.0);
    h.state_mut().guides.selected = Some(1);
    h.run();
    let group = h.get_by_label("Guides");
    assert_eq!(group.accesskit_node().role(), Role::Group);
    let objects: Vec<_> = descendants(group.accesskit_node())
        .into_iter()
        .filter(|n| n.role() == Role::GraphicsObject)
        .map(|n| (n.label().unwrap_or_default(), n.value().unwrap_or_default(), n.data().is_selected()))
        .collect();
    assert_eq!(
        objects,
        [("Vertical guide".to_owned(), "100.0 px".to_owned(), Some(true)), ("Horizontal guide".to_owned(), "80.5 px".to_owned(), Some(false))]
    );
    assert_eq!(group.accesskit_node().active_descendant().and_then(|n| n.label()).as_deref(), Some("Vertical guide"));
}

fn descendants(node: egui_kittest::kittest::AccessKitNode<'_>) -> Vec<egui_kittest::kittest::AccessKitNode<'_>> {
    node.children().flat_map(|child| std::iter::once(child).chain(descendants(child))).collect()
}

/// The role and name of the node AccessKit reports focused.
fn focused(h: &Harness<'_, Host>) -> Option<(Role, String)> {
    h.query_by(|n| n.is_focused()).map(|n| (n.accesskit_node().role(), n.accesskit_node().label().unwrap_or_default()))
}
