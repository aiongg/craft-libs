//! `AngleDial` points at the pointer in its orientation and steps round or against the ends of its range;
//! `PointPicker` maps a press to the point on the well and moves one axis per arrow key.

use craft_behavior::Mixed;
use craft_i18n::Direction;
use craft_ui::contract::{Bind, Props as _, Role, Show as _};
use craft_ui::testing::access::Declared;
use craft_ui::testing::focus::AccessActionKind;
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
    PointPicker::new(Bind::bound(data.value("point", Mixed::each([50.0, 50.0]))), FULL, FULL).label("Blur center").show(ui).into()
}

fn picker_y_up(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    PointPicker::new(Bind::bound(data.value("point", Mixed::each([50.0, 50.0]))), FULL, 100.0..=0.0).label("Blur center").show(ui).into()
}

fn shown(show: fn(&mut Ui, &mut StoryData) -> StoryOutput) -> Fixture {
    let mut fixture = Fixture::story(&Story::new("dials", show));
    fixture.settle();
    fixture
}

fn control_rect(fixture: &Fixture) -> Rect {
    fixture.root_id().and_then(|id| fixture.node_of(id)).map(|target| target.rect).expect("the control has a node")
}

/// Press at `from`, move through `through` and release at the last point.
fn drag_through(fixture: &mut Fixture, from: Pos2, through: &[Pos2]) {
    fixture.frame(vec![moved(from)]);
    fixture.frame(vec![press(from)]);
    for at in through {
        fixture.frame(vec![moved(*at)]);
    }
    if let Some(last) = through.last() {
        fixture.frame(vec![release(*last)]);
    }
    fixture.idle(2);
}

fn angle(fixture: &Fixture) -> f64 {
    *fixture.data().value("angle", f64::NAN)
}

/// The picker's point, with a mixed axis as NaN.
fn point(fixture: &Fixture) -> [f64; 2] {
    let value = *fixture.data().value("point", Mixed::each([f64::NAN; 2]));
    value.map(|part| part.same().copied().unwrap_or(f64::NAN))
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
    up.click_at(center + Vec2::new(0.0, -REACH));
    assert_eq!(angle(&up), 90.0, "counter-clockwise, 0 at 3 o'clock: up is 90");
    up.click_at(center + Vec2::new(0.0, REACH));
    assert_eq!(angle(&up), -90.0);

    let mut right = shown(clockwise);
    let center = control_rect(&right).center();
    right.click_at(center + Vec2::new(REACH, 0.0));
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
    fixture.click_at(well.lerp_inside(Vec2::new(0.25, 0.75)));
    let [x, y] = point(&fixture);
    assert!((x - 25.0).abs() < 0.1 && (y - 75.0).abs() < 0.1, "a click a quarter across and three quarters down is (25, 75), not ({x}, {y})");
}

/// The picker's value is two slider sub-nodes, one per axis, each with its own numeric; the picker stays one Tab
/// stop, and Increment or Decrement on a sub-node steps that axis alone.
#[trial]
fn a_point_picker_has_a_slider_sub_node_per_axis_that_steps_its_own_axis() {
    let mut fixture = shown(picker);
    let sliders = fixture.by_role(Role::SLIDER);
    assert_eq!(sliders.len(), 2, "one slider sub-node per axis");
    for axis in &sliders {
        assert!(fixture.tree().declares_key(axis.node, Declared::Increment) && fixture.tree().declares_key(axis.node, Declared::SetValue));
    }
    let named = |fixture: &Fixture, end: char| {
        fixture.by_role(Role::SLIDER).into_iter().find(|target| fixture.tree().name_of(target.node).is_some_and(|name| name.ends_with(end)))
    };
    let y = named(&fixture, 'Y').expect("the y axis has a node");
    fixture.access(AccessActionKind::Increment, &y);
    fixture.idle(2);
    let stepped_y = [50.0, 51.0];
    assert_eq!(point(&fixture), stepped_y, "Increment on the y sub-node steps y and leaves x");
    let x = named(&fixture, 'X').expect("the x axis has a node");
    fixture.access(AccessActionKind::Decrement, &x);
    fixture.idle(2);
    let stepped_x = [49.0, 51.0];
    assert_eq!(point(&fixture), stepped_x);
    fixture.access(AccessActionKind::SetValue(20.0), &y);
    fixture.idle(2);
    let set_y = [49.0, 20.0];
    assert_eq!(point(&fixture), set_y, "SetValue on the y sub-node sets y");
}

#[trial]
fn a_point_picker_with_a_y_up_axis_takes_keys_and_drags_and_up_raises_the_value() {
    let mut fixture = focused(picker_y_up);
    key(&mut fixture, Key::ArrowUp, Modifiers::NONE);
    assert_eq!(point(&fixture), [50.0, 51.0], "Up moves the point up, which raises y on a y-up axis");
    key(&mut fixture, Key::ArrowDown, Modifiers::NONE);
    assert_eq!(point(&fixture), [50.0, 50.0]);

    let well = control_rect(&fixture);
    fixture.click_at(well.lerp_inside(Vec2::new(0.5, 0.25)));
    let [_, y] = point(&fixture);
    assert!((y - 75.0).abs() < 0.1, "a click a quarter down a y-up axis is 75, not {y}");
    let (from, to) = (well.lerp_inside(Vec2::new(0.5, 0.5)), well.lerp_inside(Vec2::new(0.5, 0.75)));
    drag_through(&mut fixture, from, &[from + Vec2::new(0.0, 4.0), to]);
    let [_, y] = point(&fixture);
    assert!((y - 25.0).abs() < 0.1, "a drag to three quarters down is 25, not {y}");
}

/// Alt makes a drag fine; a click is not a drag, so Alt+click still puts the point where the pointer is.
#[trial]
fn an_alt_click_on_a_point_picker_sets_the_clicked_point() {
    let mut fixture = shown(picker);
    let well = control_rect(&fixture);
    fixture.hold(Modifiers::ALT);
    fixture.click_at(well.lerp_inside(Vec2::new(0.25, 0.75)));
    let [x, y] = point(&fixture);
    assert!((x - 25.0).abs() < 0.1 && (y - 75.0).abs() < 0.1, "Alt+click is (25, 75), not ({x}, {y})");
}

#[trial]
fn keys_do_nothing_to_a_dial_while_a_drag_runs() {
    let mut fixture = focused(counter_clockwise);
    let center = control_rect(&fixture).center();
    let right = center + Vec2::new(REACH, 0.0);
    fixture.frame(vec![moved(center + Vec2::new(0.0, -REACH))]);
    fixture.frame(vec![press(center + Vec2::new(0.0, -REACH))]);
    fixture.frame(vec![moved(center + Vec2::new(10.0, -10.0))]);
    fixture.frame(vec![moved(right)]);
    fixture.key(Key::ArrowUp, Modifiers::NONE);
    fixture.frame(vec![release(right)]);
    fixture.idle(2);
    assert_eq!(angle(&fixture), 0.0, "the pointer's angle stands; the key did not step it");
}

/// The picker keeps its axis order: Right raises x and Left lowers it in either reading direction.
#[trial]
fn a_point_picker_moves_right_on_right_in_either_reading_direction() {
    for direction in [Direction::Ltr, Direction::Rtl] {
        let mut fixture = shown(picker);
        fixture.direction(direction).settle();
        key(&mut fixture, Key::Tab, Modifiers::NONE);
        key(&mut fixture, Key::ArrowRight, Modifiers::NONE);
        assert_eq!(point(&fixture), [51.0, 50.0], "Right raises x under {direction:?}");
        key(&mut fixture, Key::ArrowLeft, Modifiers::NONE);
        assert_eq!(point(&fixture), [50.0, 50.0], "Left lowers x under {direction:?}");
    }
}
