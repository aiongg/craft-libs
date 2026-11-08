//! The colour area, strip, ramp, disc and wheel: the colour maths and the edits they report.

use std::cell::RefCell;
use std::rc::Rc;

use craft_color::AreaMode;
use craft_color::ui::{
    ColorArea, ColorDisc, ColorRamp, ColorStrip, ColorWheel, DiscMove, RampKind, RampPick, WheelGeometry, hue_at, triangle_corners, triangle_point,
    triangle_sv,
};
use craft_theme::{ColorModel, HsbModel, ModelValues, Srgba};
use craft_ui::contract::{Bind, Show as _};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, Pos2, Rect, pos2, vec2};

const AREA: &str = "Saturation and brightness";
/// The room the parts are shown in.
const ROOM: egui::Vec2 = egui::Vec2::new(400.0, 300.0);
/// A web-safe channel has six levels, five steps apart.
const WEB_STEPS: f32 = 5.0;
/// The hue range of the hue strip, in degrees.
const HUE_RANGE: f32 = 360.0;
/// The hue strip's arrow key step and its Shift step, in degrees.
const HUE_STEP: f32 = 1.0;
const HUE_BIG_STEP: f32 = 10.0;
/// What a click a quarter down the hue strip may differ from its hue by, in degrees.
const HUE_TOLERANCE: f32 = 2.0;
/// A grey ramp's left end counts as white from this red level.
const NEAR_WHITE: f32 = 0.9;
/// The saturation of the first disc marker's colour, rounded up.
const MARKER_SATURATION: f32 = 0.78;
/// Straight up on the wheel, in degrees counter-clockwise from the right.
const UP: f32 = 90.0;
/// The wheel's last whole hue, in degrees.
const LAST_HUE: f32 = 359.0;

/// The HSB values of `color`.
fn hsb(color: Srgba) -> ModelValues {
    HsbModel.from_srgb(color)
}

/// A fixture showing `show` over a shared value.
fn fixture<T: 'static>(value: T, show: impl Fn(&mut egui::Ui, &mut T) + 'static) -> (Fixture, Rc<RefCell<T>>) {
    let shared = Rc::new(RefCell::new(value));
    let held = shared.clone();
    let mut fixture = Fixture::new(move |ui| show(ui, &mut held.borrow_mut()));
    fixture.room(ROOM);
    fixture.settle();
    (fixture, shared)
}

fn within(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

/// A drag across the area edits saturation and brightness and leaves the hue, also a grey's.
#[trial]
fn an_area_drag_changes_its_two_components_and_keeps_the_hue() {
    let start = ModelValues::new("hsb", &[200.0, 50.0, 50.0]);
    let hue = start.values[0];
    let (mut fixture, values) = fixture(start, |ui, values| {
        ColorArea::new(AREA, AreaMode::Hue, Bind::bound(values)).show(ui);
    });
    let area = fixture.by_label(AREA).expect("the area is shown");
    fixture.drag(&area, vec2(-area.rect.width(), 0.0));
    let after = *values.borrow();
    assert!(within(after.values[1], 0.0), "dragged to the left edge, saturation is 0, got {}", after.values[1]);
    assert!(within(after.values[0], hue), "the hue is not the area's to change");
}

/// A web-safe area reports colours on the six levels.
#[trial]
fn a_quantized_area_reports_web_colours() {
    let (mut fixture, values) = fixture(hsb(Srgba::rgb(0.2, 0.6, 0.8)), |ui, values| {
        ColorArea::new(AREA, AreaMode::Hue, Bind::bound(values)).quantize(6).show(ui);
    });
    let area = fixture.by_label(AREA).expect("the area is shown");
    fixture.click_at(area.rect.min + vec2(area.rect.width() * 0.37, area.rect.height() * 0.41));
    let color = HsbModel.to_srgb(&values.borrow().values);
    for channel in [color.r, color.g, color.b] {
        assert!(within((channel * WEB_STEPS).round(), channel * WEB_STEPS), "{channel} is a web level");
    }
}

/// A click on the strip sets its component to the pointer, and the arrow keys step it by the channel's step.
#[trial]
fn a_strip_click_sets_the_value_and_keys_step_it() {
    let (mut fixture, values) = fixture(ModelValues::new("hsb", &[0.0, 80.0, 80.0]), |ui, values| {
        ColorStrip::new("Hue", AreaMode::Hue, Bind::bound(values)).show(ui);
    });
    let strip = fixture.by_label("Hue").expect("the strip is shown");
    fixture.click_at(pos2(strip.rect.center().x, strip.rect.top() + strip.rect.height() * 0.25));
    let clicked = values.borrow().values[0];
    let expected = HUE_RANGE * 0.75;
    assert!((clicked - expected).abs() < HUE_TOLERANCE, "a quarter down the strip is three quarters of the hue range, got {clicked}");
    fixture.key(Key::ArrowUp, Modifiers::NONE);
    let stepped = clicked + HUE_STEP;
    assert!(within(values.borrow().values[0], stepped), "Up steps one degree");
    fixture.key(Key::ArrowDown, Modifiers::SHIFT);
    let shifted = stepped - HUE_BIG_STEP;
    assert!(within(values.borrow().values[0], shifted), "Shift Down steps ten");
}

/// A drag on the strip is one gesture that moves only its component.
#[trial]
fn a_strip_drag_keeps_the_other_components() {
    let start = ModelValues::new("hsb", &[0.0, 80.0, 60.0]);
    let (mut fixture, values) = fixture(start, |ui, values| {
        ColorStrip::new("Hue", AreaMode::Hue, Bind::bound(values)).show(ui);
    });
    let strip = fixture.by_label("Hue").expect("the strip is shown");
    fixture.drag(&strip, vec2(0.0, -strip.rect.height() / 4.0));
    let after = *values.borrow();
    assert!(after.values[0] > 0.0);
    assert!(within(after.values[1], start.values[1]) && within(after.values[2], start.values[2]));
}

/// The ramp's colours: corners of the spectrum, the grey ramp, the ends of a between ramp, web rounding and the gamut.
#[trial]
fn the_ramp_colour_function_follows_the_kind() {
    let ends = [Srgba::rgb(0.0, 0.0, 0.0), Srgba::rgb(1.0, 1.0, 1.0)];
    let at = |kind, x: f32, y: f32| ColorRamp::color_at(kind, false, None, ends, x, y);
    assert_eq!(at(RampKind::Spectrum, 0.0, 0.0), Srgba::rgb(1.0, 1.0, 1.0), "white at the top");
    assert_eq!(at(RampKind::Spectrum, 0.0, 1.0), Srgba::rgb(0.0, 0.0, 0.0), "black at the bottom");
    assert_eq!(at(RampKind::Spectrum, 0.0, 0.5), Srgba::rgb(1.0, 0.0, 0.0), "the full colour in the middle");
    assert_eq!(at(RampKind::Grayscale, 0.0, 0.3), Srgba::rgb(1.0, 1.0, 1.0));
    assert_eq!(at(RampKind::Grayscale, 1.0, 0.3), Srgba::rgb(0.0, 0.0, 0.0));
    assert_eq!(at(RampKind::Between, 1.0, 0.0), ends[1]);
    assert_eq!(at(RampKind::Between, f32::NAN, f32::NAN), ends[0], "a number that is none counts as the start");
    let web = ColorRamp::color_at(RampKind::Grayscale, true, None, ends, 0.37, 0.0);
    assert!(within(web.r * 5.0, (web.r * 5.0).round()), "web safe rounds each channel to a fifth");
    let invert = |c: Srgba| Srgba::new(1.0 - c.r, 1.0 - c.g, 1.0 - c.b, c.a);
    let mapped = ColorRamp::color_at(RampKind::CmykSpectrum, false, Some(&invert), ends, 0.0, 0.0);
    assert_eq!(mapped, Srgba::rgb(0.0, 0.0, 0.0), "the spectrum goes through the gamut");
}

/// A click on the ramp picks the colour there; Shift cycles and picks nothing; a box picks its colour.
#[trial]
fn a_ramp_click_picks_and_a_shift_click_cycles() {
    let seen = Rc::new(RefCell::new((RampPick::new(Srgba::rgb(0.5, 0.5, 0.5), false), 0usize, false)));
    let shared = seen.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut state = shared.borrow_mut();
        let mut pick = state.0;
        let out = ColorRamp::new("Color ramp", RampKind::Grayscale, Bind::bound(&mut pick)).show(ui);
        state.0 = pick;
        state.1 += usize::from(out.cycle);
        state.2 |= out.menu_at.is_some();
    });
    fixture.room(vec2(400.0, 120.0));
    fixture.settle();
    let ramp = fixture.by_label("Color ramp").expect("the ramp is shown");
    fixture.click_at(pos2(ramp.rect.left() + 2.0, ramp.rect.center().y));
    let picked = seen.borrow().0.color;
    assert!(picked.r > NEAR_WHITE, "the left end of a grey ramp is white, got {picked:?}");
    fixture.hold(Modifiers::SHIFT);
    fixture.click_at(pos2(ramp.rect.left() + ramp.rect.width() * 0.4, ramp.rect.center().y));
    fixture.hold(Modifiers::NONE);
    assert_eq!(seen.borrow().1, 1, "a Shift click cycles");
    assert_eq!(seen.borrow().0.color, picked, "and picks nothing");
}

/// The disc grabs the marker under the press and a drag moves it.
#[trial]
fn a_disc_drag_moves_the_grabbed_marker() {
    let markers = [Srgba::rgb(0.9, 0.2, 0.2), Srgba::rgb(0.2, 0.7, 0.4)];
    let (mut fixture, moved) = fixture(DiscMove::new(9, 0.0, 0.0), move |ui, moved| {
        ColorDisc::new("Harmony", &markers, Bind::bound(moved)).chosen(0).show(ui);
    });
    let marker = fixture.by_label("Color 1").expect("the selected marker is a stop with a node");
    let disc = fixture.by_label("Harmony").expect("the disc is shown");
    fixture.drag(&marker, (disc.rect.center() - marker.center()) * 0.5);
    let after = *moved.borrow();
    assert_eq!(after.marker, 0, "the drag started on the first marker");
    assert!(after.saturation < MARKER_SATURATION, "the marker moved towards the centre, got {}", after.saturation);
}

/// A press on the disc away from every marker grabs nothing.
#[trial]
fn a_disc_press_away_from_the_markers_grabs_nothing() {
    let markers = [Srgba::rgb(0.9, 0.2, 0.2)];
    let untouched = DiscMove::new(9, 1.0, 0.5);
    let (mut fixture, moved) = fixture(untouched, move |ui, moved| {
        ColorDisc::new("Harmony", &markers, Bind::bound(moved)).show(ui);
    });
    let disc = fixture.by_label("Harmony").expect("the disc is shown");
    fixture.click_at(disc.rect.center() - vec2(0.0, disc.rect.height() / 4.0));
    assert_eq!(*moved.borrow(), untouched);
}

/// The wheel's pure geometry: a triangle point reads back as its saturation and brightness, and the ring hue is whole
/// degrees counter-clockwise from the right.
#[trial]
fn the_wheel_geometry_round_trips() {
    let center = pos2(100.0, 100.0);
    let corners = triangle_corners(center, 60.0, 210.0);
    for (s, b) in [(0.0, 1.0), (1.0, 1.0), (0.4, 0.7), (0.9, 0.2)] {
        let (read_s, read_b) = triangle_sv(corners, triangle_point(corners, s, b)).expect("a triangle point reads back");
        assert!(within(read_s, s) && within(read_b, b), "({s}, {b}) read as ({read_s}, {read_b})");
    }
    assert_eq!(hue_at(center, center + vec2(10.0, 0.0)), Some(0.0));
    assert_eq!(hue_at(center, center + vec2(0.0, -10.0)), Some(90.0), "up is 90 degrees");
    assert_eq!(hue_at(center, Pos2::new(f32::NAN, 0.0)), None);
    let geometry = WheelGeometry::new(Rect::from_min_size(Pos2::ZERO, vec2(200.0, 200.0)), 4.0, (8.0, 24.0), 0.09, 1.0);
    assert!(geometry.inner < geometry.outer && geometry.triangle < geometry.inner);
}

/// A click on the wheel's ring sets the hue to the pointer's angle and leaves saturation and brightness.
#[trial]
fn a_wheel_ring_click_changes_only_the_hue() {
    let start = ModelValues::new("hsb", &[0.0, 60.0, 70.0]);
    let (mut fixture, values) = fixture(start, |ui, values| {
        ColorWheel::new("Color wheel", Bind::bound(values)).show(ui);
    });
    let wheel = fixture.by_label("Hue").expect("the ring has a node");
    let half = wheel.rect.width() / 2.0;
    fixture.click_at(wheel.center() + vec2(0.0, -half * 0.92));
    let after = *values.borrow();
    assert!(within(after.values[0], UP), "above the centre is 90 degrees, got {}", after.values[0]);
    assert!(within(after.values[1], start.values[1]) && within(after.values[2], start.values[2]));
}

/// A press beside the wheel, outside the ring's reach, starts nothing.
#[trial]
fn a_wheel_press_outside_the_ring_starts_nothing() {
    let start = ModelValues::new("hsb", &[10.0, 60.0, 70.0]);
    let (mut fixture, values) = fixture(start, |ui, values| {
        ColorWheel::new("Color wheel", Bind::bound(values)).show(ui);
    });
    let wheel = fixture.by_label("Hue").expect("the ring has a node");
    fixture.click_at(wheel.rect.right_bottom() + vec2(1.0, 1.0));
    assert!(within(values.borrow().values[0], start.values[0]));
}

/// The ring's keys wrap the hue at 360.
#[trial]
fn the_ring_keys_wrap_the_hue() {
    let (mut fixture, values) = fixture(ModelValues::new("hsb", &[LAST_HUE, 60.0, 70.0]), |ui, values| {
        ColorWheel::new("Color wheel", Bind::bound(values)).show(ui);
    });
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    assert!(within(values.borrow().values[0], 0.0), "the last hue plus a step wraps to 0, got {}", values.borrow().values[0]);
    fixture.key(Key::Home, Modifiers::NONE);
    fixture.key(Key::End, Modifiers::NONE);
    assert!(within(values.borrow().values[0], LAST_HUE), "End is the last hue");
}
