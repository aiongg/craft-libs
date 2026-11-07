//! `AngleDial` points at the pointer in its orientation and steps round or against the ends of its range;
//! `PointPicker` maps a press to the point on the well and moves one axis per arrow key.

use craft_ui::contract::{Bind, Props as _, Show as _};
use craft_ui::testing::pointer::{moved, press, release};
use craft_ui::testing::{Fixture, Story, StoryData, StoryOutput, trial};
use craft_ui::value_drag::{AngleDial, AngleOrientation, PointPicker};
use egui::{Key, Modifiers, Pos2, Rect, Ui, Vec2};

/// How far from the dial's centre a click lands: inside the face, outside the dead zone.
const REACH: f32 = 14.0;
const FULL: std::ops::RangeInclusive<f64> = 0.0..=100.0;

fn counter_clockwise(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    AngleDial::new(Bind::bound(data.value("angle", 45.0))).label("Angle").show(ui).into()
}

fn clockwise(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    AngleDial::new(Bind::bound(data.value("angle", 45.0))).orientation(AngleOrientation::Clockwise).label("Angle").show(ui).into()
}

fn at_turn_end(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    AngleDial::new(Bind::bound(data.value("angle", 180.0))).label("Angle").show(ui).into()
}

fn quarter_end(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    AngleDial::new(Bind::bound(data.value("angle", 90.0))).range(0.0..=90.0).label("Angle").show(ui).into()
}

fn quarter_end_wrapping(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    AngleDial::new(Bind::bound(data.value("angle", 90.0))).range(0.0..=90.0).wrap(true).label("Angle").show(ui).into()
}

fn picker(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    PointPicker::new(Bind::bound(data.value("point", [50.0, 50.0])), FULL, FULL).label("Blur center").show(ui).into()
}

fn shown(show: fn(&mut Ui, &mut StoryData) -> StoryOutput) -> Fixture {
    let mut fixture = Fixture::story(&Story::new("dials", show));
    fixture.settle();
    fixture
}

fn control_rect(fixture: &Fixture) -> Rect {
    fixture.root_id().and_then(|id| fixture.node_of(id)).map(|target| target.rect).expect("the control has a node")
}

fn click(fixture: &mut Fixture, at: Pos2) {
    fixture.frame(vec![moved(at)]);
    fixture.frame(vec![press(at)]);
    fixture.frame(vec![release(at)]);
    fixture.idle(2);
}

fn angle(fixture: &Fixture) -> f64 {
    *fixture.data().value("angle", f64::NAN)
}

fn point(fixture: &Fixture) -> [f64; 2] {
    *fixture.data().value("point", [f64::NAN; 2])
}

fn key(fixture: &mut Fixture, key: Key, modifiers: Modifiers) {
    fixture.key(key, modifiers);
    fixture.idle(2);
}

fn focused(show: fn(&mut Ui, &mut StoryData) -> StoryOutput) -> Fixture {
    let mut fixture = shown(show);
    key(&mut fixture, Key::Tab, Modifiers::NONE);
    fixture
}

#[trial]
fn a_dial_points_at_the_pointer_from_its_zero_in_its_direction() {
    let mut up = shown(counter_clockwise);
    let center = control_rect(&up).center();
    click(&mut up, center + Vec2::new(0.0, -REACH));
    assert_eq!(angle(&up), 90.0, "counter-clockwise, 0 at 3 o'clock: up is 90");
    click(&mut up, center + Vec2::new(0.0, REACH));
    assert_eq!(angle(&up), -90.0);

    let mut right = shown(clockwise);
    let center = control_rect(&right).center();
    click(&mut right, center + Vec2::new(REACH, 0.0));
    assert_eq!(angle(&right), 90.0, "clockwise, 0 at 12 o'clock: right is 90");
}

#[trial]
fn a_dial_wraps_round_a_turn_and_stops_at_the_ends_of_a_narrower_range() {
    let mut turn = focused(at_turn_end);
    key(&mut turn, Key::ArrowRight, Modifiers::NONE);
    assert_eq!(angle(&turn), -179.0, "one step past 180 continues from -180 by the overshoot");

    let mut stops = focused(quarter_end);
    key(&mut stops, Key::ArrowRight, Modifiers::NONE);
    assert_eq!(angle(&stops), 90.0);

    let mut wraps = focused(quarter_end_wrapping);
    key(&mut wraps, Key::ArrowRight, Modifiers::NONE);
    assert_eq!(angle(&wraps), 1.0, "a range that wraps by choice continues from its start");
}

#[trial]
fn a_point_picker_maps_a_click_to_the_range_with_y_growing_downwards() {
    let mut fixture = shown(picker);
    let well = control_rect(&fixture);
    click(&mut fixture, well.lerp_inside(Vec2::new(0.25, 0.75)));
    let [x, y] = point(&fixture);
    assert!((x - 25.0).abs() < 0.1 && (y - 75.0).abs() < 0.1, "a click a quarter across and three quarters down is (25, 75), not ({x}, {y})");
}

#[trial]
fn a_point_picker_moves_one_axis_per_arrow_key() {
    let mut fixture = focused(picker);
    key(&mut fixture, Key::ArrowRight, Modifiers::NONE);
    assert_eq!(point(&fixture), [51.0, 50.0]);
    key(&mut fixture, Key::ArrowDown, Modifiers::NONE);
    assert_eq!(point(&fixture), [51.0, 51.0]);
    key(&mut fixture, Key::ArrowUp, Modifiers::SHIFT);
    assert_eq!(point(&fixture), [51.0, 41.0], "Shift moves ten steps, and up lowers y");
}
