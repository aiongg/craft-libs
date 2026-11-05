//! Tests of the grip set (task K12).

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect, Sense, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use kurbo::{Point, Vec2};

use super::{Grip, GripClick, GripDrag, GripSet, GripSetStyle, GripShape, GripState};
use crate::canvas::{CanvasView, RulerUnits};
use crate::edit::EditEvent;
use crate::localized::LocaleNumbers;
use crate::theme::{Base, current, install};

/// A host with a 400 × 300 canvas showing document (0, 0)–(400, 300) at zoom 1, turned by
/// `rotation`; it moves a grip to the `to` of every event.
struct Host {
    grips: Vec<Grip>,
    rotation: f64,
    view: Option<CanvasView>,
    style: Option<GripSetStyle>,
    events: Vec<EditEvent<GripDrag>>,
    clicks: Vec<GripClick>,
    hovered: Option<u64>,
    picked: Option<u64>,
}

fn harness(grips: Vec<Grip>) -> Harness<'static, Host> {
    let host = Host { grips, rotation: 0.0, view: None, style: None, events: Vec::new(), clicks: Vec::new(), hovered: None, picked: None };
    Harness::<Host>::builder().with_size([480.0, 400.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut Host| {
            ui.add_space(40.0);
            // Not a Tab stop, so Tab goes to the set.
            let (rect, canvas) = ui.allocate_exact_size(vec2(400.0, 300.0), Sense::click_and_drag() - Sense::FOCUSABLE);
            let view = CanvasView { rotation: host.rotation, ..CanvasView::new(rect, Point::new(200.0, 150.0), 1.0) };
            host.view = Some(view);
            host.style = Some(GripSet::style(&current(ui.ctx())));
            let px = RulerUnits::pixels(LocaleNumbers::from_tag("en"));
            let r = GripSet::new("grips", &host.grips).units(&px, &px).show(ui, &canvas, &view);
            let moved = match &r.drag {
                Some(EditEvent::Begin { value: Some(v), .. }) => Some(*v),
                Some(EditEvent::Update { value, .. } | EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) => Some(*value),
                _ => None,
            };
            if let Some(v) = moved
                && let Some(g) = host.grips.iter_mut().find(|g| g.key == v.key)
            {
                g.at = v.to;
            }
            host.events.extend(r.drag);
            host.clicks.extend(r.clicked);
            (host.hovered, host.picked) = (r.hovered, r.picked);
        },
        host,
    )
}

fn view(h: &Harness<'_, Host>) -> CanvasView {
    h.state().view.expect("drawn")
}

fn style(h: &Harness<'_, Host>) -> GripSetStyle {
    h.state().style.expect("drawn")
}

fn button(pos: Pos2, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers }
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

fn hover_at(h: &mut Harness<'_, Host>, p: Pos2) -> Option<u64> {
    h.event(Event::PointerMoved(p));
    h.run();
    h.state().hovered
}

fn square(key: u64, x: f64, y: f64) -> Grip {
    Grip::new(key, Point::new(x, y), GripShape::Square)
}

fn bounds(h: &Harness<'_, Host>, g: &Grip) -> Rect {
    let (v, s) = (view(h), style(h));
    Rect::from_points(&GripSet::outline(g.shape, v.to_screen(g.at), s.size, s.midpoint_height, &v))
}

// A grip under the pointer (within warm_pad) is Warm, a hot grip is Hot over Warm, the rest Cold.
#[test]
fn grip_states_cold_warm_hot() {
    let mut h = harness(Vec::new());
    h.run();
    let s = style(&h);
    let grip = square(1, 100.0, 100.0);
    let b = bounds(&h, &grip);
    let state = |g: &Grip, b: Rect, p: Option<Pos2>| GripSet::state(g, b, p, &s);
    assert_eq!(state(&grip, b, None), GripState::Cold);
    assert_eq!(state(&grip, b, Some(b.right_center() + vec2(s.warm_pad - 0.1, 0.0))), GripState::Warm);
    assert_eq!(state(&grip, b, Some(b.right_center() + vec2(s.warm_pad + 0.1, 0.0))), GripState::Cold);
    let hot = grip.clone().hot(true);
    assert_eq!(state(&hot, b, Some(b.center())), GripState::Hot, "hot wins over warm");
    assert_eq!(state(&hot, b, None), GripState::Hot);
    // A dynamic grip warms over its hit area.
    let arrow = Grip::new(2, grip.at, GripShape::Arrow { direction: Vec2::new(1.0, 0.0) });
    let b = bounds(&h, &arrow);
    assert_eq!(state(&arrow, b, Some(b.right_center() + vec2(s.hit_pad - 0.1, 0.0))), GripState::Warm);
    assert_eq!(state(&arrow, b, Some(b.right_center() + vec2(s.hit_pad + 0.1, 0.0))), GripState::Cold);
}

// The hit area is each shape's bounds plus hit_pad: square, midpoint rectangle, arrow along its
// screen direction, triangle.
#[test]
fn grip_hit_pad_and_shape_bounds() {
    let grips = vec![
        square(1, 60.0, 150.0),
        Grip::new(2, Point::new(160.0, 150.0), GripShape::Midpoint),
        Grip::new(3, Point::new(260.0, 150.0), GripShape::Arrow { direction: Vec2::new(0.0, 1.0) }),
        Grip::new(4, Point::new(340.0, 150.0), GripShape::Triangle),
    ];
    for rotation in [0.0, 90.0] {
        let mut h = harness(grips.clone());
        h.state_mut().rotation = rotation;
        h.run();
        let (v, s) = (view(&h), style(&h));
        for g in &grips {
            let b = bounds(&h, g);
            let pad = s.hit_pad;
            assert_eq!(hover_at(&mut h, b.right_center() + vec2(pad - 0.1, 0.0)), Some(g.key), "{:?} turned {rotation}: inside the pad", g.shape);
            assert_eq!(hover_at(&mut h, b.right_center() + vec2(pad + 0.1, 0.0)), None, "{:?} turned {rotation}: outside the pad", g.shape);
            let pts = GripSet::outline(g.shape, v.to_screen(g.at), s.size, s.midpoint_height, &v);
            match g.shape {
                GripShape::Midpoint => assert_eq!((b.width(), b.height()), (s.size, s.midpoint_height)),
                GripShape::Square => assert_eq!((b.width(), b.height()), (s.size, s.size)),
                GripShape::Arrow { direction } => {
                    // The tip lies along the screen image of the direction.
                    let tip = pts[1] - v.to_screen(g.at);
                    let want = v.map_vec(direction).normalized();
                    assert!((tip.normalized() - want).length() < 1e-4, "turned {rotation}: {tip:?} vs {want:?}");
                }
                GripShape::Triangle => assert!(pts[2].y > pts[0].y, "a downward triangle"),
            }
        }
    }
}

// A press and release on a grip with ⇧ reports `GripClick { shift: true }`; Enter and ⇧+Enter on the
// picked grip do the same.
#[test]
fn grip_shift_click_reports_modifier() {
    let mut h = harness(vec![square(7, 100.0, 100.0), square(8, 200.0, 100.0)]);
    h.run();
    let at = view(&h).to_screen(Point::new(100.0, 100.0));
    for (shift, mods) in [(false, Modifiers::NONE), (true, Modifiers::SHIFT)] {
        h.state_mut().clicks.clear();
        h.event(Event::ModifiersChanged(mods));
        h.event(Event::PointerMoved(at));
        h.event(button(at, true, mods));
        h.run();
        h.event(button(at, false, mods));
        h.event(Event::ModifiersChanged(Modifiers::NONE));
        h.run();
        assert_eq!(h.state().clicks, vec![GripClick { key: 7, shift }]);
        assert!(h.state().events.is_empty(), "a click is not a drag");
    }
    assert_eq!(h.state().picked, Some(7), "the pointer picks the grip");

    h.state_mut().clicks.clear();
    h.key_press(Key::PageDown);
    h.run();
    assert_eq!(h.state().picked, Some(8));
    h.key_press(Key::Enter);
    h.run();
    h.key_press_modifiers(Modifiers::SHIFT, Key::Enter);
    h.run();
    assert_eq!(h.state().clicks, vec![GripClick { key: 8, shift: false }, GripClick { key: 8, shift: true }]);
}

// A drag reports Begin, Update and Commit of `GripDrag`; an arrow on the picked grip one `Set`.
#[test]
fn grip_drag_and_arrows_report_edit_events() {
    let mut h = harness(vec![square(1, 100.0, 100.0)]);
    h.run();
    let v = view(&h);
    let (from, to) = (v.to_screen(Point::new(100.0, 100.0)), v.to_screen(Point::new(140.0, 130.0)));
    h.event(Event::PointerMoved(from));
    h.event(button(from, true, Modifiers::NONE));
    h.run();
    h.event(Event::PointerMoved(from.lerp(to, 0.5)));
    h.run();
    h.event(Event::PointerMoved(to));
    h.run();
    h.event(button(to, false, Modifiers::NONE));
    h.run();
    let seen = kinds(&h);
    assert_eq!(seen.first(), Some(&"begin"), "{seen:?}");
    assert_eq!(seen.last(), Some(&"commit"), "{seen:?}");
    assert!(seen.contains(&"update"), "{seen:?}");
    let Some(EditEvent::Commit { value, .. }) = h.state().events.last().cloned() else { panic!("a commit last") };
    assert_eq!(value, GripDrag { key: 1, from: Point::new(100.0, 100.0), to: Point::new(140.0, 130.0) });
    assert!(h.state().clicks.is_empty());

    // Esc cancels a drag.
    h.state_mut().events.clear();
    h.event(Event::PointerMoved(to));
    h.event(button(to, true, Modifiers::NONE));
    h.run();
    h.event(Event::PointerMoved(to + vec2(30.0, 0.0)));
    h.run();
    h.key_press(Key::Escape);
    h.run();
    h.event(button(to + vec2(30.0, 0.0), false, Modifiers::NONE));
    h.run();
    let seen = kinds(&h);
    assert_eq!(seen.last(), Some(&"cancel"), "{seen:?}");
    assert!(!seen.contains(&"commit"), "{seen:?}");

    // Arrows move the picked grip, one Set per press.
    h.state_mut().events.clear();
    h.key_press(Key::Tab);
    h.run();
    h.key_press(Key::ArrowRight);
    h.run();
    h.key_press_modifiers(Modifiers::SHIFT, Key::ArrowDown);
    h.run();
    assert_eq!(kinds(&h), vec!["set", "set"]);
    let Some(EditEvent::Set { value, .. }) = h.state().events.last().cloned() else { panic!("a set last") };
    assert_eq!(value.key, 1);
    assert_eq!(value.to - value.from, Vec2::new(0.0, 10.0));
}

// 250 grips: all drawn, 200 parts in the AccessKit tree and the group's value names the rest.
#[test]
fn grip_access_is_capped_at_200() {
    let grips = (0..250u64).map(|i| square(i, 10.0 + (i % 25) as f64 * 15.0, 10.0 + (i / 25) as f64 * 25.0)).collect();
    let mut h = harness(grips);
    h.run();
    let group = h.get_by_label("Grips");
    assert_eq!(group.accesskit_node().role(), Role::Group);
    assert_eq!(group.accesskit_node().value().as_deref(), Some("More grips"));
    assert_eq!(h.get_all_by_role(Role::GraphicsObject).count(), GripSet::ACCESS_LIMIT);
    assert!(h.query_by_label("Grip 3 of 250").is_some());
    assert!(h.query_by_label("Grip 200 of 250").is_some());
    assert!(h.query_by_label("Grip 201 of 250").is_none());
}
