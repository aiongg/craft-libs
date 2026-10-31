use egui::accesskit::Role;
use egui::{Color32, Event, Key, Modifiers, PointerButton, Pos2, Sense, Shape, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use kurbo::{Point, Rect};

use super::{Guide, GuideHost, GuideNames, GuidePlace, Guides, Orientation};
use crate::canvas::{CanvasView, RulerUnits};
use crate::edit::{CancelReason, EditEvent};
use crate::localized::LocaleNumbers;
use crate::testing::TypingField;
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
    interactive: bool,
    /// The host skips the guides (another tool, guides hidden).
    hidden: bool,
    field: TypingField,
    /// A text field that holds the keyboard while it is `Some`.
    text: Option<String>,
    modifiers: Option<Modifiers>,
}

fn harness(guides: Vec<Guide>, rotation: f64) -> Harness<'static, Host> {
    let host = Host {
        guides: GuideHost::new(guides),
        rotation,
        zero: Point::ZERO,
        view: None,
        events: Vec::new(),
        clicked: Vec::new(),
        ruler: None,
        interactive: true,
        hidden: false,
        field: TypingField::default(),
        text: None,
        modifiers: None,
    };
    Harness::<Host>::builder().with_size([480.0, 400.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut Host| {
            host.field.show(ui);
            if let Some(text) = &mut host.text {
                let field = ui.add(egui::TextEdit::singleline(text));
                field.widget_info(|| egui::WidgetInfo::labeled(egui::accesskit::Role::TextInput, true, "Name"));
                field.request_focus();
            }
            ui.add_space(40.0);
            // Not a Tab stop, so Tab goes to the guides.
            let (rect, canvas) = ui.allocate_exact_size(vec2(400.0, 300.0), Sense::click_and_drag() - Sense::FOCUSABLE);
            let view = CanvasView { rotation: host.rotation, ..CanvasView::new(rect, Point::new(200.0, 150.0), 1.0) };
            host.view = Some(view);
            if host.hidden {
                return;
            }
            let px = RulerUnits::pixels(LocaleNumbers::from_tag("en"));
            let names = GuideNames::new("Guides", "Vertical guide", "Horizontal guide");
            let mut guides = Guides::new("guides", &host.guides.guides, &px, &px, names);
            if let Some(m) = host.modifiers {
                guides = guides.modifiers(m);
            }
            let response = guides
                .selected(host.guides.selected)
                .interactive(host.interactive)
                .bounds(Rect::new(0.0, 0.0, 400.0, 300.0))
                .zero(host.zero)
                .readout("X", "Y")
                .show(ui, &canvas, &view);
            if let Some(orientation) = host.ruler.take() {
                Guides::drag_from_ruler(ui.ctx(), response.id, orientation);
            }
            host.field.note(response.event.is_some());
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
    h.run();
    // The ruler hands the drag over once it starts; the guides take it in their next pass.
    h.state_mut().ruler = Some(orientation);
    h.step();
    h.step();
    ruler
}

#[test]
fn alt_turns_a_guide_out_of_a_ruler() {
    let mut h = harness(Vec::new(), 0.0);
    h.run();
    press_ruler(&mut h, Orientation::Horizontal);
    assert_eq!(events(&h).first().map(|e| e.1), Some("begin"), "the drag begins at the handoff");
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

/// The first pass of a ruler drag already carries the guide's position, and the readout shows it.
#[test]
fn a_ruler_drag_begins_with_its_position_and_shows_the_readout() {
    let mut h = harness(Vec::new(), 0.0);
    h.run();
    let ruler = Pos2::new(view(&h).rect.center().x, view(&h).rect.top() - 8.0);
    h.event(Event::PointerMoved(ruler));
    h.event(button(ruler, true, Modifiers::NONE));
    h.run();
    h.state_mut().ruler = Some(Orientation::Horizontal);
    h.event(Event::PointerMoved(screen(&h, Point::new(120.0, 80.0))));
    h.step();
    h.step();
    let (key, kind, value) = events(&h).first().copied().expect("the first pass reports");
    assert_eq!((key, kind), (None, "begin"));
    assert_eq!(value, place(Orientation::Horizontal, 80.0), "Begin carries the position");
    h.run();
    let _ = h.get_by_label("Y 80 px");
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

#[test]
fn delete_acts_while_the_canvas_has_the_pointer_unless_a_text_field_has_the_keys() {
    let mut h = harness(vec![Guide::new(1, Orientation::Vertical, 100.0), Guide::new(2, Orientation::Vertical, 200.0)], 0.0);
    h.state_mut().guides.selected = Some(2);
    h.run();
    // No focus on the guides, the pointer outside the canvas: the key is not for them.
    h.event(Event::PointerMoved(Pos2::new(1.0, 1.0)));
    h.run();
    h.key_press(Key::Delete);
    h.run();
    assert_eq!(positions(&h).len(), 2);
    // Over the canvas, off the guides.
    let over = screen(&h, Point::new(150.0, 150.0));
    h.event(Event::PointerMoved(over));
    h.run();
    h.state_mut().text = Some(String::new());
    h.run();
    h.key_press(Key::Delete);
    h.run();
    assert_eq!(positions(&h).len(), 2, "a text field has the keys");
    h.state_mut().text = None;
    h.run();
    h.key_press(Key::Delete);
    h.run();
    assert_eq!(last(&h), Some((Some(2), "set", Some(GuidePlace { orientation: Orientation::Vertical, position: None }))));
    assert_eq!(positions(&h), [(1, Orientation::Vertical, 100.0)]);
    assert_eq!(h.state().guides.selected, Some(1));
}

#[test]
fn the_drag_readout_drops_trailing_zeros() {
    let mut h = harness(vec![Guide::new(1, Orientation::Vertical, 100.0)], 0.0);
    h.run();
    let from = screen(&h, Point::new(100.0, 200.0));
    h.event(Event::PointerMoved(from));
    h.event(button(from, true, Modifiers::NONE));
    h.run();
    for (x, text) in [(103.0, "X 103 px"), (103.5, "X 103.5 px")] {
        h.event(Event::PointerMoved(screen(&h, Point::new(x, 200.0))));
        h.run();
        let _ = h.get_by_label(text);
    }
}

/// The shapes of the last frame in `color`.
fn painted(h: &Harness<'_, Host>, color: Color32) -> usize {
    fn count(shape: &Shape, color: Color32) -> usize {
        match shape {
            Shape::Vec(shapes) => shapes.iter().map(|s| count(s, color)).sum(),
            Shape::Mesh(mesh) => usize::from(!mesh.vertices.is_empty() && mesh.vertices.iter().all(|v| v.color == color)),
            Shape::LineSegment { stroke, .. } => usize::from(stroke.color == color),
            Shape::Rect(rect) => usize::from(rect.stroke.color == color && rect.stroke.width > 0.0),
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

/// Queues `events` into one frame, as winit delivers a quick gesture, and runs it.
fn frame(h: &mut Harness<'_, Host>, events: impl IntoIterator<Item = Event>) {
    h.input_mut().events.extend(events);
    h.step();
}

#[test]
fn a_quick_drag_in_one_frame_reports_under_the_guide_it_took() {
    let mut h = harness(vec![Guide::new(1, Orientation::Vertical, 100.0), Guide::new(2, Orientation::Horizontal, 100.0)], 0.0);
    h.state_mut().guides.selected = Some(2);
    h.run();
    let from = screen(&h, Point::new(100.0, 200.0));
    let to = from + vec2(40.0, 0.0);
    frame(
        &mut h,
        [Event::PointerMoved(from), button(from, true, Modifiers::NONE), Event::PointerMoved(from + vec2(20.0, 0.0)), Event::PointerMoved(to)],
    );
    frame(&mut h, [button(to, false, Modifiers::NONE)]);
    h.run();
    assert!(events(&h).iter().all(|(key, ..)| *key == Some(1)), "{:?}", events(&h));
    assert_eq!(last(&h), Some((Some(1), "commit", place(Orientation::Vertical, 140.0))));
    assert_eq!(positions(&h), [(1, Orientation::Vertical, 140.0), (2, Orientation::Horizontal, 100.0)]);

    // Press, moves and release in one frame.
    h.state_mut().events.clear();
    h.state_mut().guides.selected = Some(2);
    h.run();
    let (from, to) = (screen(&h, Point::new(140.0, 200.0)), screen(&h, Point::new(170.0, 200.0)));
    frame(&mut h, [Event::PointerMoved(from), button(from, true, Modifiers::NONE), Event::PointerMoved(to), button(to, false, Modifiers::NONE)]);
    h.run();
    assert!(events(&h).iter().all(|(key, ..)| *key == Some(1)), "{:?}", events(&h));
    assert_eq!(positions(&h), [(1, Orientation::Vertical, 170.0), (2, Orientation::Horizontal, 100.0)]);
}

#[test]
fn a_ruler_drag_the_guides_missed_the_end_of_does_nothing_later() {
    let mut h = harness(vec![Guide::new(1, Orientation::Vertical, 100.0)], 0.0);
    h.run();
    // The ruler hands a drag over; the host hides the guides before they show again.
    let ruler = Pos2::new(view(&h).rect.center().x, view(&h).rect.top() - 8.0);
    frame(&mut h, [Event::PointerMoved(ruler), button(ruler, true, Modifiers::NONE)]);
    h.state_mut().ruler = Some(Orientation::Horizontal);
    frame(&mut h, []);
    h.state_mut().hidden = true;
    frame(&mut h, [button(ruler, false, Modifiers::NONE)]);
    h.run();
    h.state_mut().hidden = false;
    let over = screen(&h, Point::new(200.0, 150.0));
    frame(&mut h, [Event::PointerMoved(over)]);
    h.run();
    assert!(events(&h).is_empty(), "{:?}", events(&h));
    assert_eq!(positions(&h), [(1, Orientation::Vertical, 100.0)]);

    // A guide drag whose release the guides missed: over, and the next press is free.
    let from = screen(&h, Point::new(100.0, 200.0));
    frame(&mut h, [Event::PointerMoved(from), button(from, true, Modifiers::NONE), Event::PointerMoved(from + vec2(30.0, 0.0))]);
    h.state_mut().hidden = true;
    frame(&mut h, [button(from + vec2(30.0, 0.0), false, Modifiers::NONE)]);
    let other = screen(&h, Point::new(300.0, 250.0));
    frame(&mut h, [Event::PointerMoved(other), button(other, true, Modifiers::NONE)]);
    h.state_mut().hidden = false;
    let before = events(&h).len();
    frame(&mut h, [Event::PointerMoved(other + vec2(5.0, 0.0))]);
    frame(&mut h, [button(other + vec2(5.0, 0.0), false, Modifiers::NONE)]);
    h.run();
    assert_eq!(events(&h).len(), before, "no event after the missed release: {:?}", events(&h));
}

#[test]
fn shift_snaps_a_guide_out_of_a_ruler_while_alt_turns_it() {
    let mut h = harness(Vec::new(), 0.0);
    h.run();
    press_ruler(&mut h, Orientation::Horizontal);
    let to = screen(&h, Point::new(143.0, 80.0));
    frame(&mut h, [Event::ModifiersChanged(Modifiers::SHIFT | Modifiers::ALT), Event::PointerMoved(to)]);
    frame(&mut h, [button(to, false, Modifiers::SHIFT | Modifiers::ALT)]);
    frame(&mut h, [Event::ModifiersChanged(Modifiers::NONE)]);
    h.run();
    assert_eq!(positions(&h), [(1, Orientation::Vertical, 140.0)]);
}

#[test]
fn a_press_off_the_guides_deselects_whatever_the_tool_and_one_on_a_guide_keeps_it() {
    let mut h = harness(vec![Guide::new(1, Orientation::Vertical, 100.0)], 0.0);
    h.state_mut().guides.selected = Some(1);
    h.state_mut().interactive = false;
    h.run();
    let on = screen(&h, Point::new(100.0, 200.0));
    click(&mut h, on);
    assert_eq!(h.state().guides.selected, Some(1), "a brush press on the guide");
    assert!(events(&h).is_empty(), "takes nothing");
    let off = screen(&h, Point::new(250.0, 200.0));
    click(&mut h, off);
    assert_eq!(h.state().guides.selected, None, "a brush press elsewhere");
}

#[test]
fn the_focus_stays_visible_after_delete_and_on_a_guide_out_of_view() {
    let mut h = harness(
        vec![
            Guide::new(1, Orientation::Vertical, 100.0),
            Guide::new(2, Orientation::Vertical, 200.0),
            Guide::new(3, Orientation::Horizontal, 1000.0),
        ],
        0.0,
    );
    h.run();
    h.key_press(Key::Tab);
    h.run();
    let style = super::Guides::style(&current(&h.ctx));
    assert_eq!(h.state().guides.selected, Some(1));
    assert_eq!(painted(&h, style.focus.color), 2, "two lines beside guide 1");
    assert_eq!(painted(&h, style.focus_halo.color), 2, "each on a halo");
    h.key_press(Key::Delete);
    h.run();
    assert_eq!(h.state().guides.selected, Some(2), "Delete selects the next guide");
    assert_eq!(painted(&h, style.focus.color), 2, "and its focus lines show");
    h.key_press(Key::PageDown);
    h.run();
    assert_eq!(h.state().guides.selected, Some(3));
    assert_eq!(painted(&h, style.focus.color), 1, "the guide is out of view: the ring goes round the canvas");
    h.key_press(Key::Delete);
    h.run();
    assert_eq!(h.state().guides.selected, Some(2), "the last one deleted: the previous");
}

#[test]
fn guide_nodes_follow_the_keyboard_order() {
    let mut h = harness(
        vec![
            Guide::new(1, Orientation::Horizontal, 50.0),
            Guide { locked: true, ..Guide::new(2, Orientation::Vertical, 300.0) },
            Guide::new(3, Orientation::Vertical, 100.0),
            Guide::new(4, Orientation::Horizontal, 20.0),
        ],
        0.0,
    );
    h.run();
    let group = h.get_by_label("Guides");
    let values: Vec<String> =
        descendants(group.accesskit_node()).into_iter().filter(|n| n.role() == Role::GraphicsObject).map(|n| n.value().unwrap_or_default()).collect();
    assert_eq!(values, ["100.0 px", "300.0 px", "20.0 px", "50.0 px"]);
}

/// A press on a guide during a text field's typing run begins after the run's Commit and the
/// guide still goes where the pointer is released.
#[test]
fn a_press_during_a_typing_run_begins_after_the_runs_commit() {
    let mut h = harness(vec![Guide::new(1, Orientation::Vertical, 200.0)], 0.0);
    h.state_mut().field.on = true;
    h.step();
    let from = screen(&h, Point::new(200.0, 100.0));
    h.event(Event::PointerMoved(from));
    h.step();
    h.event(button(from, true, Modifiers::NONE));
    h.step();
    assert!(h.state().events.is_empty(), "the press waits: {:?}", h.state().events);
    for i in 1..=6 {
        h.event(Event::PointerMoved(from + vec2(5.0 * i as f32, 0.0)));
        h.step();
    }
    h.event(button(from + vec2(30.0, 0.0), false, Modifiers::NONE));
    h.step();
    h.step();
    assert!(matches!(events(&h).first(), Some((Some(1), "begin", _))), "{:?}", events(&h));
    assert!(h.state().field.commit_came_first());
    assert!(near(positions(&h)[0].2, 230.0), "{:?}", positions(&h));
}

#[test]
fn modifiers_override_replaces_the_input_modifiers_in_the_guides() {
    // Shift snaps to ticks counted from x = 5; the override decides, not the keys held.
    for (input, held, end) in [(Modifiers::NONE, Modifiers::SHIFT, 145.0), (Modifiers::SHIFT, Modifiers::NONE, 143.0)] {
        let mut h = harness(vec![Guide::new(1, Orientation::Vertical, 100.0)], 0.0);
        h.state_mut().zero = Point::new(5.0, 0.0);
        h.state_mut().modifiers = Some(held);
        h.run();
        let (from, to) = (screen(&h, Point::new(100.0, 50.0)), screen(&h, Point::new(143.0, 50.0)));
        drag(&mut h, from, to, input);
        assert_eq!(last(&h), Some((Some(1), "commit", place(Orientation::Vertical, end))), "input {input:?}, override {held:?}");
    }
}

/// The colours of the 1 point line shapes one frame of `guides` paints.
fn smart_colors(guides: impl Fn() -> super::SmartGuides<'static> + 'static) -> Vec<Color32> {
    let h = Harness::builder().with_size([400.0, 300.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui(move |ui| {
        let view = CanvasView::new(ui.max_rect(), Point::new(200.0, 150.0), 1.0);
        guides().show(ui, &view);
    });
    let mut out = Vec::new();
    fn walk(shape: &Shape, out: &mut Vec<Color32>) {
        match shape {
            Shape::Vec(v) => v.iter().for_each(|s| walk(s, out)),
            Shape::LineSegment { stroke, .. } => out.push(stroke.color),
            Shape::Path(p) => {
                if let egui::epaint::ColorMode::Solid(c) = p.stroke.color {
                    out.push(c);
                }
            }
            Shape::Mesh(m) => out.extend(m.vertices.iter().map(|v| v.color).take(1)),
            _ => {}
        }
    }
    h.output().shapes.iter().for_each(|s| walk(&s.shape, &mut out));
    out
}

static SMART_LINES: [super::SmartLine; 1] = [super::SmartLine { a: Point::new(50.0, 50.0), b: Point::new(150.0, 50.0), extension: false }];

#[test]
fn smart_guides_take_the_apps_colour_and_a_moving_bounds_outline() {
    let token = current_smart();
    assert!(smart_colors(|| super::SmartGuides::new(&SMART_LINES, &[])).contains(&token));
    let pink = Color32::from_rgb(10, 200, 90);
    let with = smart_colors(move || super::SmartGuides::new(&SMART_LINES, &[]).color(pink));
    assert!(with.contains(&pink) && !with.contains(&token), "{with:?}");
    let outlined = smart_colors(|| super::SmartGuides::new(&SMART_LINES, &[]).bounds(Rect::new(10.0, 10.0, 90.0, 90.0)));
    assert_eq!(outlined.len(), 2, "the outline and the line: {outlined:?}");
    assert_ne!(outlined[0], token);
}

fn current_smart() -> Color32 {
    let h = Harness::builder().with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui(|_| {});
    super::Guides::style(&current(&h.ctx)).smart
}
