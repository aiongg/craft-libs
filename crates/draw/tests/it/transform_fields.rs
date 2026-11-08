//! The transform fields and the transform bar: the box maths behind each field, typed values, gestures and the bar's
//! members.

use craft_behavior::geometry::quad::BoxState;
use craft_behavior::kurbo::{Point, Rect};
use craft_behavior::{EditEvent, RulerUnits, Ticks};
use craft_draw::ui::transform::{
    BarGroups, Interpolation, SizeUnit, TransformBar, TransformEdit, TransformFields, TransformRow, TransformSession, TransformSettings,
};
use craft_ui::canvas::RulerText;
use craft_ui::contract::{Bind, Entry, Show as _};
use craft_ui::runtime::env::UnitKind;
use craft_ui::testing::{Fixture, Target, pointer, trial};
use egui::{Key, Modifiers, Pos2, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

/// The box before the transform, in document units.
const START: Rect = Rect::new(0.0, 0.0, 200.0, 100.0);

/// Document units per inch.
const PIXELS_PER_INCH: f64 = 72.0;

struct Host {
    row: TransformRow,
    linked: bool,
    enabled: bool,
    events: Vec<EditEvent<TransformRow>>,
}

struct Config {
    bar: bool,
    size_unit: SizeUnit,
    units: Option<RulerText>,
    groups: BarGroups,
}

struct Harness {
    fixture: Fixture,
    host: Rc<RefCell<Host>>,
}

fn inches() -> RulerText {
    RulerText::new(RulerUnits::new(1.0 / PIXELS_PER_INCH, Ticks::decimal(false)), UnitKind::Inches)
}

fn plain() -> Config {
    Config { bar: false, size_unit: SizeUnit::default(), units: None, groups: BarGroups::ALL }
}

fn harness_of(row: TransformRow, config: Config) -> Harness {
    let host = Rc::new(RefCell::new(Host { row, linked: true, enabled: true, events: Vec::new() }));
    let shown = Rc::clone(&host);
    let mut fixture = Fixture::new(move |ui| {
        let mut host = shown.borrow_mut();
        let Host { row, linked, enabled, events } = &mut *host;
        let mut fields =
            TransformFields::new("Transform", Bind::bound(row), START, Bind::bound(linked)).size_unit(config.size_unit.clone()).enabled(*enabled);
        if config.bar {
            fields = fields.bar().groups(config.groups);
        }
        if let Some(units) = config.units {
            fields = fields.units(units, units);
        }
        let output = fields.show(ui);
        events.extend(output.event);
    });
    fixture.settle();
    Harness { fixture, host }
}

fn harness() -> Harness {
    harness_of(TransformRow::new(BoxState::from_rect(START), 0.0), plain())
}

impl Harness {
    fn row(&self) -> TransformRow {
        self.host.borrow().row
    }

    fn parts(&self) -> craft_behavior::geometry::quad::Parts {
        let row = self.row();
        row.state.parts(START, row.skew_v)
    }

    fn target(&self, label: &str) -> Target {
        self.fixture.by_label(label).unwrap_or_else(|| panic!("the fields show {label}"))
    }

    /// Replaces the text of the field named `label` with `text` and commits it with Enter.
    fn type_into(&mut self, label: &str, text: &str) {
        let field = self.target(label);
        self.fixture.click(&field);
        self.fixture.key(Key::A, Modifiers::COMMAND);
        self.fixture.type_text(text);
        self.fixture.key(Key::Enter, Modifiers::NONE);
        self.fixture.idle(3);
    }

    /// Presses the caption left of the field named `label`, drags it by `by` and leaves the button down.
    fn scrub(&mut self, label: &str, by: Vec2) {
        let field = self.target(label);
        let caption = Pos2::new(field.rect.min.x - 10.0, field.rect.center().y);
        self.fixture.frame(vec![pointer::moved(caption)]);
        self.fixture.frame(vec![pointer::press(caption)]);
        self.fixture.frame(vec![pointer::moved(caption + by * 0.5)]);
        self.fixture.frame(vec![pointer::moved(caption + by)]);
        self.fixture.idle(1);
    }

    fn release(&mut self) {
        self.fixture.frame(vec![pointer::release(Pos2::ZERO)]);
        self.fixture.idle(2);
    }
}

/// A screen displacement.
fn by(x: f32, y: f32) -> Vec2 {
    Vec2::new(x, y)
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-3
}

/// Typing a width scales the height with it, about the reference point.
#[trial]
fn a_linked_width_scales_the_height_about_the_reference_point() {
    let mut h = harness();
    let pivot = h.row().state.pivot;
    h.type_into("Width", "50");
    let parts = h.parts();
    assert!(near(parts.w, 0.5) && near(parts.h, 0.5), "{parts:?}");
    assert!(h.row().state.pivot.distance(pivot) < 1e-6, "the reference point stays");
}

/// The angle and skew fields turn and lean the box about the reference point.
#[trial]
fn the_angle_and_skew_fields_turn_and_lean_the_box_about_the_reference_point() {
    let mut h = harness();
    let pivot = h.row().state.pivot;
    h.type_into("Angle", "30");
    assert!(near(h.parts().angle, 30.0), "{:?}", h.parts());
    h.type_into("Horizontal skew", "20");
    assert!(near(h.parts().skew_h, 20.0) && near(h.parts().angle, 30.0), "{:?}", h.parts());
    assert!(h.row().state.pivot.distance(pivot) < 1e-6);
}

/// A flipped box reads as a negative width and keeps its sign through the other fields.
#[trial]
fn a_flipped_box_keeps_its_sign_through_the_fields() {
    let state = BoxState::from_rect(START);
    let mut parts = state.parts(START, 0.0);
    parts.w = -1.0;
    let mut h = harness_of(TransformRow::new(state.with_parts(START, 0.0, parts), 0.0), plain());
    assert!(h.parts().w < 0.0);
    h.type_into("Angle", "10");
    assert!(near(h.parts().w, -1.0) && near(h.parts().angle, 10.0), "{:?}", h.parts());
}

/// X and Y move the box with its reference point, and every field stops at its limit.
#[trial]
fn position_moves_the_box_with_its_reference_point_and_every_field_has_a_limit() {
    let mut h = harness();
    h.type_into("X position", "500");
    assert!(near(h.row().state.pivot.x, 500.0));
    h.type_into("X position", "99999999");
    assert!(h.row().state.pivot.x <= 300_000.0 + 1e-6, "{:?}", h.row().state.pivot);
    h.type_into("Angle", "999");
    assert!(h.parts().angle.abs() <= 180.0 + 1e-6, "{:?}", h.parts());
    h.type_into("Vertical skew", "500");
    assert!(h.row().skew_v.abs() <= craft_behavior::geometry::quad::SKEW_LIMIT + 1e-6, "{}", h.row().skew_v);
}

/// An arrow key on a field steps its value by one and reports a box.
#[trial]
fn an_arrow_step_reports_a_box() {
    let mut h = harness();
    let before = h.row().state.pivot.x;
    let field = h.target("X position");
    h.fixture.click(&field);
    h.fixture.key(Key::ArrowUp, Modifiers::NONE);
    h.fixture.idle(2);
    assert!(near(h.row().state.pivot.x, before + 1.0), "{:?}", h.row().state.pivot);
    assert!(matches!(h.host.borrow().events.last(), Some(EditEvent::Set { .. })));
}

/// Escape during a scrub of the V skew puts the skew back to the one the scrub began with.
#[trial]
fn a_cancelled_skew_scrub_restores_the_host_s_skew() {
    let mut h = harness_of(TransformRow::new(BoxState::from_rect(START), 5.0), plain());
    h.scrub("Vertical skew", by(40.0, 0.0));
    h.fixture.key(Key::Escape, Modifiers::NONE);
    h.release();
    assert!(near(h.row().skew_v, 5.0), "{}", h.row().skew_v);
}

/// A scrub the host ends by disabling the fields restores the skew the scrub began with.
#[trial]
fn a_skew_scrub_cancelled_by_the_host_restores_the_host_s_skew() {
    let mut h = harness_of(TransformRow::new(BoxState::from_rect(START), 5.0), plain());
    h.scrub("Vertical skew", by(40.0, 0.0));
    h.host.borrow_mut().enabled = false;
    h.fixture.idle(2);
    h.release();
    assert!(near(h.row().skew_v, 5.0), "{}", h.row().skew_v);
}

/// Unlinking the fields keeps the link in the host's binding, and an unlinked width leaves the height alone.
#[trial]
fn the_row_keeps_its_own_link() {
    let mut h = harness();
    let link = h.target("Maintain aspect ratio");
    h.fixture.click(&link);
    h.fixture.idle(2);
    assert!(!h.host.borrow().linked);
    h.type_into("Width", "50");
    assert!(near(h.parts().w, 0.5) && near(h.parts().h, 1.0), "{:?}", h.parts());
}

/// X and Y follow the units the host gives: a typed 1 in an inch field is 72 document units.
#[trial]
fn the_units_are_the_host_s() {
    let mut h = harness_of(TransformRow::new(BoxState::from_rect(START), 0.0), Config { units: Some(inches()), ..plain() });
    h.type_into("X position", "1");
    assert!(near(h.row().state.pivot.x, PIXELS_PER_INCH), "{:?}", h.row().state.pivot);
}

/// Hostile text: a negative width flips the box, a huge one stops at the limit, zero collapses it without a
/// non-finite value.
#[trial]
fn hostile_typed_lengths_flip_clamp_or_collapse() {
    let mut h = harness();
    h.type_into("Width", "-50");
    assert!(h.parts().w < 0.0, "{:?}", h.parts());
    h.type_into("Width", "99999999999");
    assert!(h.parts().w.is_finite() && h.parts().w.abs() <= 10_000.0 + 1e-6, "{:?}", h.parts());
    h.type_into("Width", "0");
    let row = h.row();
    assert!(row.state.quad.iter().all(|p| p.x.is_finite() && p.y.is_finite()), "{row:?}");
}

/// The bar form lays out the reference point, then the captioned cells in PhotoCraft's order.
#[trial]
fn the_bar_form_lays_out_the_reference_point_then_captioned_cells_in_photocraft_s_order() {
    let h = harness_of(TransformRow::new(BoxState::from_rect(START), 0.0), Config { bar: true, ..plain() });
    let order =
        ["Reference point", "X position", "Y position", "Width", "Maintain aspect ratio", "Height", "Angle", "Horizontal skew", "Vertical skew"];
    let xs: Vec<f32> = order.iter().map(|label| h.target(label).rect.min.x).collect();
    assert!(xs.windows(2).all(|pair| pair[0] < pair[1]), "{xs:?}");
}

/// A length typed in another unit sets the width: 40px in an inch field is 40 document units.
#[trial]
fn a_typed_length_in_another_unit_sets_the_width() {
    let length = SizeUnit::length(inches(), inches());
    let mut h = harness_of(TransformRow::new(BoxState::from_rect(START), 0.0), Config { size_unit: length, units: Some(inches()), ..plain() });
    let link = h.target("Maintain aspect ratio");
    h.fixture.click(&link);
    h.type_into("Width", "40px");
    assert!(near(h.parts().w * START.width(), 40.0), "{:?}", h.parts());
}

/// A bar of position and size only has just those cells, in the same order.
#[trial]
fn a_position_and_size_subset_lays_out_only_those_cells_in_photocraft_s_order() {
    let mut groups = BarGroups::ALL;
    (groups.reference, groups.angle, groups.skew, groups.interpolation) = (false, false, false, false);
    let h = harness_of(TransformRow::new(BoxState::from_rect(START), 0.0), Config { bar: true, groups, ..plain() });
    for absent in ["Reference point", "Angle", "Horizontal skew", "Vertical skew"] {
        assert!(h.fixture.by_label(absent).is_none(), "{absent} is hidden");
    }
    let xs: Vec<f32> =
        ["X position", "Y position", "Width", "Maintain aspect ratio", "Height"].iter().map(|label| h.target(label).rect.min.x).collect();
    assert!(xs.windows(2).all(|pair| pair[0] < pair[1]), "{xs:?}");
}

/// The text typed into W of a perspective box stays in the field until the field commits, whatever the host wrote
/// back.
#[trial]
fn typing_into_w_of_a_perspective_box_keeps_the_typed_text_through_the_hosts_write_back() {
    let quad = [Point::new(0.0, 0.0), Point::new(200.0, 0.0), Point::new(220.0, 100.0), Point::new(-20.0, 100.0)];
    let mut h = harness_of(TransformRow::new(BoxState::new(quad, Point::new(100.0, 50.0)), 0.0), plain());
    let field = h.target("Width");
    h.fixture.click(&field);
    h.fixture.key(Key::A, Modifiers::COMMAND);
    h.fixture.type_text("50");
    h.fixture.idle(3);
    let shown = h.fixture.tree().value_of(field.node).map(str::to_owned).unwrap_or_default();
    assert!(shown.contains("50"), "{shown}");
}

/// In percent mode a typed value is rounded to the percent decimals and no further.
#[trial]
fn a_typed_length_in_percent_mode_is_rounded_to_the_percent_decimals_only() {
    let mut h = harness_of(TransformRow::new(BoxState::from_rect(START), 0.0), Config { size_unit: SizeUnit::percent(1), ..plain() });
    h.type_into("Width", "33.349");
    assert!(near(h.parts().w * 100.0, 33.3), "{:?}", h.parts());
}

/// Typing 50 into W of a distorted box digit by digit gives the box typing it at once gives.
#[trial]
fn typing_50_into_w_of_a_distorted_box_equals_typing_50_once_from_the_start_box() {
    let quad = [Point::new(0.0, 0.0), Point::new(200.0, 0.0), Point::new(220.0, 100.0), Point::new(-20.0, 100.0)];
    let row = TransformRow::new(BoxState::new(quad, Point::new(100.0, 50.0)), 0.0);
    let mut once = harness_of(row, plain());
    once.type_into("Width", "50");
    let mut by_digit = harness_of(row, plain());
    let field = by_digit.target("Width");
    by_digit.fixture.click(&field);
    by_digit.fixture.key(Key::A, Modifiers::COMMAND);
    by_digit.fixture.type_text("5");
    by_digit.fixture.type_text("0");
    by_digit.fixture.key(Key::Enter, Modifiers::NONE);
    by_digit.fixture.idle(3);
    let (a, b) = (once.row(), by_digit.row());
    assert!(a.state.quad.iter().zip(&b.state.quad).all(|(p, q)| p.distance(*q) < 1e-6), "{a:?} {b:?}");
}

/// What the bar host keeps.
struct BarHost {
    session: TransformSession<()>,
    settings: TransformSettings,
    edits: Vec<TransformEdit<()>>,
    apply: bool,
    discard: bool,
    reset: bool,
    unit: Option<UnitKind>,
}

fn bar(warp: bool) -> (Fixture, Rc<RefCell<BarHost>>) {
    let mut session = TransformSession::<()>::new(START);
    if warp {
        let _ = session.apply(TransformEdit::Warp(()), 0.0);
    }
    let host = Rc::new(RefCell::new(BarHost {
        session,
        settings: TransformSettings::default(),
        edits: Vec::new(),
        apply: false,
        discard: false,
        reset: false,
        unit: None,
    }));
    let shown = Rc::clone(&host);
    let units = [Entry::new(UnitKind::Percent, "Percent"), Entry::new(UnitKind::Pixels, "Pixels"), Entry::new(UnitKind::Inches, "Inches")];
    let mut fixture = Fixture::new(move |ui| {
        let mut host = shown.borrow_mut();
        let BarHost { session, settings, edits, apply, discard, reset, unit } = &mut *host;
        let output = TransformBar::new("Free Transform", session, Bind::bound(settings)).size_units(&units).show(ui);
        edits.extend(output.edits);
        (*apply, *discard, *reset) = (*apply | output.apply, *discard | output.discard, *reset | output.reset_warp);
        *unit = unit.or(output.unit_picked);
    });
    fixture.settle();
    (fixture, host)
}

fn click(fixture: &mut Fixture, label: &str) {
    let target = fixture.by_label(label).unwrap_or_else(|| panic!("the bar shows {label}"));
    fixture.click(&target);
    fixture.idle(3);
}

/// The Box phase in reading order: the fields, the interpolation, the check, the switch, then cancel and commit.
#[trial]
fn the_bar_shows_fields_interpolation_check_and_buttons_in_phase_order() {
    let (fixture, _) = bar(false);
    let order = [
        "Reference point",
        "X position",
        "Width",
        "Angle",
        "Vertical skew",
        "Interpolation",
        "Show Reference Point",
        "Switch between free transform and warp modes",
        "Cancel transform",
        "Commit transform",
    ];
    let xs: Vec<f32> = order.iter().map(|label| fixture.by_label(label).unwrap_or_else(|| panic!("the bar shows {label}")).rect.min.x).collect();
    assert!(xs.windows(2).all(|pair| pair[0] < pair[1]), "{xs:?}");
}

/// The commit and cancel buttons carry the phase's names and report their presses.
#[trial]
fn commit_and_cancel_buttons_carry_the_phase_names() {
    let (mut fixture, host) = bar(false);
    assert!(fixture.by_label("Commit warp").is_none());
    click(&mut fixture, "Commit transform");
    assert!(host.borrow().apply);
    click(&mut fixture, "Cancel transform");
    assert!(host.borrow().discard);
    let (fixture, _) = bar(true);
    assert!(fixture.by_label("Commit warp").is_some() && fixture.by_label("Cancel warp").is_some());
}

/// The check and the interpolation report their edits and write the settings.
#[trial]
fn the_check_and_the_interpolation_report_edits_and_write_the_settings() {
    let (mut fixture, host) = bar(false);
    click(&mut fixture, "Show Reference Point");
    assert!(host.borrow().settings.show_reference_point);
    assert!(host.borrow().edits.iter().any(|edit| matches!(edit, TransformEdit::ReferencePoint(true))));
    click(&mut fixture, "Interpolation");
    click(&mut fixture, "Bilinear");
    assert_eq!(host.borrow().settings.interpolation, Interpolation::Bilinear);
    assert!(host.borrow().edits.iter().any(|edit| matches!(edit, TransformEdit::Interpolation(Interpolation::Bilinear))));
}

/// The switch asks for a warp in the Box phase and to leave it in the Warp phase; Reset warp asks for an identity warp.
#[trial]
fn the_switch_asks_for_a_warp_and_reset_asks_for_an_identity_warp() {
    let (mut fixture, host) = bar(false);
    click(&mut fixture, "Switch between free transform and warp modes");
    assert!(host.borrow().edits.iter().any(|edit| matches!(edit, TransformEdit::WarpRequested)));
    let (mut fixture, host) = bar(true);
    click(&mut fixture, "Reset warp");
    assert!(host.borrow().reset);
    click(&mut fixture, "Switch between free transform and warp modes");
    assert!(host.borrow().edits.iter().any(|edit| matches!(edit, TransformEdit::LeaveWarp)));
}

/// A unit picked in the W and H menu is written to the settings and reported.
#[trial]
fn a_unit_picked_in_the_size_menu_is_written_to_the_settings() {
    let (mut fixture, host) = bar(false);
    let width = fixture.by_label("Width").expect("the bar shows Width");
    fixture.click(&width);
    fixture.key(Key::F10, Modifiers::SHIFT);
    fixture.idle(3);
    click(&mut fixture, "Inches");
    assert_eq!(host.borrow().settings.size_unit, Some(UnitKind::Inches));
    assert_eq!(host.borrow().unit, Some(UnitKind::Inches));
}
