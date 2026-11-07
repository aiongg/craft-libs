//! `Slider` and `RangeSlider`: the value maths that is the family's own: stepping in the reading direction, the scales
//! that spread a range over a track, and thumbs that never cross.

mod dials;
mod thumb_row;

use craft_i18n::Direction;
use craft_ui::contract::{Bind, Props as _, Show as _};
use craft_ui::testing::pointer;
use craft_ui::testing::{Fixture, Story, StoryData, StoryOutput, trial};
use craft_ui::value_drag::{RangeSlider, Slider, SliderScale};
use egui::{Key, Modifiers, Ui, Vec2};

const START: f64 = 50.0;
const STEP: f64 = 5.0;
const GEOMETRIC_MEAN: f64 = 10.0;
const TOLERANCE: f64 = 1e-6;
const UPPER_END: f64 = 60.0;
const MARK: f64 = 48.0;
const RANGE_START: (f64, f64) = (20.0, UPPER_END);

fn show(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Slider::new(Bind::bound(data.value("value", START)), 0.0..=100.0).step(STEP).label("Opacity").show(ui).into()
}

fn after(direction: Direction, key: Key, modifiers: Modifiers) -> f64 {
    let mut fixture = Fixture::story(&Story::new("slider", show));
    fixture.direction(direction).settle();
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.key(key, modifiers);
    fixture.idle(2);

    *fixture.data().value("value", START)
}

#[trial]
fn a_slider_steps_toward_the_end_edge_of_the_reading_direction() {
    assert_eq!(after(Direction::Ltr, Key::ArrowRight, Modifiers::NONE), START + STEP);
    assert_eq!(after(Direction::Ltr, Key::ArrowLeft, Modifiers::NONE), START - STEP);
    assert_eq!(after(Direction::Rtl, Key::ArrowLeft, Modifiers::NONE), START + STEP, "the mirrored slider grows toward the left");
    assert_eq!(after(Direction::Ltr, Key::ArrowRight, Modifiers::SHIFT), 100.0, "Shift steps ten times as far, to the end of the range");
}

fn show_log(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Slider::new(Bind::bound(data.value("value", 1.0)), 1.0..=100.0).scale(SliderScale::Log).label("Size").show(ui).into()
}

#[trial]
fn a_press_halfway_along_a_log_track_sets_the_geometric_mean() {
    let mut fixture = Fixture::story(&Story::new("slider-log", show_log));
    fixture.settle();
    let slider = fixture.by_label("Size").expect("the slider is named by its label");
    fixture.click(&slider);

    let value = *fixture.data().value("value", 1.0_f64);
    assert!((value - GEOMETRIC_MEAN).abs() < TOLERANCE, "halfway along 1 to 100 is 10, got {value}");
}

fn show_range(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    RangeSlider::new(Bind::bound(data.value("range", RANGE_START)), 0.0..=100.0).label("Level").show(ui).into()
}

#[trial]
fn the_lower_thumb_stops_at_the_upper_thumb() {
    let mut fixture = Fixture::story(&Story::new("range-slider", show_range));
    fixture.settle();
    let lower = fixture.by_label("Level, Minimum").expect("the lower thumb is named by the label and its end");
    fixture.drag(&lower, Vec2::new(500.0, 0.0));

    assert_eq!(*fixture.data().value("range", RANGE_START), (UPPER_END, UPPER_END));
}

/// A press on the track beside the upper thumb moves that thumb and focuses it, so the next arrow steps it.
#[trial]
fn a_track_press_near_the_upper_thumb_moves_and_focuses_it() {
    let mut fixture = Fixture::story(&Story::new("range-slider", show_range));
    fixture.settle();
    let upper = fixture.by_label("Level, Maximum").expect("the upper thumb is named by the label and its end");
    let at = upper.center() + Vec2::new(24.0, 0.0);
    fixture.frame(vec![pointer::moved(at)]);
    fixture.frame(vec![pointer::press(at)]);
    fixture.frame(vec![pointer::release(at)]);
    fixture.idle(2);
    let pressed = *fixture.data().value("range", RANGE_START);
    assert_eq!(pressed.0, RANGE_START.0, "the lower thumb stays");
    assert!(pressed.1 > UPPER_END, "the upper thumb moved to the press: {pressed:?}");
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    fixture.idle(2);
    let stepped = *fixture.data().value("range", RANGE_START);
    assert_eq!(stepped.0, pressed.0, "the arrow does not step the lower thumb");
    assert!(stepped.1 > pressed.1, "the arrow steps the focused upper thumb: {pressed:?} then {stepped:?}");
}

fn show_marked(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Slider::new(Bind::bound(data.value("value", 0.0)), 0.0..=100.0).marks(&[MARK]).label("Level").show(ui).into()
}

#[trial]
fn a_press_near_a_mark_sets_the_mark() {
    let mut fixture = Fixture::story(&Story::new("slider-marks", show_marked));
    fixture.settle();
    let slider = fixture.by_label("Level").expect("the slider is named by its label");
    fixture.click(&slider);

    assert_eq!(*fixture.data().value("value", 0.0), MARK, "the middle of the track is 50, two units from the mark");
}
