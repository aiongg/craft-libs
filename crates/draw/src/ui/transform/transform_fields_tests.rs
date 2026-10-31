use egui::accesskit::{Action, ActionRequest, Role};
use egui::{Context, Event, Key, Modifiers, PointerButton, Pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use kurbo::{Point, Rect};

use super::*;
use craft_ui::canvas::quad::Anchor9;
use craft_ui::direction::{self, Direction};
use craft_ui::edit::{self, CancelReason, EditEvent};
use craft_ui::theme::{Base, install};

const RECT: Rect = Rect::new(100.0, 50.0, 300.0, 150.0);

fn conversion(linked: bool) -> Conversion {
    let pixels = RulerUnits::pixels(LocaleNumbers::from_tag("en"));
    Conversion { rect: RECT, x: pixels.clone(), y: pixels, size: SizeUnit::default(), linked }
}

fn close(a: Point, b: Point) -> bool {
    (a - b).hypot() < 1e-6
}

#[test]
fn a_linked_width_scales_the_height_about_the_reference_point() {
    let state = BoxState::from_rect(RECT);
    let (linked, _) = conversion(true).edit(&state, 0.0, TransformField::W, 50.0);
    let (unlinked, _) = conversion(false).edit(&state, 0.0, TransformField::W, 50.0);
    assert!(close(linked.pivot, state.pivot), "the reference point stays put");
    assert!(close(linked.quad[0], Point::new(150.0, 75.0)) && close(linked.quad[2], Point::new(250.0, 125.0)), "{:?}", linked.quad);
    assert!(close(unlinked.quad[0], Point::new(150.0, 50.0)) && close(unlinked.quad[2], Point::new(250.0, 150.0)), "{:?}", unlinked.quad);
    // Through a corner reference point the opposite corner is fixed.
    let corner = BoxState { pivot: Point::new(RECT.x0, RECT.y0), ..state };
    let (scaled, _) = conversion(true).edit(&corner, 0.0, TransformField::H, 200.0);
    assert!(close(scaled.quad[0], Point::new(RECT.x0, RECT.y0)) && close(scaled.quad[2], Point::new(500.0, 250.0)), "{:?}", scaled.quad);
}

#[test]
fn the_angle_and_skew_fields_turn_and_lean_the_box_about_the_reference_point() {
    let state = BoxState::from_rect(RECT);
    let c = conversion(true);
    let (turned, _) = c.edit(&state, 0.0, TransformField::Angle, 30.0);
    assert!(close(turned.pivot, state.pivot));
    assert!((c.shown(&turned, 0.0, TransformField::Angle) - 30.0).abs() < 1e-9);
    assert!((c.shown(&turned, 0.0, TransformField::W) - 100.0).abs() < 1e-9, "turning keeps the size");
    let (leaned, _) = c.edit(&state, 0.0, TransformField::SkewH, 20.0);
    assert!(close(leaned.pivot, state.pivot) && close(leaned.point_at(Anchor9::Center), state.pivot), "{leaned:?}");
    assert!((c.shown(&leaned, 0.0, TransformField::SkewH) - 20.0).abs() < 1e-6);
    // The V skew is the host's: editing it returns it, and the box leans about the reference point.
    let (v, kept) = c.edit(&state, 0.0, TransformField::SkewV, 15.0);
    assert_eq!(kept, 15.0);
    assert!(close(v.point_at(Anchor9::Center), state.pivot));
    assert!((c.shown(&v, kept, TransformField::SkewV) - 15.0).abs() < 1e-9);
}

#[test]
fn a_flipped_box_keeps_its_sign_through_the_fields() {
    let c = conversion(false);
    let state = BoxState::from_rect(RECT);
    let (flipped, _) = c.edit(&state, 0.0, TransformField::W, -100.0);
    assert!(c.shown(&flipped, 0.0, TransformField::W) < 0.0 && c.shown(&flipped, 0.0, TransformField::H) > 0.0);
    for (field, value) in [(TransformField::Angle, 20.0), (TransformField::H, 50.0), (TransformField::SkewH, 10.0)] {
        let (edited, _) = c.edit(&flipped, 0.0, field, value);
        assert!(c.shown(&edited, 0.0, TransformField::W) < 0.0, "{field:?} left the flip");
    }
    let (turned, _) = c.edit(&flipped, 0.0, TransformField::Angle, 20.0);
    assert!((c.shown(&turned, 0.0, TransformField::W) + 100.0).abs() < 1e-6);
}

#[test]
fn position_moves_the_box_with_its_reference_point_and_every_field_has_a_limit() {
    let c = conversion(true);
    let state = BoxState::from_rect(RECT);
    let (moved, _) = c.edit(&state, 0.0, TransformField::X, 10.0);
    assert!(close(moved.pivot, Point::new(10.0, state.pivot.y)) && close(moved.quad[0], Point::new(-90.0, 50.0)), "{moved:?}");
    let (moved, _) = c.edit(&moved, 0.0, TransformField::Y, -5.0);
    assert!(close(moved.pivot, Point::new(10.0, -5.0)));
    assert_eq!(
        [TransformField::X, TransformField::Y, TransformField::Angle, TransformField::SkewH, TransformField::SkewV].map(|f| c.bound(f)),
        [300_000.0, 300_000.0, 180.0, 89.0, 89.0]
    );
    // A 200 px side stays within 300 000 px: 1500 times, 150 000 %.
    assert_eq!(c.bound(TransformField::W), 150_000.0);
    // Linked, the other side follows within its own limit: H (100 px) allows 3000 times, W 1500.
    let (huge, _) = c.edit(&state, 0.0, TransformField::H, 1.0e9);
    let (w, h) = (c.shown(&huge, 0.0, TransformField::W), c.shown(&huge, 0.0, TransformField::H));
    assert!((w / 100.0 - 1500.0).abs() < 1e-3 && (h / 100.0 - 1500.0).abs() < 1e-3, "{w} {h}");
    // In lengths, the box's side in the unit.
    let px = RulerUnits::pixels(LocaleNumbers::from_tag("en"));
    let length = Conversion { size: SizeUnit::Length { x: px.clone(), y: px }, ..conversion(false) };
    assert_eq!(length.shown(&state, 0.0, TransformField::W), 200.0);
    let (wide, _) = length.edit(&state, 0.0, TransformField::W, 400.0);
    assert!((length.shown(&wide, 0.0, TransformField::W) - 400.0).abs() < 1e-9);
}

struct Host {
    state: BoxState,
    /// The box as the running gesture began, to restore on a cancel.
    before: Option<TransformRow>,
    linked: bool,
    /// Whether the host gives the row the link; without it the row keeps its own.
    host_link: bool,
    skew_v: f64,
    /// Whether the row takes input.
    enabled: bool,
    labels: TransformFieldLabels,
    events: Vec<EditEvent<TransformRow>>,
    /// The egui pass in which each event was reported.
    passes: Vec<u64>,
}

impl Host {
    fn event(&self) -> Option<&EditEvent<TransformRow>> {
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
                _ => "other",
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
        enabled: true,
        labels: TransformFieldLabels::default(),
        events: Vec::new(),
        passes: Vec::new(),
    };
    setup(&mut host);
    Harness::<Host>::builder()
        .with_size([900.0, 200.0])
        // A press that waits for a typing run repaints until it runs.
        .with_max_steps(8)
        .with_context_setup(move |ctx: &Context| {
            install(ctx, &Base::suite_default());
            direction::apply(ctx, dir);
        })
        .build_ui_state(
            |ui, host: &mut Host| {
                let state = host.state;
                let fields = TransformFields::new("fields", &state, RECT, host.labels.clone()).skew_v(host.skew_v).enabled(host.enabled);
                let edit = if host.host_link { fields.linked(&mut host.linked).show(ui) } else { fields.show(ui) };
                // A host that applies every update live, as a box session does.
                match &edit.event {
                    Some(EditEvent::Begin { .. }) => host.before = Some(TransformRow { state: host.state, skew_v: host.skew_v }),
                    Some(EditEvent::Update { value, .. } | EditEvent::Set { value, .. } | EditEvent::Commit { value, .. }) => {
                        (host.state, host.skew_v) = (value.state, value.skew_v)
                    }
                    Some(EditEvent::Cancel { .. }) => {
                        if let Some(start) = host.before.take() {
                            (host.state, host.skew_v) = (start.state, start.skew_v);
                        }
                    }
                    Some(_) | None => {}
                }
                host.passes.extend(edit.event.iter().map(|_| ui.ctx().cumulative_pass_nr()));
                host.events.extend(edit.event);
            },
            host,
        )
}

fn harness(dir: Direction) -> Harness<'static, Host> {
    harness_with(dir, |_| {})
}

fn centre<S>(h: &Harness<'_, S>, name: &str) -> Pos2 {
    h.get_by_label(name).rect().center()
}

fn button(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

/// Scrubs the field `name` by `steps` moves of `dx` points, a frame each; the button stays down.
fn scrub<S>(h: &mut Harness<'_, S>, name: &str, dx: f32, steps: usize) -> Pos2 {
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

fn release<S>(h: &mut Harness<'_, S>, at: Pos2) {
    h.event(button(at, false));
    h.run();
}

fn shown(h: &Harness<'_, Host>, field: TransformField) -> f64 {
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
    let value = value.state;
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
    assert!((shown(&h, TransformField::Angle) - 36.0).abs() < 1e-6, "{}", shown(&h, TransformField::Angle));
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

/// A cancel that doesn't come from the row (the host's undo mid-gesture through `cancel_all`,
/// or disabling the row) restores the V skew too.
#[test]
fn a_skew_scrub_cancelled_by_the_host_restores_the_host_s_skew() {
    for disable in [false, true] {
        let mut h = harness_with(Direction::Ltr, |host| host.skew_v = 5.0);
        h.run();
        let at = scrub(&mut h, "Vertical skew", 3.0, 4);
        assert!((h.state().skew_v - 17.0).abs() < 1e-9, "disable: {disable}: {}", h.state().skew_v);
        if disable {
            h.state_mut().enabled = false;
        } else {
            edit::cancel_all(&h.ctx, CancelReason::UndoDuringGesture);
        }
        h.run();
        assert!(matches!(h.state().event(), Some(EditEvent::Cancel { .. })), "disable: {disable}: {:?}", h.state().kinds());
        assert_eq!(h.state().skew_v, 5.0, "disable: {disable}");
        assert_eq!(h.state().state, BoxState::from_rect(RECT), "disable: {disable}");
        release(&mut h, at);
    }
}

/// A scrub on another field while a typing run is open waits for the run: the run commits
/// first, and the scrub is a gesture of its own from the box the run left. The angle's scrub
/// changes what W reads, so W's run would otherwise end as moved by something else.
#[test]
fn a_scrub_during_another_field_s_typing_run_waits_for_its_commit() {
    let mut h = harness(Direction::Ltr);
    h.run();
    let w = centre(&h, "Width");
    h.event(Event::PointerMoved(w));
    h.event(button(w, true));
    h.event(button(w, false));
    h.run();
    h.event(Event::Text("120".into()));
    h.run();
    let at = scrub(&mut h, "Angle", 3.0, 7);
    release(&mut h, at);
    let kinds = h.state().kinds();
    let run = kinds.iter().position(|k| *k == "Commit").expect("the run commits");
    assert!(kinds[0] == "Begin" && kinds[1..run].iter().all(|k| *k == "Update"), "{kinds:?}");
    let Some(EditEvent::Commit { value, .. }) = h.state().events.get(run) else { panic!("{kinds:?}") };
    let value = &value.state;
    let c = conversion(true);
    assert!(
        (c.shown(value, 0.0, TransformField::W) - 120.0).abs() < 1e-6 && c.shown(value, 0.0, TransformField::Angle).abs() < 1e-6,
        "the run's commit: {value:?}"
    );
    assert_eq!(kinds[run + 1], "Begin", "{kinds:?}");
    let passes = &h.state().passes;
    assert_eq!(passes[run + 1], passes[run] + 1, "the scrub begins in the pass after the run's commit: {passes:?}");
    assert_eq!(kinds.last(), Some(&"Commit"), "{kinds:?}");
    assert!(!kinds.contains(&"Cancel"), "{kinds:?}");
    assert_eq!(kinds.iter().filter(|k| **k == "Begin").count(), 2, "{kinds:?}");
    assert!((shown(&h, TransformField::W) - 120.0).abs() < 1e-6, "{}", shown(&h, TransformField::W));
    assert!(shown(&h, TransformField::Angle) > 1.0, "the scrub turned the box: {}", shown(&h, TransformField::Angle));
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
    assert!((shown(&h, TransformField::W) - 101.0).abs() < 1e-6, "{}", shown(&h, TransformField::W));
    assert!((shown(&h, TransformField::Angle) - 1.0).abs() < 1e-6, "{}", shown(&h, TransformField::Angle));
}

/// A screen reader's step on Width in the same frame as an update of an Angle scrub waits for the
/// release: the scrub's update, mapped after X in drawing order, doesn't carry the step.
#[test]
fn a_step_on_an_earlier_field_in_a_scrub_update_frame_waits_for_the_release() {
    let mut h = harness(Direction::Ltr);
    h.run();
    let x = shown(&h, TransformField::W);
    let at = scrub(&mut h, "Angle", 3.0, 3);
    let (target_node, target_tree) = h.get_by_label("Width").accesskit_node().locate();
    let to = at + vec2(3.0, 0.0);
    h.input_mut().events.extend([
        Event::AccessKitActionRequest(ActionRequest { action: Action::Increment, target_tree, target_node, data: None }),
        Event::PointerMoved(to),
    ]);
    h.step();
    assert!((shown(&h, TransformField::W) - x).abs() < 1e-6, "the step waits: {} from {x}", shown(&h, TransformField::W));
    release(&mut h, to);
    let kinds = h.state().kinds();
    assert_eq!(kinds[kinds.len() - 2..], ["Commit", "Set"], "{kinds:?}");
    assert!((shown(&h, TransformField::W) - (x + 1.0)).abs() < 1e-6, "the step applies after the release: {}", shown(&h, TransformField::W));
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

/// A document whose undo history holds whole snapshots: the box, the V skew and another value.
#[derive(Clone, Copy)]
struct Doc {
    row: TransformRow,
    other: i32,
}

struct History {
    now: Doc,
    undo: Vec<Doc>,
}

impl History {
    fn apply(&mut self, edit: impl FnOnce(&mut Doc)) {
        self.undo.push(self.now);
        edit(&mut self.now);
    }
}

impl edit::UndoMarks for History {
    type Mark = usize;

    fn undo_mark(&self) -> usize {
        self.undo.len()
    }

    fn revert_to(&mut self, mark: usize) {
        if let Some(&before) = self.undo.get(mark) {
            self.now = before;
            self.undo.truncate(mark);
        }
    }

    fn squash_since(&mut self, mark: usize) {
        self.undo.truncate(mark + 1);
    }
}

/// A V skew scrub that the host ends as a cancel because another control sets a value: the
/// document reverts both the box and the V skew, and the row reads the reverted values.
#[test]
fn a_skew_scrub_ended_by_the_host_for_another_control_reads_the_reverted_row() {
    let start = TransformRow { state: BoxState::from_rect(RECT), skew_v: 5.0 };
    let hosted = (History { now: Doc { row: start, other: 0 }, undo: Vec::new() }, edit::GestureHost::<History>::new(), false);
    let mut h = Harness::<(History, edit::GestureHost<History>, bool)>::builder()
        .with_size([900.0, 200.0])
        .with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, (doc, host, set): &mut (History, edit::GestureHost<History>, bool)| {
                let ctx = ui.ctx().clone();
                let row = doc.now.row;
                let edit = TransformFields::new("fields", &row.state, RECT, TransformFieldLabels::default()).skew_v(row.skew_v).show(ui);
                if let Some(event) = edit.event {
                    host.handle(&ctx, doc, event, |doc: &mut History, v: TransformRow| doc.apply(|now| now.row = v));
                }
                if std::mem::take(set) {
                    host.handle(&ctx, doc, EditEvent::set(7, egui::Modifiers::NONE), |doc: &mut History, v: i32| doc.apply(|now| now.other = v));
                }
            },
            hosted,
        );
    h.run();
    let at = scrub(&mut h, "Vertical skew", 3.0, 4);
    assert!((h.state().0.now.row.skew_v - 17.0).abs() < 1e-9, "{}", h.state().0.now.row.skew_v);
    h.state_mut().2 = true;
    h.run();
    h.run();
    let doc = h.state().0.now;
    assert_eq!(doc.other, 7);
    assert_eq!(doc.row, start);
    let c = conversion(true);
    assert_eq!(c.shown(&doc.row.state, doc.row.skew_v, TransformField::SkewV), 5.0);
    release(&mut h, at);
    assert_eq!(h.state().0.now.row, start);
}

/// PC `w_and_h_reject_or_clamp_hostile_typed_lengths`, on the fields' conversion: a negative
/// length mirrors the typed side, a huge one stops at the position limit, zero collapses.
#[test]
fn hostile_typed_lengths_flip_clamp_or_collapse() {
    let px = RulerUnits::pixels(LocaleNumbers::from_tag("en"));
    let length = Conversion { size: SizeUnit::Length { x: px.clone(), y: px }, ..conversion(true) };
    let state = BoxState::from_rect(RECT);
    let side = |s: &BoxState| (s.quad[1].x - s.quad[0].x, s.quad[3].y - s.quad[0].y);
    let (flipped, _) = length.edit(&state, 0.0, TransformField::W, -40.0);
    let (w, h) = side(&flipped);
    assert!((w + 40.0).abs() < 1e-6 && (h - 20.0).abs() < 1e-6, "{w} × {h}");
    let (huge, _) = length.edit(&state, 0.0, TransformField::W, 1e12);
    let (w, h) = side(&huge);
    assert!(w.is_finite() && w <= POSITION_LIMIT + 0.01 && h.abs() <= POSITION_LIMIT + 0.01, "{w} × {h}");
    let (zero, _) = length.edit(&state, 0.0, TransformField::W, 0.0);
    assert!(zero.quad.iter().all(|q| q.is_finite()));
}

#[test]
fn the_bar_form_lays_out_the_reference_point_then_captioned_cells_in_photocraft_s_order() {
    struct Bar {
        state: BoxState,
        interpolation: String,
        picked: Option<usize>,
    }
    let options = [("bicubic".to_owned(), "Bicubic".to_owned()), ("nearest".to_owned(), "Nearest Neighbor".to_owned())];
    let entries = [];
    let mut h =
        Harness::<Bar>::builder().with_size([1400.0, 80.0]).with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, bar: &mut Bar| {
                let state = bar.state;
                let _ = TransformFields::new("bar", &state, RECT, TransformFieldLabels::default())
                    .size_menu(&entries, &mut bar.picked)
                    .interpolation("Interpolation", &mut bar.interpolation, &options)
                    .show_bar(ui);
            },
            Bar { state: BoxState::from_rect(RECT), interpolation: "bicubic".into(), picked: None },
        );
    h.run();
    let x = |h: &Harness<'_, Bar>, label: &str, n: usize| h.get_all_by_label(label).nth(n).unwrap_or_else(|| panic!("{label} {n}")).rect().left();
    let order = [
        x(&h, "X:", 0),
        x(&h, "X position", 0),
        x(&h, "Y:", 0),
        x(&h, "Y position", 0),
        x(&h, "W:", 0),
        x(&h, "Width", 0),
        x(&h, "Maintain aspect ratio", 0),
        x(&h, "H:", 0),
        x(&h, "Height", 0),
        x(&h, "Angle", 0),
        x(&h, "H:", 1),
        x(&h, "Horizontal skew", 0),
        x(&h, "V:", 0),
        x(&h, "Vertical skew", 0),
        x(&h, "Interpolation:", 0),
    ];
    assert!(order.windows(2).all(|w| w[0] < w[1]), "{order:?}");
    // The reference point comes before the first cell.
    let reference = h.get_all_by_role(Role::RadioButton).chain(h.get_all_by_role(Role::Button)).map(|n| n.rect().left()).fold(f32::MAX, f32::min);
    assert!(reference < order[0], "{reference} {order:?}");
}

struct Typed {
    state: BoxState,
    size: SizeUnit,
    groups: BarGroups,
}

/// A bar whose host parser reads "<n>px" as a length of the 200 px wide box and "<n>%" as that
/// share of the whole side, and the state after `text` is typed into W and committed.
fn typed_into_width(size: SizeUnit, text: &str) -> BoxState {
    let mut h = Harness::<Typed>::builder()
        .with_size([1400.0, 80.0])
        .with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, host: &mut Typed| {
                let state = host.state;
                let edit = TransformFields::new("bar", &state, RECT, TransformFieldLabels::default())
                    .size_unit(host.size.clone())
                    .groups(host.groups)
                    .parser(|field, text, context| {
                        assert!(field != TransformField::W || context.whole.is_some());
                        let number = |suffix: &str| text.strip_suffix(suffix)?.trim().parse::<f64>().ok();
                        match (number("px"), number("%")) {
                            (Some(px), _) => Some(px / 200.0 * context.whole?),
                            (_, Some(share)) => Some(share / 100.0 * context.whole?),
                            _ => None,
                        }
                    })
                    .show_bar(ui);
                if let Some(EditEvent::Update { value, .. } | EditEvent::Set { value, .. } | EditEvent::Commit { value, .. }) = edit.event {
                    host.state = value.state;
                }
            },
            Typed { state: BoxState::from_rect(RECT), size, groups: BarGroups::ALL },
        );
    h.run();
    let w = centre(&h, "Width");
    h.event(Event::PointerMoved(w));
    h.event(button(w, true));
    h.event(button(w, false));
    h.run();
    h.event(Event::Key { key: Key::A, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::COMMAND });
    h.event(Event::Text(text.into()));
    h.run();
    h.key_press(Key::Enter);
    h.run();
    h.state().state
}

fn width_of(state: &BoxState) -> f64 {
    state.quad[1].x - state.quad[0].x
}

#[test]
fn a_host_parser_turns_a_typed_length_into_the_width_and_hostile_text_is_rejected() {
    let px = RulerUnits::pixels(LocaleNumbers::from_tag("en"));
    let length = || SizeUnit::Length { x: px.clone(), y: px.clone() };
    assert!((width_of(&typed_into_width(SizeUnit::default(), "50px")) - 50.0).abs() < 1e-6);
    assert!((width_of(&typed_into_width(length(), "50%")) - 100.0).abs() < 1e-6);
    for text in ["px", "NaN", "inf%", "1e999px"] {
        assert!((width_of(&typed_into_width(SizeUnit::default(), text)) - 200.0).abs() < 1e-6, "{text}");
    }
    let huge = width_of(&typed_into_width(SizeUnit::default(), "1e12px"));
    assert!(huge.is_finite() && huge <= POSITION_LIMIT + 0.01, "{huge}");
}

#[test]
fn a_position_and_size_subset_lays_out_only_those_cells_in_photocraft_s_order() {
    let mut h = Harness::<Typed>::builder()
        .with_size([1400.0, 80.0])
        .with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, host: &mut Typed| {
                let state = host.state;
                let _ = TransformFields::new("bar", &state, RECT, TransformFieldLabels::default()).groups(host.groups).show_bar(ui);
            },
            Typed {
                state: BoxState::from_rect(RECT),
                size: SizeUnit::default(),
                groups: BarGroups { angle: false, skew: false, interpolation: false, ..BarGroups::ALL },
            },
        );
    h.run();
    for absent in ["Angle", "Horizontal skew", "Vertical skew"] {
        assert!(h.query_by_label(absent).is_none(), "{absent}");
    }
    let x = |label: &str| h.get_all_by_label(label).next().unwrap_or_else(|| panic!("{label}")).rect().left();
    let order = [x("X position"), x("Y position"), x("Width"), x("Maintain aspect ratio"), x("Height")];
    assert!(order.windows(2).all(|w| w[0] < w[1]), "{order:?}");
}

/// A perspective box does not read back what a W edit sent (PhotoCraft: typing 5 reads back
/// 4.9 %). The field keeps the typed text through the host's write-back, whatever the number of
/// passes the host takes to apply an update, and the commit carries what was typed.
#[test]
fn typing_into_w_of_a_perspective_box_keeps_the_typed_text_through_the_hosts_write_back() {
    const BOX: Rect = Rect::new(0.0, 0.0, 100.0, 50.0);
    let distorted = BoxState {
        quad: [Point::new(0.0, 0.0), Point::new(100.0, 10.0), Point::new(90.0, 60.0), Point::new(-5.0, 50.0)],
        pivot: Point::new(50.0, 25.0),
    };
    let pixels = RulerUnits::pixels(LocaleNumbers::from_tag("en"));
    let convert = Conversion { rect: BOX, x: pixels.clone(), y: pixels, size: SizeUnit::default(), linked: true };
    for delay in 0..5usize {
        let applied = std::rc::Rc::new(std::cell::RefCell::new(Vec::<(u64, BoxState)>::new()));
        let queue = applied.clone();
        let mut h = Harness::<Typed>::builder()
            .with_size([1400.0, 80.0])
            .with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default()))
            .build_ui_state(
                move |ui, host: &mut Typed| {
                    let mut queue = queue.borrow_mut();
                    let pass = ui.ctx().cumulative_pass_nr();
                    while queue.first().is_some_and(|(sent, _)| pass >= sent + delay as u64) {
                        host.state = queue.remove(0).1;
                    }
                    let state = host.state;
                    let edit = TransformFields::new("bar", &state, BOX, TransformFieldLabels::default()).groups(host.groups).show_bar(ui);
                    if let Some(EditEvent::Begin { value: Some(value), .. } | EditEvent::Update { value, .. } | EditEvent::Commit { value, .. }) =
                        edit.event
                    {
                        queue.push((pass, value.state));
                    }
                },
                Typed { state: distorted, size: SizeUnit::default(), groups: BarGroups::ALL },
            );
        h.run();
        let w = centre(&h, "Width");
        h.event(Event::PointerMoved(w));
        h.event(button(w, true));
        h.event(button(w, false));
        h.run();
        h.event(Event::Key { key: Key::A, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::COMMAND });
        let text = |h: &Harness<'_, Typed>| h.get_by_label("Width").accesskit_node().value();
        for (typed, shown) in [("5", "5%"), ("0", "50%")] {
            h.event(Event::Text(typed.into()));
            for _ in 0..4 {
                h.step();
                assert_eq!(text(&h).as_deref(), Some(shown), "delay {delay}");
            }
        }
        assert_eq!(text(&h).as_deref(), Some("50%"), "delay {delay}");
        h.key_press(Key::Enter);
        for _ in 0..6 {
            h.step();
        }
        let once = convert.edit(&distorted, 0.0, TransformField::W, 50.0).0;
        let quad = h.state().state.quad;
        assert!(quad.iter().zip(once.quad).all(|(a, b)| close(*a, b)), "delay {delay}: {quad:?} {:?}", once.quad);
    }
}

/// Types `text` into W of a percent bar over `rect`, the host applying each report in the pass it
/// arrives, and returns the box after the commit.
fn typed_into_percent_width(rect: Rect, state: BoxState, text: &[&str], parser: Option<fn(&str, f64) -> Option<f64>>) -> BoxState {
    let mut h = Harness::<BoxState>::builder()
        .with_size([1400.0, 80.0])
        .with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default()))
        .build_ui_state(
            move |ui, host: &mut BoxState| {
                let state = *host;
                let mut fields = TransformFields::new("bar", &state, rect, TransformFieldLabels::default());
                if let Some(parser) = parser {
                    fields = fields.parser(move |field, text, context| {
                        context.whole.filter(|_| field == TransformField::W).and_then(|whole| parser(text, whole))
                    });
                }
                let edit = fields.show_bar(ui);
                if let Some(EditEvent::Begin { value: Some(value), .. } | EditEvent::Update { value, .. } | EditEvent::Commit { value, .. }) =
                    edit.event
                {
                    *host = value.state;
                }
            },
            state,
        );
    h.run();
    let w = centre(&h, "Width");
    h.event(Event::PointerMoved(w));
    h.event(button(w, true));
    h.event(button(w, false));
    h.run();
    h.event(Event::Key { key: Key::A, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::COMMAND });
    for typed in text {
        h.event(Event::Text((*typed).into()));
        for _ in 0..4 {
            h.step();
        }
    }
    h.key_press(Key::Enter);
    for _ in 0..6 {
        h.step();
    }
    *h.state()
}

#[test]
fn a_typed_length_in_percent_mode_is_rounded_to_the_percent_decimals_only() {
    // 5 cm at 300 dpi is 590.5512 px of a 100 px side.
    let rect = Rect::new(0.0, 0.0, 100.0, 50.0);
    let state = typed_into_percent_width(
        rect,
        BoxState::from_rect(rect),
        &["5cm"],
        Some(|text, whole| {
            let cm = text.strip_suffix("cm")?.parse::<f64>().ok()?;
            Some(cm / 2.54 * 300.0 / 100.0 * whole)
        }),
    );
    let width = width_of(&state);
    assert!((width - 590.55).abs() < 1e-9, "{width}");
}

#[test]
fn typing_50_into_w_of_a_distorted_box_equals_typing_50_once_from_the_start_box() {
    let rect = Rect::new(0.0, 0.0, 100.0, 50.0);
    let distorted = BoxState {
        quad: [Point::new(0.0, 0.0), Point::new(100.0, 10.0), Point::new(90.0, 60.0), Point::new(-5.0, 50.0)],
        pivot: Point::new(50.0, 25.0),
    };
    let state = typed_into_percent_width(rect, distorted, &["5", "0"], None);
    let upstream = [(25.300, 12.650), (74.570, 17.629), (70.164, 42.643), (22.139, 37.664)];
    for (corner, (x, y)) in state.quad.iter().zip(upstream) {
        assert!((corner.x - x).abs() < 5e-4 && (corner.y - y).abs() < 5e-4, "{:?}", state.quad);
    }
}
