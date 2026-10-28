use egui::accesskit::{Action, ActionRequest, Role};
use egui::{Context, Event, Key, Modifiers, PointerButton, Pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use kurbo::{Point, Rect};

use super::super::quad::Anchor9;
use super::*;
use crate::direction::{self, Direction};
use crate::edit::EditEvent;
use crate::theme::{Base, install};

const RECT: Rect = Rect::new(100.0, 50.0, 300.0, 150.0);

fn conversion(linked: bool) -> Conversion {
    let pixels = RulerUnits::pixels(LocaleNumbers::from_tag("en"));
    Conversion { rect: RECT, x: pixels.clone(), y: pixels, size: SizeUnit::Percent, linked }
}

fn close(a: Point, b: Point) -> bool {
    (a - b).hypot() < 1e-6
}

#[test]
fn a_linked_width_scales_the_height_about_the_reference_point() {
    let state = BoxState::from_rect(RECT);
    let (linked, _) = conversion(true).edit(&state, 0.0, Field::W, 50.0);
    let (unlinked, _) = conversion(false).edit(&state, 0.0, Field::W, 50.0);
    assert!(close(linked.pivot, state.pivot), "the reference point stays put");
    assert!(close(linked.quad[0], Point::new(150.0, 75.0)) && close(linked.quad[2], Point::new(250.0, 125.0)), "{:?}", linked.quad);
    assert!(close(unlinked.quad[0], Point::new(150.0, 50.0)) && close(unlinked.quad[2], Point::new(250.0, 150.0)), "{:?}", unlinked.quad);
    // Through a corner reference point the opposite corner is fixed.
    let corner = BoxState { pivot: Point::new(RECT.x0, RECT.y0), ..state };
    let (scaled, _) = conversion(true).edit(&corner, 0.0, Field::H, 200.0);
    assert!(close(scaled.quad[0], Point::new(RECT.x0, RECT.y0)) && close(scaled.quad[2], Point::new(500.0, 250.0)), "{:?}", scaled.quad);
}

#[test]
fn the_angle_and_skew_fields_turn_and_lean_the_box_about_the_reference_point() {
    let state = BoxState::from_rect(RECT);
    let c = conversion(true);
    let (turned, _) = c.edit(&state, 0.0, Field::Angle, 30.0);
    assert!(close(turned.pivot, state.pivot));
    assert!((c.shown(&turned, 0.0, Field::Angle) - 30.0).abs() < 1e-9);
    assert!((c.shown(&turned, 0.0, Field::W) - 100.0).abs() < 1e-9, "turning keeps the size");
    let (leaned, _) = c.edit(&state, 0.0, Field::SkewH, 20.0);
    assert!(close(leaned.pivot, state.pivot) && close(leaned.point_at(Anchor9::Center), state.pivot), "{leaned:?}");
    assert!((c.shown(&leaned, 0.0, Field::SkewH) - 20.0).abs() < 1e-6);
    // The V skew is the host's: editing it returns it, and the box leans about the reference point.
    let (v, kept) = c.edit(&state, 0.0, Field::SkewV, 15.0);
    assert_eq!(kept, 15.0);
    assert!(close(v.point_at(Anchor9::Center), state.pivot));
    assert!((c.shown(&v, kept, Field::SkewV) - 15.0).abs() < 1e-9);
}

#[test]
fn a_flipped_box_keeps_its_sign_through_the_fields() {
    let c = conversion(false);
    let state = BoxState::from_rect(RECT);
    let (flipped, _) = c.edit(&state, 0.0, Field::W, -100.0);
    assert!(c.shown(&flipped, 0.0, Field::W) < 0.0 && c.shown(&flipped, 0.0, Field::H) > 0.0);
    for (field, value) in [(Field::Angle, 20.0), (Field::H, 50.0), (Field::SkewH, 10.0)] {
        let (edited, _) = c.edit(&flipped, 0.0, field, value);
        assert!(c.shown(&edited, 0.0, Field::W) < 0.0, "{field:?} left the flip");
    }
    let (turned, _) = c.edit(&flipped, 0.0, Field::Angle, 20.0);
    assert!((c.shown(&turned, 0.0, Field::W) + 100.0).abs() < 1e-6);
}

#[test]
fn position_moves_the_box_with_its_reference_point_and_every_field_has_a_limit() {
    let c = conversion(true);
    let state = BoxState::from_rect(RECT);
    let (moved, _) = c.edit(&state, 0.0, Field::X, 10.0);
    assert!(close(moved.pivot, Point::new(10.0, state.pivot.y)) && close(moved.quad[0], Point::new(-90.0, 50.0)), "{moved:?}");
    let (moved, _) = c.edit(&moved, 0.0, Field::Y, -5.0);
    assert!(close(moved.pivot, Point::new(10.0, -5.0)));
    assert_eq!([Field::X, Field::Y, Field::Angle, Field::SkewH, Field::SkewV].map(|f| c.bound(f)), [300_000.0, 300_000.0, 180.0, 89.0, 89.0]);
    // A 200 px side stays within 300 000 px: 1500 times, 150 000 %.
    assert_eq!(c.bound(Field::W), 150_000.0);
    // Linked, the other side follows within its own limit: H (100 px) allows 3000 times, W 1500.
    let (huge, _) = c.edit(&state, 0.0, Field::H, 1.0e9);
    let (w, h) = (c.shown(&huge, 0.0, Field::W), c.shown(&huge, 0.0, Field::H));
    assert!((w / 100.0 - 1500.0).abs() < 1e-3 && (h / 100.0 - 1500.0).abs() < 1e-3, "{w} {h}");
    // In lengths, the box's side in the unit.
    let px = RulerUnits::pixels(LocaleNumbers::from_tag("en"));
    let length = Conversion { size: SizeUnit::Length { x: px.clone(), y: px }, ..conversion(false) };
    assert_eq!(length.shown(&state, 0.0, Field::W), 200.0);
    let (wide, _) = length.edit(&state, 0.0, Field::W, 400.0);
    assert!((length.shown(&wide, 0.0, Field::W) - 400.0).abs() < 1e-9);
}

struct Host {
    state: BoxState,
    /// The box as the running gesture began, to restore on a cancel.
    before: Option<BoxState>,
    linked: bool,
    /// Whether the host gives the row the link; without it the row keeps its own.
    host_link: bool,
    skew_v: f64,
    labels: TransformFieldLabels,
    events: Vec<EditEvent<BoxState>>,
}

impl Host {
    fn event(&self) -> Option<&EditEvent<BoxState>> {
        self.events.last()
    }

    fn kinds(&self) -> Vec<&'static str> {
        self.events
            .iter()
            .map(|e| match e {
                EditEvent::Begin { .. } => "Begin",
                EditEvent::Update { .. } => "Update",
                EditEvent::Commit { .. } => "Commit",
                EditEvent::Cancel { .. } => "Cancel",
                EditEvent::Set { .. } => "Set",
            })
            .collect()
    }
}

fn harness_with(dir: Direction, setup: impl FnOnce(&mut Host)) -> Harness<'static, Host> {
    let mut host = Host {
        state: BoxState::from_rect(RECT),
        before: None,
        linked: true,
        host_link: true,
        skew_v: 0.0,
        labels: TransformFieldLabels::default(),
        events: Vec::new(),
    };
    setup(&mut host);
    Harness::<Host>::builder()
        .with_size([900.0, 200.0])
        .with_context_setup(move |ctx: &Context| {
            install(ctx, &Base::suite_default());
            direction::apply(ctx, dir);
        })
        .build_ui_state(
            |ui, host: &mut Host| {
                let state = host.state;
                let fields = TransformFields::new("fields", &state, RECT, host.labels.clone()).skew_v(&mut host.skew_v);
                let edit = if host.host_link { fields.linked(&mut host.linked).show(ui) } else { fields.show(ui) };
                // A host that applies every update live, as a box session does.
                match &edit.event {
                    Some(EditEvent::Begin { .. }) => host.before = Some(host.state),
                    Some(EditEvent::Update { value, .. } | EditEvent::Set { value, .. } | EditEvent::Commit { value, .. }) => host.state = *value,
                    Some(EditEvent::Cancel { .. }) => host.state = host.before.take().unwrap_or(host.state),
                    None => {}
                }
                host.events.extend(edit.event);
            },
            host,
        )
}

fn harness(dir: Direction) -> Harness<'static, Host> {
    harness_with(dir, |_| {})
}

fn centre(h: &Harness<'_, Host>, name: &str) -> Pos2 {
    h.get_by_label(name).rect().center()
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

/// Scrubs the field `name` by `steps` moves of `dx` points, a frame each; the button stays down.
fn scrub(h: &mut Harness<'_, Host>, name: &str, dx: f32, steps: usize) -> Pos2 {
    let from = centre(h, name);
    h.event(Event::PointerMoved(from));
    h.event(button(from, true));
    h.run();
    let mut at = from;
    for _ in 0..steps {
        at += vec2(dx, 0.0);
        h.event(Event::PointerMoved(at));
        h.run();
    }
    at
}

fn release(h: &mut Harness<'_, Host>, at: Pos2) {
    h.event(button(at, false));
    h.run();
}

fn shown(h: &Harness<'_, Host>, field: Field) -> f64 {
    conversion(h.state().linked).shown(&h.state().state, h.state().skew_v, field)
}

#[test]
fn the_fields_are_named_spin_buttons_and_an_arrow_step_reports_a_box() {
    let mut h = harness(Direction::Ltr);
    h.run();
    let names = ["X position", "Y position", "Width", "Height", "Angle", "Horizontal skew", "Vertical skew"];
    for name in names {
        assert_eq!(h.get_by_label(name).accesskit_node().role(), Role::SpinButton, "{name}");
    }
    assert!(h.query_by(|n| n.role() == Role::Button && n.label().as_deref() == Some("Maintain aspect ratio")).is_some());
    h.get_by_label("Width").focus();
    h.run();
    h.key_press(Key::ArrowUp);
    h.run();
    let Some(EditEvent::Set { value, .. }) = h.state().event().cloned() else { panic!("{:?}", h.state().events) };
    // Linked, 101 % wide is 101 % high, about the centre.
    assert!((value.quad[1].x - value.quad[0].x - 202.0).abs() < 1e-6 && (value.quad[3].y - value.quad[0].y - 101.0).abs() < 1e-6, "{value:?}");
    assert!(close(value.pivot, BoxState::from_rect(RECT).pivot));
}

/// The box read back through its parts is often a hair off the value a field gave (an angle of
/// 1° reads 0.9999999999999998): while the box is the one the field made, the field reads its
/// own value, so a scrub through a host that applies its updates goes on to the release.
#[test]
fn a_scrub_through_a_host_that_applies_its_updates_is_one_gesture() {
    let mut h = harness(Direction::Ltr);
    h.run();
    let at = scrub(&mut h, "Angle", 3.0, 12);
    release(&mut h, at);
    let kinds = h.state().kinds();
    assert_eq!((kinds.first(), kinds.last()), (Some(&"Begin"), Some(&"Commit")), "{kinds:?}");
    assert!(kinds.iter().filter(|k| **k == "Update").count() >= 10, "{kinds:?}");
    assert!(!kinds.contains(&"Cancel"), "{kinds:?}");
    assert!((shown(&h, Field::Angle) - 36.0).abs() < 1e-6, "{}", shown(&h, Field::Angle));
}

/// The V skew is the host's and part of the row's gesture: a cancelled V scrub restores the one
/// the gesture began with.
#[test]
fn a_cancelled_skew_scrub_restores_the_host_s_skew() {
    let mut h = harness_with(Direction::Ltr, |host| host.skew_v = 5.0);
    h.run();
    scrub(&mut h, "Vertical skew", 3.0, 4);
    assert!((h.state().skew_v - 17.0).abs() < 1e-9, "{}", h.state().skew_v);
    h.key_press(Key::Escape);
    h.run();
    assert!(matches!(h.state().event(), Some(EditEvent::Cancel { .. })), "{:?}", h.state().kinds());
    assert_eq!(h.state().skew_v, 5.0);
    assert_eq!(h.state().state, BoxState::from_rect(RECT));
}

/// A scrub on W while X's typing run is open joins the run's gesture: the host sees one gesture,
/// and its commit carries both.
#[test]
fn a_scrub_during_another_field_s_typing_run_joins_its_gesture() {
    let mut h = harness(Direction::Ltr);
    h.run();
    let x = centre(&h, "X position");
    h.event(Event::PointerMoved(x));
    h.event(button(x, true));
    h.event(button(x, false));
    h.run();
    h.event(Event::Text("5".into()));
    h.run();
    let at = scrub(&mut h, "Width", 2.0, 5);
    release(&mut h, at);
    h.key_press(Key::Enter);
    h.run();
    let kinds = h.state().kinds();
    assert_eq!(kinds.iter().filter(|k| **k == "Begin").count(), 1, "{kinds:?}");
    assert_eq!(kinds.last(), Some(&"Commit"), "{kinds:?}");
    assert!(!kinds.contains(&"Cancel"), "{kinds:?}");
    assert!((h.state().state.pivot.x - 5.0).abs() < 1e-9, "{:?}", h.state().state);
    assert!((shown(&h, Field::W) - 110.0).abs() < 1e-6, "{}", shown(&h, Field::W));
}

/// Two fields reporting in one frame (an arrow in the focused field while a screen reader steps
/// another) both count: the second applies to the box the first left.
#[test]
fn two_fields_in_one_frame_both_reach_the_host() {
    let mut h = harness(Direction::Ltr);
    h.run();
    h.get_by_label("Width").focus();
    h.run();
    let (target_node, target_tree) = h.get_by_label("Angle").accesskit_node().locate();
    h.input_mut().events.extend([
        Event::Key { key: Key::ArrowUp, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE },
        Event::AccessKitActionRequest(ActionRequest { action: Action::Increment, target_tree, target_node, data: None }),
    ]);
    h.run();
    assert_eq!(h.state().kinds(), ["Set"]);
    assert!((shown(&h, Field::W) - 101.0).abs() < 1e-6, "{}", shown(&h, Field::W));
    assert!((shown(&h, Field::Angle) - 1.0).abs() < 1e-6, "{}", shown(&h, Field::Angle));
}

/// Without a host link the row keeps its own, and the toggle turns it off.
#[test]
fn the_row_keeps_its_own_link() {
    let mut h = harness_with(Direction::Ltr, |host| host.host_link = false);
    h.run();
    h.get_by_label("Maintain aspect ratio").click();
    h.run();
    h.get_by_label("Width").focus();
    h.run();
    h.key_press(Key::ArrowUp);
    h.run();
    let state = h.state().state;
    assert!((state.quad[1].x - state.quad[0].x - 202.0).abs() < 1e-6, "{state:?}");
    assert!((state.quad[3].y - state.quad[0].y - 100.0).abs() < 1e-6, "unlinked: the height stays: {state:?}");
}

/// The percent and degree units are the host's translated templates.
#[test]
fn the_units_are_the_host_s() {
    let mut h = harness_with(Direction::Ltr, |host| {
        host.labels.percent = Unit::new("%{n}");
        host.labels.degrees = Unit::new("{n} deg");
    });
    h.run();
    assert_eq!(h.get_by_label("Width").accesskit_node().value().as_deref(), Some("%100"));
    assert_eq!(h.get_by_label("Angle").accesskit_node().value().as_deref(), Some("0 deg"));
}
