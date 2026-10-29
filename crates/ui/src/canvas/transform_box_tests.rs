use egui::accesskit::Role;
use egui::epaint::ColorMode;
use egui::{Color32, Event, Key, Modifiers, PointerButton, Pos2, Sense, Shape, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use kurbo::{Point, Rect};

use super::{BoxHost, RotateZone, TransformBox, TransformLabels, english_names};
use crate::canvas::guides::GuideHost;
use crate::canvas::quad::{self, BoxPart, BoxRules, BoxState, TransformMode};
use crate::canvas::{CanvasView, Guide, GuideNames, Guides, Orientation, RulerUnits};
use crate::edit::{CancelReason, EditEvent, EditPolicy, Gesture};
use crate::localized::LocaleNumbers;
use crate::theme::{Base, current, install};

/// The box before any drag.
const RECT: Rect = Rect::new(100.0, 80.0, 300.0, 220.0);

/// A host with a 400 × 300 canvas showing document (0, 0)–(400, 300) at zoom 1, turned by
/// `rotation` and mirrored by `flip`, with an open box and optionally guides under it.
struct Host {
    host: BoxHost,
    rules: BoxRules,
    zone: RotateZone,
    inside_moves: bool,
    rotation: f64,
    flip: bool,
    view: Option<CanvasView>,
    events: Vec<EditEvent<BoxState>>,
    applied: usize,
    discarded: usize,
    picked: Option<BoxPart>,
    hovered: Option<BoxPart>,
    claimed: bool,
    guides: Option<GuideHost>,
    guide_events: usize,
    /// A text field's typing run beside the canvas: its gesture is begun in the first pass, has
    /// the focus until something takes it, and reports its events here.
    typing: bool,
    typing_events: Vec<EditEvent<i32>>,
    /// Passes drawn, and the pass of each event of the box and of the field.
    pass: usize,
    box_passes: Vec<usize>,
    typing_passes: Vec<usize>,
    /// The modifiers the box reads in place of the input state.
    modifiers: Option<Modifiers>,
    /// The box rounds its corners to whole units; the host stores the emitted box unchanged.
    pixel_grid: bool,
}

fn harness_with(state: BoxState, rotation: f64, flip: bool, setup: impl FnOnce(&mut Host)) -> Harness<'static, Host> {
    let mut host = Host {
        host: BoxHost::new(state),
        rules: BoxRules::default(),
        zone: RotateZone::Outside,
        inside_moves: true,
        rotation,
        flip,
        view: None,
        events: Vec::new(),
        applied: 0,
        discarded: 0,
        picked: None,
        hovered: None,
        claimed: false,
        guides: None,
        guide_events: 0,
        typing: false,
        typing_events: Vec::new(),
        pass: 0,
        box_passes: Vec::new(),
        typing_passes: Vec::new(),
        modifiers: None,
        pixel_grid: false,
    };
    setup(&mut host);
    Harness::<Host>::builder().with_size([480.0, 400.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut Host| {
            host.pass += 1;
            if host.typing {
                let field = ui.make_persistent_id("typing field");
                let first = host.pass == 1;
                let response = ui.interact(egui::Rect::from_min_size(Pos2::ZERO, vec2(20.0, 20.0)), field, Sense::click());
                if first {
                    response.request_focus();
                }
                let mut gesture = Gesture::load(ui, field, &0, EditPolicy::default());
                if first {
                    gesture.begin();
                }
                if let Some(event) = gesture.finish() {
                    host.typing_events.push(event);
                    host.typing_passes.push(host.pass);
                }
            }
            ui.add_space(40.0);
            // Not a Tab stop, so Tab goes to the box.
            let (rect, canvas) = ui.allocate_exact_size(vec2(400.0, 300.0), Sense::click_and_drag() - Sense::FOCUSABLE);
            let view = CanvasView { rotation: host.rotation, flip_x: host.flip, ..CanvasView::new(rect, Point::new(200.0, 150.0), 1.0) };
            host.view = Some(view);
            let px = RulerUnits::pixels(LocaleNumbers::from_tag("en"));
            let state = host.host.state;
            let mut b = TransformBox::new("box", &state, english_names());
            if let Some(m) = host.modifiers {
                b = b.modifiers(m);
            }
            let r = b
                .pixel_grid(host.pixel_grid)
                .rules(host.rules)
                .rotate_zone(host.zone)
                .inside_moves(host.inside_moves)
                .units(&px, &px)
                .readout(TransformLabels::new("Angle:", "W:", "H:", "ΔX:", "ΔY:"))
                .start(RECT, 0.0)
                .value("W 100%")
                .show(ui, &canvas, &view);
            host.host.apply(&r);
            host.box_passes.extend(r.event.iter().map(|_| host.pass));
            host.events.extend(r.event.clone());
            host.applied += usize::from(r.apply);
            host.discarded += usize::from(r.discard);
            (host.picked, host.hovered, host.claimed) = (r.picked, r.hovered, r.claimed);
            if let Some(guides) = &mut host.guides {
                let names = GuideNames::new("Guides", "Vertical guide", "Horizontal guide");
                let g = Guides::new("guides", &guides.guides, &px, &px, names).interactive(true).show(ui, &canvas, &view);
                host.guide_events += usize::from(g.event.is_some());
                guides.apply(&g);
            }
        },
        host,
    )
}

fn harness(setup: impl FnOnce(&mut Host)) -> Harness<'static, Host> {
    harness_with(BoxState::from_rect(RECT), 0.0, false, setup)
}

fn view(h: &Harness<'_, Host>) -> CanvasView {
    h.state().view.expect("drawn")
}

fn screen(h: &Harness<'_, Host>, p: Point) -> Pos2 {
    view(h).to_screen(p)
}

fn state(h: &Harness<'_, Host>) -> BoxState {
    h.state().host.state
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

/// Presses at `from` and moves to `to`, the button held.
fn hold(h: &mut Harness<'_, Host>, from: Pos2, to: Pos2) {
    h.event(Event::PointerMoved(from));
    h.event(button(from, true, Modifiers::NONE));
    h.run();
    h.event(Event::PointerMoved(to));
    h.run();
}

/// The box `quad::drag` gives for a drag of `part` between screen points.
fn expected(h: &Harness<'_, Host>, start: &BoxState, part: BoxPart, from: Pos2, to: Pos2, mods: Modifiers) -> BoxState {
    let v = view(h);
    quad::drag(start, start, part, v.to_doc(from), v.to_doc(to), mods, &h.state().rules)
}

fn near(a: &BoxState, b: &BoxState) -> bool {
    a.quad.iter().chain([&a.pivot]).zip(b.quad.iter().chain([&b.pivot])).all(|(p, q)| p.distance(*q) < 1e-3)
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
        })
        .collect()
}

#[test]
fn the_nearest_handle_wins_on_a_small_box() {
    let small = BoxState::from_rect(Rect::new(190.0, 140.0, 210.0, 160.0));
    for (press, part) in [(Point::new(193.0, 141.0), BoxPart::Corner(0)), (Point::new(198.0, 141.0), BoxPart::Edge(0))] {
        let mut h = harness_with(small, 0.0, false, |_| {});
        h.run();
        let (from, to) = (screen(&h, press), screen(&h, press) + vec2(-20.0, -30.0));
        let want = expected(&h, &small, part, from, to, Modifiers::NONE);
        drag(&mut h, from, to, Modifiers::NONE);
        assert!(near(&state(&h), &want), "{part:?}: {:?}", state(&h));
        assert_eq!(h.state().picked, Some(part));
    }
}

#[test]
fn a_handle_takes_the_pointer_within_12_points_in_a_turned_flipped_view() {
    for (offset, part) in [(11.9, BoxPart::Corner(2)), (12.1, BoxPart::Outside)] {
        let mut h = harness_with(BoxState::from_rect(RECT), 30.0, true, |_| {});
        h.run();
        let v = view(&h);
        // Out of the box along the corner's diagonal, as the view draws it.
        let corner = screen(&h, Point::new(RECT.x1, RECT.y1));
        let out = (corner - screen(&h, RECT.center())).normalized();
        let at = corner + out * offset;
        h.event(Event::PointerMoved(at));
        h.run();
        assert_eq!(h.state().hovered, Some(part), "{offset} pt from the corner");
        let start = state(&h);
        let to = at + out * 30.0;
        let want = quad::drag(&start, &start, part, v.to_doc(at), v.to_doc(to), Modifiers::NONE, &BoxRules::default());
        drag(&mut h, at, to, Modifiers::NONE);
        assert!(near(&state(&h), &want), "{offset} pt: {:?}", state(&h));
    }
}

#[test]
fn each_part_drags_as_quad_drag_has_it_in_every_mode() {
    let modes = [TransformMode::Free, TransformMode::Rotate, TransformMode::Skew, TransformMode::Distort, TransformMode::Perspective];
    let parts = [
        (BoxPart::Corner(2), Point::new(300.0, 220.0)),
        (BoxPart::Edge(1), Point::new(300.0, 150.0)),
        (BoxPart::Inside, Point::new(150.0, 110.0)),
        (BoxPart::Outside, Point::new(340.0, 260.0)),
        (BoxPart::Pivot, RECT.center()),
    ];
    for mode in modes {
        for (part, at) in parts {
            let mut h = harness(|host| host.rules.mode = mode);
            h.run();
            let start = state(&h);
            let from = screen(&h, at);
            let to = from + vec2(30.0, 20.0);
            let rules = h.state().rules;
            let want = if rules.allows(part) { expected(&h, &start, part, from, to, Modifiers::NONE) } else { start };
            drag(&mut h, from, to, Modifiers::NONE);
            assert!(near(&state(&h), &want), "{mode:?} {part:?}: {:?}", state(&h));
            assert_eq!(h.state().applied, 0, "{mode:?} {part:?}: a drag doesn't apply");
            let moved = !near(&start, &want);
            assert_eq!(h.state().events.is_empty(), !moved, "{mode:?} {part:?}: {:?}", kinds(&h));
        }
    }
}

#[test]
fn a_click_outside_applies_and_a_drag_outside_rotates() {
    let mut h = harness(|_| {});
    h.run();
    let at = screen(&h, Point::new(340.0, 260.0));
    drag(&mut h, at, at + vec2(1.5, 0.0), Modifiers::NONE);
    assert_eq!(h.state().applied, 1, "a release within 2 pt applies");
    assert!(h.state().events.is_empty() && state(&h) == BoxState::from_rect(RECT));

    drag(&mut h, at, at + vec2(-40.0, 30.0), Modifiers::NONE);
    assert_eq!(h.state().applied, 1, "a drag doesn't apply");
    assert!(state(&h).angle().abs() > 1.0, "the drag turns the box: {}", state(&h).angle());

    // ⇧ snaps the angle to the rotate step.
    let mut h = harness(|_| {});
    h.run();
    drag(&mut h, at, at + vec2(-40.0, 30.0), Modifiers::SHIFT);
    let angle = state(&h).angle();
    assert!((angle / 15.0 - (angle / 15.0).round()).abs() < 1e-6 && angle.abs() > 1.0, "{angle}");
}

#[test]
fn a_double_click_inside_applies() {
    let mut h = harness(|_| {});
    h.run();
    let at = screen(&h, Point::new(150.0, 110.0));
    // A click per frame: frames are a quarter second apart, and egui's double-click window is 0.3 s.
    frame(&mut h, [Event::PointerMoved(at), button(at, true, Modifiers::NONE), button(at, false, Modifiers::NONE)]);
    frame(&mut h, [button(at, true, Modifiers::NONE), button(at, false, Modifiers::NONE)]);
    h.run();
    assert_eq!(h.state().applied, 1);
    assert!(h.state().events.is_empty(), "clicks don't move the box");
}

#[test]
fn enter_applies_esc_undoes_a_drag_then_closes_the_box() {
    let mut h = harness(|_| {});
    h.run();
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().applied, 1);

    let from = screen(&h, Point::new(150.0, 110.0));
    hold(&mut h, from, from + vec2(40.0, 0.0));
    assert!(near(&state(&h), &BoxState::from_rect(RECT).moved(kurbo::Vec2::new(40.0, 0.0))), "the host shows the drag");
    h.key_press(Key::Escape);
    h.run();
    assert!(matches!(h.state().events.last(), Some(EditEvent::Cancel { reason: CancelReason::Escape, .. })));
    assert_eq!(state(&h), BoxState::from_rect(RECT), "Esc undoes the drag");
    assert_eq!(h.state().discarded, 0, "the box stays open");
    let count = h.state().events.len();
    h.event(Event::PointerMoved(from + vec2(60.0, 0.0)));
    h.event(button(from + vec2(60.0, 0.0), false, Modifiers::NONE));
    h.run();
    assert_eq!(h.state().events.len(), count, "the rest of the press does nothing");
    h.key_press(Key::Escape);
    h.run();
    assert_eq!(h.state().discarded, 1, "a second Esc closes the box");
    assert_eq!(h.state().applied, 1);
}

#[test]
fn the_band_zone_leaves_the_rest_of_the_outside_to_the_tool() {
    let mut h = harness(|host| host.zone = RotateZone::Band);
    h.run();
    let far = screen(&h, Point::new(200.0, 30.0));
    h.event(Event::PointerMoved(far));
    h.run();
    assert_eq!(h.state().hovered, None);
    drag(&mut h, far, far + vec2(1.0, 0.0), Modifiers::NONE);
    assert!(!h.state().claimed);
    assert_eq!(h.state().applied, 0, "a click outside the band is the tool's");
    drag(&mut h, far, far + vec2(40.0, 30.0), Modifiers::NONE);
    assert!(h.state().events.is_empty());

    // 15 pt out along the corner's diagonal: in the band, it rotates.
    let corner = screen(&h, Point::new(300.0, 220.0));
    let out = (corner - screen(&h, RECT.center())).normalized();
    let at = corner + out * 15.0;
    h.event(Event::PointerMoved(at));
    h.run();
    assert_eq!(h.state().hovered, Some(BoxPart::Outside));
    drag(&mut h, at, at + vec2(-30.0, 20.0), Modifiers::NONE);
    assert!(state(&h).angle().abs() > 1.0);
}

#[test]
fn inside_moves_false_leaves_the_inside_to_the_tool() {
    let mut h = harness(|host| host.inside_moves = false);
    h.run();
    let from = screen(&h, Point::new(150.0, 110.0));
    h.event(Event::PointerMoved(from));
    h.event(button(from, true, Modifiers::NONE));
    h.run();
    assert!(!h.state().claimed, "the press is the tool's");
    h.event(Event::PointerMoved(from + vec2(30.0, 0.0)));
    h.event(button(from + vec2(30.0, 0.0), false, Modifiers::NONE));
    h.run();
    assert!(h.state().events.is_empty());
    let corner = screen(&h, Point::new(100.0, 80.0));
    drag(&mut h, corner, corner + vec2(-20.0, -10.0), Modifiers::NONE);
    assert_ne!(state(&h), BoxState::from_rect(RECT), "the handles still drag");
}

#[test]
fn arrows_move_the_box_and_a_picked_corner_scales() {
    let mut h = harness(|_| {});
    h.run();
    h.key_press(Key::ArrowRight);
    h.run();
    assert_eq!(state(&h), BoxState::from_rect(RECT).moved(kurbo::Vec2::new(1.0, 0.0)), "the keys work without a click first");
    assert!(matches!(h.state().events.last(), Some(EditEvent::Set { .. })));
    h.key_press_modifiers(Modifiers::SHIFT, Key::ArrowDown);
    h.run();
    assert_eq!(state(&h), BoxState::from_rect(RECT).moved(kurbo::Vec2::new(1.0, 10.0)));

    h.key_press(Key::Tab);
    h.run();
    assert_eq!(h.state().picked, Some(BoxPart::Inside), "focus picks the whole box");
    h.key_press(Key::PageDown);
    h.run();
    assert_eq!(h.state().picked, Some(BoxPart::Corner(0)));
    let start = state(&h);
    h.key_press(Key::ArrowLeft);
    h.run();
    let want = quad::drag(
        &start,
        &start,
        BoxPart::Corner(0),
        start.quad[0],
        start.quad[0] + kurbo::Vec2::new(-1.0, 0.0),
        Modifiers::NONE,
        &BoxRules::default(),
    );
    assert_eq!(state(&h), want, "a plain corner drag: proportional about the opposite corner");
    assert!((state(&h).quad[0].y - start.quad[0].y).abs() > 1e-6, "proportional: the height follows");
}

/// The pick is the keyboard's: with the focus elsewhere an arrow moves the whole box, not the
/// corner a drag picked (PhotoCraft).
#[test]
fn without_the_focus_the_arrows_move_the_whole_box_whatever_is_picked() {
    let mut h = harness(|_| {});
    h.run();
    let corner = screen(&h, RECT.origin());
    drag(&mut h, corner, corner + vec2(-10.0, 0.0), Modifiers::NONE);
    assert_eq!(h.state().picked, Some(BoxPart::Corner(0)));
    let start = state(&h);
    h.key_press(Key::ArrowLeft);
    h.run();
    assert_ne!(state(&h), start.moved(kurbo::Vec2::new(-1.0, 0.0)), "the release gave the box the focus: the corner scales");

    let away = Pos2::new(10.0, 10.0);
    h.event(Event::PointerMoved(away));
    h.event(button(away, true, Modifiers::NONE));
    h.event(button(away, false, Modifiers::NONE));
    h.run();
    let start = state(&h);
    h.key_press(Key::ArrowRight);
    h.run();
    assert_eq!(state(&h), start.moved(kurbo::Vec2::new(1.0, 0.0)));
}

/// Esc during a press that hasn't moved yet ends the press, as it would end its drag: the box
/// stays open and the rest of the press does nothing.
#[test]
fn esc_before_the_first_move_ends_the_press() {
    let mut h = harness(|_| {});
    h.run();
    let from = screen(&h, Point::new(150.0, 110.0));
    h.event(Event::PointerMoved(from));
    h.event(button(from, true, Modifiers::NONE));
    h.run();
    h.key_press(Key::Escape);
    h.run();
    assert_eq!(h.state().discarded, 0, "the box stays open");
    let to = from + vec2(40.0, 0.0);
    h.event(Event::PointerMoved(to));
    h.run();
    h.event(button(to, false, Modifiers::NONE));
    h.run();
    assert!(h.state().events.is_empty(), "{:?}", kinds(&h));
    assert_eq!(state(&h), BoxState::from_rect(RECT));
    h.key_press(Key::Escape);
    h.run();
    assert_eq!(h.state().discarded, 1, "the next Esc closes the box");
}

#[test]
fn page_keys_skip_the_parts_the_mode_refuses() {
    use BoxPart::{Corner, Inside, Pivot};
    let mut h = harness(|host| host.rules.mode = TransformMode::Distort);
    h.run();
    h.key_press(Key::Tab);
    h.run();
    let mut order = vec![h.state().picked];
    for _ in 0..6 {
        h.key_press(Key::PageDown);
        h.run();
        order.push(h.state().picked);
    }
    assert_eq!(order, [Some(Inside), Some(Corner(0)), Some(Corner(1)), Some(Corner(2)), Some(Corner(3)), Some(Pivot), Some(Pivot)]);
    h.key_press(Key::Home);
    h.run();
    assert_eq!(h.state().picked, Some(Inside));
}

/// The text of the readout on screen, if any.
fn readout(h: &Harness<'_, Host>) -> Option<String> {
    h.query_by(|n| n.role() == Role::Tooltip).and_then(|n| n.accesskit_node().label())
}

#[test]
fn the_readout_shows_what_each_gesture_changes() {
    let cases = [
        (Point::new(340.0, 260.0), vec2(-30.0, 30.0), "Angle:"),
        (Point::new(300.0, 220.0), vec2(20.0, 0.0), "W:"),
        (Point::new(150.0, 110.0), vec2(20.0, -10.0), "ΔX: 20 px\nΔY: -10 px"),
    ];
    for (at, by, want) in cases {
        let mut h = harness(|_| {});
        h.run();
        let from = screen(&h, at);
        hold(&mut h, from, from + by);
        let text = readout(&h).unwrap_or_default();
        assert!(text.starts_with(want), "{at:?}: {text:?}");
        h.event(button(from + by, false, Modifiers::NONE));
        h.run();
        assert_eq!(readout(&h), None, "only during the drag");
    }
    // A corner scaling the box 110 %, proportionally.
    let mut h = harness(|_| {});
    h.run();
    let from = screen(&h, Point::new(300.0, 220.0));
    hold(&mut h, from, from + vec2(20.0, 0.0));
    assert_eq!(readout(&h).as_deref(), Some("W: 110.0%\nH: 110.0%"));
    // The reference point and a distorted box have none.
    for (mode, at) in [(TransformMode::Free, RECT.center()), (TransformMode::Distort, Point::new(300.0, 220.0))] {
        let mut h = harness(|host| host.rules.mode = mode);
        h.run();
        let from = screen(&h, at);
        hold(&mut h, from, from + vec2(20.0, 5.0));
        assert_eq!(readout(&h), None, "{mode:?} {at:?}");
    }
}

#[test]
fn the_box_takes_a_press_on_a_handle_before_a_guide_under_it() {
    let mut h = harness(|host| host.guides = Some(GuideHost::new(vec![Guide::new(1, Orientation::Vertical, 100.0)])));
    h.run();
    let corner = screen(&h, Point::new(100.0, 80.0));
    drag(&mut h, corner, corner + vec2(-20.0, -10.0), Modifiers::NONE);
    assert_ne!(state(&h), BoxState::from_rect(RECT));
    assert_eq!(h.state().guide_events, 0);
    assert_eq!(h.state().guides.as_ref().map(|g| g.guides[0].position), Some(100.0));
    // Off the handles and outside the box, with the box applied by the click there, the guide
    // still loses the press: the box takes the whole outside.
    let on_guide = screen(&h, Point::new(100.0, 280.0));
    drag(&mut h, on_guide, on_guide + vec2(30.0, 0.0), Modifiers::NONE);
    assert_eq!(h.state().guides.as_ref().map(|g| g.guides[0].position), Some(100.0));
}

/// Queues `events` into one frame, as winit delivers a quick gesture, and runs it.
fn frame(h: &mut Harness<'_, Host>, events: impl IntoIterator<Item = Event>) {
    h.input_mut().events.extend(events);
    h.step();
}

#[test]
fn a_quick_drag_in_one_frame_starts_at_the_press() {
    let mut h = harness(|_| {});
    h.run();
    let from = screen(&h, Point::new(300.0, 220.0));
    let to = from + vec2(40.0, 10.0);
    let want = expected(&h, &state(&h), BoxPart::Corner(2), from, to, Modifiers::NONE);
    frame(
        &mut h,
        [Event::PointerMoved(from), button(from, true, Modifiers::NONE), Event::PointerMoved(from + vec2(20.0, 5.0)), Event::PointerMoved(to)],
    );
    frame(&mut h, [button(to, false, Modifiers::NONE)]);
    h.run();
    assert!(near(&state(&h), &want), "{:?}", state(&h));
    assert_eq!(kinds(&h).first(), Some(&"begin"));
    assert_eq!(kinds(&h).last(), Some(&"commit"));
}

/// The shapes of the last frame stroked in `color`.
fn painted(h: &Harness<'_, Host>, color: Color32) -> usize {
    fn count(shape: &Shape, color: Color32) -> usize {
        match shape {
            Shape::Vec(shapes) => shapes.iter().map(|s| count(s, color)).sum(),
            Shape::Path(path) => usize::from(path.stroke.color == ColorMode::Solid(color)),
            Shape::Rect(rect) => usize::from(rect.stroke.color == color && rect.stroke.width > 0.0),
            _ => 0,
        }
    }
    h.output().shapes.iter().map(|c| count(&c.shape, color)).sum()
}

fn descendants(node: egui_kittest::kittest::AccessKitNode<'_>) -> Vec<egui_kittest::kittest::AccessKitNode<'_>> {
    node.children().flat_map(|child| std::iter::once(child).chain(descendants(child))).collect()
}

#[test]
fn the_box_is_one_tab_stop_with_its_focus_after_keyboard_input_only() {
    let mut h = harness(|_| {});
    h.run();
    let focus = TransformBox::style(&current(&h.ctx)).focus.color;
    let corner = screen(&h, Point::new(100.0, 80.0));
    drag(&mut h, corner, corner, Modifiers::NONE);
    assert_eq!(h.state().picked, Some(BoxPart::Corner(0)), "the pointer picks the part it took");
    assert!(h.state().events.is_empty(), "a click on a handle is no event");
    assert_eq!(painted(&h, focus), 0, "no focus after the pointer");
    h.key_press(Key::PageDown);
    h.run();
    assert_eq!(h.state().picked, Some(BoxPart::Corner(1)), "the release gave the box the keyboard");
    assert_eq!(painted(&h, focus), 1, "the ring round the picked handle");
    h.key_press(Key::Home);
    h.run();
    assert_eq!(painted(&h, focus), 1, "the focus line round the whole box");
}

#[test]
fn the_parts_are_graphics_objects_in_a_group() {
    let mut h = harness(|host| host.rules.mode = TransformMode::Distort);
    h.run();
    h.key_press(Key::Tab);
    h.run();
    h.key_press(Key::PageDown);
    h.run();
    let group = h.get_by_label("Free Transform");
    assert_eq!(group.accesskit_node().role(), Role::Group);
    assert_eq!(group.accesskit_node().value().as_deref(), Some("W 100%"));
    let stops = descendants(group.accesskit_node()).into_iter().filter(|n| n.data().supports_action(egui::accesskit::Action::Focus)).count();
    assert_eq!(stops, 0, "one Tab stop");
    let objects: Vec<_> = descendants(group.accesskit_node())
        .into_iter()
        .filter(|n| n.role() == Role::GraphicsObject)
        .map(|n| (n.label().unwrap_or_default(), n.value().unwrap_or_default(), n.data().is_selected()))
        .collect();
    let s = |name: &str, value: &str, selected: bool| (name.to_owned(), value.to_owned(), Some(selected));
    assert_eq!(
        objects,
        [
            s("Whole box", "W 100%", false),
            s("Top left corner", "100.0 px, 80.0 px", true),
            s("Top right corner", "300.0 px, 80.0 px", false),
            s("Bottom right corner", "300.0 px, 220.0 px", false),
            s("Bottom left corner", "100.0 px, 220.0 px", false),
            s("Reference point", "200.0 px, 150.0 px", false),
        ],
        "edges are refused in Distort"
    );
    assert_eq!(group.accesskit_node().active_descendant().and_then(|n| n.label()).as_deref(), Some("Top left corner"));
}

/// A press on the box while a text field's typing run has the focus asks the run to end and waits
/// for its Commit: the box's Begin comes in a later pass, from the box the host has then, and the
/// drag still follows the pointer from the press.
#[test]
fn a_press_during_a_typing_run_begins_after_the_runs_commit() {
    let mut h = harness(|h| h.typing = true);
    h.step();
    assert!(h.state().typing_events.is_empty() || matches!(h.state().typing_events[..], [EditEvent::Begin { .. }]));
    let from = screen(&h, Point::new(150.0, 110.0));
    h.event(Event::PointerMoved(from));
    h.step();
    h.event(button(from, true, Modifiers::NONE));
    h.step();
    assert!(h.state().events.is_empty(), "the press waits: {:?}", h.state().events);
    assert!(h.state().claimed, "the box holds the press meanwhile");
    h.event(Event::PointerMoved(from + vec2(30.0, 0.0)));
    for _ in 0..4 {
        h.step();
    }
    h.event(Event::PointerMoved(from + vec2(40.0, 0.0)));
    h.step();
    h.event(button(from + vec2(40.0, 0.0), false, Modifiers::NONE));
    h.step();
    h.step();
    let (events, commit_pass, box_passes) = (&h.state().events, h.state().typing_passes.last().copied(), &h.state().box_passes);
    assert!(matches!(h.state().typing_events.last(), Some(EditEvent::Commit { .. })), "{:?}", h.state().typing_events);
    assert!(matches!(events.first(), Some(EditEvent::Begin { .. })), "{events:?}");
    assert!(commit_pass < box_passes.first().copied(), "the run's Commit goes out first: {commit_pass:?} {box_passes:?}");
    assert!(matches!(events.last(), Some(EditEvent::Commit { changed: true, .. })), "{events:?}");
    let moved = state(&h);
    assert!((moved.quad[0].x - RECT.x0 - 40.0).abs() < 1e-3, "the whole move from the press: {moved:?}");
}

/// A press released while it waited still runs, as the click it was.
#[test]
fn a_press_released_while_it_waited_runs_as_a_click() {
    let mut h = harness(|h| h.typing = true);
    h.step();
    let outside = screen(&h, Point::new(20.0, 20.0));
    h.event(Event::PointerMoved(outside));
    h.step();
    h.event(button(outside, true, Modifiers::NONE));
    h.event(button(outside, false, Modifiers::NONE));
    h.step();
    assert_eq!(h.state().applied, 0, "the click waits");
    for _ in 0..4 {
        h.step();
    }
    assert_eq!(h.state().applied, 1, "and runs once");
    h.step();
    assert_eq!((h.state().applied, h.state().claimed), (1, false));
}

#[test]
fn modifiers_override_replaces_the_input_modifiers_in_each_overlay() {
    let (part, at) = (BoxPart::Corner(2), Point::new(300.0, 220.0));
    for (input, held) in [(Modifiers::NONE, Modifiers::SHIFT), (Modifiers::SHIFT, Modifiers::NONE)] {
        let mut h = harness(|host| host.modifiers = Some(held));
        h.run();
        let start = state(&h);
        let (from, to) = (screen(&h, at), screen(&h, at) + vec2(40.0, 10.0));
        let want = expected(&h, &start, part, from, to, held);
        assert!(!near(&want, &expected(&h, &start, part, from, to, input)), "the two modifier sets drag differently");
        drag(&mut h, from, to, input);
        assert!(near(&state(&h), &want), "input {input:?}, override {held:?}: {:?}", state(&h));
    }
}

/// PC `pointer`: ⌥ on a press inside or outside the box (away from the handles) puts the
/// reference point there and drags it, so the box itself stays.
#[test]
fn alt_beyond_the_box_places_the_reference_point_there_and_drags_it() {
    let far = Point::new(340.0, 260.0);
    let mut h = harness(|_| {});
    h.run();
    let start = state(&h);
    let at = screen(&h, far);
    drag(&mut h, at, at + vec2(-20.0, 10.0), Modifiers::ALT);
    let got = state(&h);
    assert_eq!(got.quad, start.quad, "the box doesn't turn");
    let want = view(&h).to_doc(at + vec2(-20.0, 10.0));
    assert!(got.pivot.distance(want) < 1e-3, "the reference point follows the pointer: {:?} vs {want:?}", got.pivot);
    assert_eq!(h.state().applied, 0);

    // A press without a drag only places it.
    let mut h = harness(|_| {});
    h.run();
    drag(&mut h, at, at, Modifiers::ALT);
    assert!(state(&h).pivot.distance(view(&h).to_doc(at)) < 1e-3, "{:?}", state(&h).pivot);
    assert_eq!(h.state().applied, 0, "no apply");
}

#[test]
fn the_rotate_zone_beyond_the_corners_turns_about_the_reference_point_and_shift_snaps_the_turn() {
    let mut h = harness(|_| {});
    h.run();
    let start = state(&h);
    // Far outside a corner, on the diagonal.
    let from = screen(&h, Point::new(380.0, 290.0));
    let to = from + vec2(-60.0, 50.0);
    drag(&mut h, from, to, Modifiers::NONE);
    let free = state(&h);
    assert_eq!(free.pivot, start.pivot, "turning keeps the reference point");
    assert!(free.angle().abs() > 1.0);
    // The turn is the pointer's angle about the pivot, with the corners rigid.
    assert!((free.quad[0].distance(free.quad[1]) - start.quad[0].distance(start.quad[1])).abs() < 1e-6);

    let mut h = harness(|_| {});
    h.run();
    drag(&mut h, from, to, Modifiers::SHIFT);
    let snapped = state(&h).angle();
    assert!((snapped / 15.0 - (snapped / 15.0).round()).abs() < 1e-6, "{snapped}");
    assert!((snapped - free.angle()).abs() <= 7.5 + 1e-6, "{snapped} vs {}", free.angle());
}

/// Presses the bottom-right corner and moves in 30 steps to `by` away, the host feeding each
/// emitted box back as `state`.
fn grid_drag(h: &mut Harness<'_, Host>, steps: u8, by: egui::Vec2) -> (BoxState, Pos2, Pos2) {
    let from = screen(h, Point::new(300.0, 220.0));
    let to = from + by;
    h.event(Event::PointerMoved(from));
    h.event(button(from, true, Modifiers::NONE));
    h.run();
    for i in 1..=steps {
        h.event(Event::PointerMoved(from.lerp(to, f32::from(i) / 30.0)));
        h.run();
    }
    (state(h), from, to)
}

#[test]
fn a_pixel_grid_box_fed_back_as_state_does_not_drift_over_a_drag() {
    let mut h = harness(|host| host.pixel_grid = true);
    h.run();
    let start = state(&h);
    let (shown, from, to) = grid_drag(&mut h, 30, vec2(30.4, 20.4));
    h.event(button(to, false, Modifiers::NONE));
    h.run();
    assert!(!kinds(&h).contains(&"cancel"), "{:?}", kinds(&h));
    let v = view(&h);
    let exact = quad::drag(&start, &start, BoxPart::Corner(2), v.to_doc(from), v.to_doc(to), Modifiers::NONE, &h.state().rules);
    let whole = |p: Point| Point::new(p.x.round(), p.y.round());
    let want = BoxState { quad: exact.quad.map(whole), pivot: exact.pivot };
    assert!(near(&shown, &want) && near(&state(&h), &want), "{shown:?} != {want:?}");
    assert!(shown.quad.iter().zip(exact.quad).all(|(a, b)| a.distance(b) <= 0.71), "within the rounding of the exact box");
    assert_eq!(kinds(&h).last(), Some(&"commit"));
}

#[test]
fn an_external_change_during_a_pixel_grid_drag_still_cancels() {
    let mut h = harness(|host| host.pixel_grid = true);
    h.run();
    grid_drag(&mut h, 10, vec2(30.4, 20.4));
    h.state_mut().host.state = BoxState::from_rect(Rect::new(10.0, 10.0, 50.0, 50.0));
    h.event(Event::PointerMoved(screen(&h, Point::new(330.0, 240.0))));
    h.run();
    assert_eq!(kinds(&h).last(), Some(&"cancel"), "{:?}", kinds(&h));
}
