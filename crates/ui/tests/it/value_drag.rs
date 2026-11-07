//! `Slider` and `RangeSlider`: the value maths that is the family's own: stepping in the reading direction, the scales
//! that spread a range over a track, and thumbs that never cross.

use craft_i18n::Direction;
use craft_ui::contract::{Bind, Props as _, Show as _};
use craft_ui::value_drag::{RangeSlider, Slider, SliderScale};
use craft_ui::testing::{Fixture, Story, StoryData, StoryOutput, trial};
use egui::{Key, Modifiers, Ui, Vec2};

const START: f64 = 50.0;
const STEP: f64 = 5.0;

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

    let value = *fixture.data().value("value", 1.0);
    assert!((value - 10.0).abs() < 1e-6, "halfway along 1 to 100 is 10, got {value}");
}

fn show_range(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    RangeSlider::new(Bind::bound(data.value("range", (20.0, 60.0))), 0.0..=100.0).label("Level").show(ui).into()
}

#[trial]
fn the_lower_thumb_stops_at_the_upper_thumb() {
    let mut fixture = Fixture::story(&Story::new("range-slider", show_range));
    fixture.settle();
    let lower = fixture.by_label("Level, Minimum").expect("the lower thumb is named by the label and its end");
    fixture.drag(&lower, Vec2::new(500.0, 0.0));

    assert_eq!(*fixture.data().value("range", (20.0, 60.0)), (60.0, 60.0));
}

fn show_marked(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Slider::new(Bind::bound(data.value("value", 0.0)), 0.0..=100.0).marks(&[48.0]).label("Level").show(ui).into()
}

#[trial]
fn a_press_near_a_mark_sets_the_mark() {
    let mut fixture = Fixture::story(&Story::new("slider-marks", show_marked));
    fixture.settle();
    let slider = fixture.by_label("Level").expect("the slider is named by its label");
    fixture.click(&slider);

    assert_eq!(*fixture.data().value("value", 0.0), 48.0, "the middle of the track is 50, two units from the mark");
}
