//! The transform fields and the transform bar: the box maths behind each field, typed values, gestures and the bar's
//! members.

use craft_behavior::geometry::quad::BoxState;
use craft_behavior::kurbo::{Point, Rect};
use craft_behavior::{EditEvent, RulerUnits, Ticks};
use craft_draw::ui::transform::{BarGroups, SizeUnit, TransformFields, TransformRow};
use craft_ui::canvas::RulerText;
use craft_ui::contract::Bind;
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
        events.extend(output.events);
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
