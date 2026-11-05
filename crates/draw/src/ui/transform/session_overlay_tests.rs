//! Tests of the session overlay and the box's hidden reference point (task TR-T8).

use egui::accesskit::Role;
use egui::epaint::ColorMode;
use egui::{Color32, Event, Id, IdSalt, Key, Modifiers, PointerButton, Pos2, Sense, Shape, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use kurbo::{Point, Rect};

use super::{TransformBox, TransformEdit, TransformNames, TransformOverlay, TransformPhase, TransformSession, TransformSettings};
use craft_ui::canvas::quad::{BoxPart, BoxState};
use craft_ui::canvas::{CanvasView, RulerUnits};
use craft_ui::localized::{Labels as _, LocaleNumbers};
use craft_ui::theme::{Base, current, install};

const RECT: Rect = Rect::new(100.0, 80.0, 300.0, 220.0);

struct Host {
    session: TransformSession<()>,
    view: Option<CanvasView>,
    edits: Vec<&'static str>,
    hovered: Option<BoxPart>,
    claimed: bool,
    picked: Option<BoxPart>,
    pass: f64,
    id: Option<Id>,
    expected_id: Option<Id>,
}

fn harness(shown: bool, setup: impl FnOnce(&mut TransformSession<()>)) -> Harness<'static, Host> {
    let mut session = TransformSession::new(RECT).settings(&TransformSettings { show_reference_point: shown, ..TransformSettings::default() });
    setup(&mut session);
    let host = Host { session, view: None, edits: Vec::new(), hovered: None, claimed: false, picked: None, pass: 0.0, id: None, expected_id: None };
    Harness::<Host>::builder().with_size([480.0, 400.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut Host| {
            ui.add_space(40.0);
            let (rect, canvas) = ui.allocate_exact_size(vec2(400.0, 300.0), Sense::click_and_drag() - Sense::FOCUSABLE);
            let view = CanvasView::new(rect, Point::new(200.0, 150.0), 1.0);
            host.view = Some(view);
            let px = RulerUnits::pixels(LocaleNumbers::from_tag("en"));
            let session = &host.session;
            let r = TransformOverlay::new("overlay", session).names(TransformNames::english()).units(&px, &px).show(ui, &canvas, &view);
            let mut edits = Vec::new();
            for edit in r.edits {
                edits.push(kind(&edit));
                host.session.apply(edit, host.pass);
            }
            if r.apply {
                edits.push("commit");
            }
            if r.discard {
                edits.push("cancel");
            }
            host.edits.extend(edits);
            host.id = Some(r.id);
            host.expected_id = Some(ui.make_persistent_id(IdSalt::new("overlay")));
            (host.hovered, host.claimed, host.picked) = (r.hovered, r.claimed, r.picked);
            host.pass += 1.0;
        },
        host,
    )
}

fn kind(edit: &TransformEdit<()>) -> &'static str {
    use craft_ui::edit::EditEvent;
    match edit {
        TransformEdit::Box(EditEvent::Begin { .. }) => "begin",
        TransformEdit::Box(EditEvent::Update { .. }) => "update",
        TransformEdit::Box(EditEvent::Commit { .. }) => "end",
        TransformEdit::Box(EditEvent::Cancel { .. }) => "cancel drag",
        TransformEdit::Box(EditEvent::Set { .. }) => "set",
        _ => "other",
    }
}

fn screen(h: &Harness<'_, Host>, p: Point) -> Pos2 {
    h.state().view.expect("drawn").to_screen(p)
}

fn button(pos: Pos2, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers }
}

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

/// One frame with `events`.
fn frame(h: &mut Harness<'_, Host>, events: impl IntoIterator<Item = Event>) {
    h.input_mut().events.extend(events);
    h.step();
}

fn painted(h: &Harness<'_, Host>, color: Color32) -> usize {
    fn count(shape: &Shape, color: Color32) -> usize {
        match shape {
            Shape::Vec(shapes) => shapes.iter().map(|s| count(s, color)).sum(),
            Shape::Path(path) => usize::from(path.stroke.color == ColorMode::Solid(color)),
            Shape::Circle(circle) => usize::from(circle.stroke.color == color && circle.stroke.width > 0.0),
            Shape::LineSegment { stroke, .. } => usize::from(stroke.color == color),
            _ => 0,
        }
    }
    h.output().shapes.iter().map(|c| count(&c.shape, color)).sum()
}

fn descendants(node: egui_kittest::kittest::AccessKitNode<'_>) -> Vec<egui_kittest::kittest::AccessKitNode<'_>> {
    node.children().flat_map(|child| std::iter::once(child).chain(descendants(child))).collect()
}

// A corner drag on the overlay reports Begin, Update and Commit as TransformEdit::Box.
#[test]
fn the_overlay_turns_box_events_into_session_edits() {
    let mut h = harness(false, |_| {});
    h.run();
    let corner = screen(&h, Point::new(300.0, 220.0));
    drag(&mut h, corner, corner + vec2(50.0, 40.0), Modifiers::NONE);
    let kinds = &h.state().edits;
    assert_eq!(kinds.first(), Some(&"begin"));
    assert_eq!(kinds.last(), Some(&"end"));
    assert!(kinds.contains(&"update"));
    assert_ne!(h.state().session.state().quad, BoxState::from_rect(RECT).quad, "the session took the edits");
}

// With `reference_point(false)` the box draws no pivot glyph, a press on the pivot's place hits
// the inside, Page keys skip it and AccessKit lists 9 parts; ⌥-press away from handles does
// nothing.
#[test]
fn a_hidden_reference_point_is_not_drawn_hit_or_picked() {
    let mut shown = harness(true, |_| {});
    shown.run();
    let pivot_colour = TransformBox::style(&current(&shown.ctx)).pivot.color;
    assert!(painted(&shown, pivot_colour) > 0, "the glyph when shown");

    let mut h = harness(false, |_| {});
    h.run();
    assert_eq!(painted(&h, pivot_colour), 0, "no glyph");
    let centre = screen(&h, Point::new(200.0, 150.0));
    h.event(Event::PointerMoved(centre));
    h.run();
    assert_eq!(h.state().hovered, Some(BoxPart::Inside), "the pivot's place is the inside");

    // Away from the handles, ⌥ places nothing and moves nothing.
    let away = screen(&h, Point::new(150.0, 110.0));
    drag(&mut h, away, away + vec2(30.0, 20.0), Modifiers::ALT);
    assert!(h.state().edits.is_empty());
    assert_eq!(h.state().session.state().pivot, Point::new(200.0, 150.0));

    h.key_press(Key::Tab);
    h.run();
    for _ in 0..12 {
        h.key_press(Key::PageDown);
        h.run();
        assert_ne!(h.state().picked, Some(BoxPart::Pivot));
    }
    let group = h.get_by_label("Free Transform");
    let parts = descendants(group.accesskit_node()).into_iter().filter(|n| n.role() == Role::GraphicsObject).count();
    assert_eq!(parts, 9);
}

// In the Warp phase the overlay paints nothing, claims no press and reports no edit.
#[test]
fn in_the_warp_phase_the_overlay_claims_nothing() {
    let mut h = harness(true, |s| {
        s.apply(TransformEdit::Warp(()), 0.0);
    });
    assert_eq!(h.state().session.phase(), TransformPhase::Warp);
    h.run();
    let line = TransformBox::style(&current(&h.ctx)).line.color;
    assert_eq!(painted(&h, line), 0);
    let corner = screen(&h, Point::new(300.0, 220.0));
    drag(&mut h, corner, corner + vec2(50.0, 40.0), Modifiers::NONE);
    assert!(h.state().edits.is_empty());
    assert!(!h.state().claimed);
    assert_eq!(h.state().session.state().quad, BoxState::from_rect(RECT).quad);
}

// Enter, a click outside and a double click inside each report `apply`; Esc with no drag reports
// `discard`.
#[test]
fn enter_applies_a_click_outside_applies_a_double_click_inside_applies() {
    let mut h = harness(false, |_| {});
    h.run();
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().edits, ["commit"]);

    let mut h = harness(false, |_| {});
    h.run();
    let outside = screen(&h, Point::new(380.0, 280.0));
    drag(&mut h, outside, outside, Modifiers::NONE);
    assert_eq!(h.state().edits, ["commit"], "a click outside");

    let mut h = harness(false, |_| {});
    h.run();
    let inside = screen(&h, Point::new(150.0, 110.0));
    frame(&mut h, [Event::PointerMoved(inside), button(inside, true, Modifiers::NONE), button(inside, false, Modifiers::NONE)]);
    frame(&mut h, [button(inside, true, Modifiers::NONE), button(inside, false, Modifiers::NONE)]);
    h.run();
    assert_eq!(h.state().edits, ["commit"], "a double click inside");

    let mut h = harness(false, |_| {});
    h.run();
    h.key_press(Key::Escape);
    h.run();
    assert_eq!(h.state().edits, ["cancel"]);
}

// Tab reaches the box; Page Down walks whole box, corners, edges (and the pivot when shown);
// arrows move the picked part and report Set.
#[test]
fn every_overlay_part_is_reachable_from_the_keyboard() {
    let mut h = harness(true, |_| {});
    h.run();
    h.key_press(Key::Tab);
    h.run();
    let mut walked = Vec::new();
    for _ in 0..9 {
        h.key_press(Key::PageDown);
        h.run();
        walked.push(h.state().picked.expect("a part is picked"));
    }
    let expected = [0, 1, 2, 3].map(BoxPart::Corner).into_iter().chain([0, 1, 2, 3].map(BoxPart::Edge)).chain([BoxPart::Pivot]).collect::<Vec<_>>();
    assert_eq!(walked, expected);

    h.key_press(Key::ArrowRight);
    h.run();
    assert_eq!(h.state().edits, ["set"]);
    assert_eq!(h.state().session.state().pivot.x, 201.0, "the picked reference point moved");
}

// The response id is the host's `make_persistent_id` of the overlay's salt in both phases.
#[test]
fn the_response_id_is_the_same_in_the_box_and_warp_phases() {
    let mut boxed = harness(false, |_| {});
    boxed.run();
    let mut warping = harness(false, |s| {
        s.apply(TransformEdit::Warp(()), 0.0);
    });
    warping.run();
    assert_eq!(warping.state().session.phase(), TransformPhase::Warp);
    let (a, b) = (boxed.state(), warping.state());
    assert!(a.id.is_some() && a.id == a.expected_id, "the box phase");
    assert_eq!(b.id, b.expected_id, "the warp phase");
    assert_eq!(a.id, b.id);
}
