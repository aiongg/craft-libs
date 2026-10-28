use egui::accesskit::Role;
use egui::{Context, Key};
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
    linked: bool,
    event: Option<EditEvent<BoxState>>,
}

fn harness(dir: Direction) -> Harness<'static, Host> {
    let host = Host { state: BoxState::from_rect(RECT), linked: true, event: None };
    Harness::<Host>::builder()
        .with_size([900.0, 200.0])
        .with_context_setup(move |ctx: &Context| {
            install(ctx, &Base::suite_default());
            direction::apply(ctx, dir);
        })
        .build_ui_state(
            |ui, host: &mut Host| {
                let edit = TransformFields::new("fields", &host.state, RECT, TransformLabels::default()).linked(&mut host.linked).show(ui);
                if edit.event.is_some() {
                    host.event = edit.event;
                }
            },
            host,
        )
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
    let Some(EditEvent::Set { value, .. }) = h.state().event.clone() else { panic!("{:?}", h.state().event) };
    // Linked, 101 % wide is 101 % high, about the centre.
    assert!((value.quad[1].x - value.quad[0].x - 202.0).abs() < 1e-6 && (value.quad[3].y - value.quad[0].y - 101.0).abs() < 1e-6, "{value:?}");
    assert!(close(value.pivot, BoxState::from_rect(RECT).pivot));
}
