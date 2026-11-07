//! `Slider` and `RangeSlider`: the value maths that is the family's own: stepping in the reading direction, the scales
//! that spread a range over a track, and thumbs that never cross.

mod dials;
mod thumb_row;

use craft_behavior::{EditEvent, GesturePhase, Mixed};
use craft_i18n::Direction;
use craft_ui::contract::{Bind, Props as _, Show as _};
use craft_ui::testing::input::key_press;
use craft_ui::testing::pointer;
use craft_ui::testing::{Fixture, Frames, Story, StoryData, StoryOutput, trial};
use craft_ui::value_drag::{RangeSlider, Slider, SliderScale};
use egui::{Key, Modifiers, Pos2, Rect, Ui, Vec2};
use std::cell::RefCell;

const START: f64 = 50.0;
const STEP: f64 = 5.0;
const GEOMETRIC_MEAN: f64 = 10.0;
const TOLERANCE: f64 = 1e-6;
const UPPER_END: f64 = 60.0;
const MARK: f64 = 48.0;
const RANGE_START: (Mixed<f64>, Mixed<f64>) = (Mixed::Same(20.0), Mixed::Same(UPPER_END));

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

    assert_eq!(*fixture.data().value("range", RANGE_START), (Mixed::Same(UPPER_END), Mixed::Same(UPPER_END)));
}

fn lower_end(fixture: &Fixture) -> f64 {
    *fixture.data().value("range", RANGE_START).0.same().expect("the lower end has a number")
}

fn upper_end(fixture: &Fixture) -> f64 {
    *fixture.data().value("range", RANGE_START).1.same().expect("the upper end has a number")
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
    let pressed = upper_end(&fixture);
    const LOWER: f64 = 20.0;
    assert_eq!(lower_end(&fixture), LOWER, "the lower thumb stays");
    assert!(pressed > UPPER_END, "the upper thumb moved to the press: {pressed:?}");
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    fixture.idle(2);
    assert_eq!(lower_end(&fixture), LOWER, "the arrow does not step the lower thumb");
    let stepped = upper_end(&fixture);
    assert!(stepped > pressed, "the arrow steps the focused upper thumb: {pressed:?} then {stepped:?}");
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

const FIELD: Rect = Rect { min: Pos2::new(20.0, 20.0), max: Pos2::new(220.0, 50.0) };
/// The distance the thumb's centre travels along the field: the width less the thumb's square.
const TRAVEL: f32 = 170.0;
const NOTCH: f64 = 0.01;

/// A slider shown in `FIELD` that holds its value and the edit events of every pass.
struct Slide {
    value: RefCell<f64>,
    range: std::ops::RangeInclusive<f64>,
    step: Option<f64>,
    wheel_step: Option<f64>,
    events: RefCell<Vec<EditEvent<f64>>>,
}

impl Slide {
    fn new(value: f64) -> Self {
        Slide { value: RefCell::new(value), range: 0.0..=100.0, step: None, wheel_step: None, events: RefCell::default() }
    }

    fn show(&self, ui: &mut Ui) {
        let mut value = self.value.borrow_mut();
        let mut slider = Slider::new(Bind::bound(&mut *value), self.range.clone()).label("Opacity");
        if let Some(step) = self.step {
            slider = slider.step(step);
        }
        if let Some(step) = self.wheel_step {
            slider = slider.wheel_step(step);
        }
        if let Some(event) = slider.show_at(ui, FIELD).event {
            self.events.borrow_mut().push(event);
        }
    }

    fn value(&self) -> f64 {
        *self.value.borrow()
    }

    fn commits(&self) -> usize {
        self.events.borrow().iter().filter(|event| event.phase() == Some(GesturePhase::Commit)).count()
    }

    fn sets(&self) -> usize {
        self.events.borrow().iter().filter(|event| matches!(event, EditEvent::Set { .. })).count()
    }
}

/// Focuses the slider with Tab and lets the passes settle.
fn focused(frames: &mut Frames, show: &mut impl FnMut(&mut Ui)) {
    frames.idle(3, &mut *show);
    frames.frame(vec![key_press(Key::Tab, Modifiers::NONE)], &mut *show);
    frames.idle(2, &mut *show);
}

/// Presses at `from`, moves `travel` along the track in four frames with `between` after the second, releases.
fn drag(frames: &mut Frames, from: Pos2, travel: f32, show: &mut impl FnMut(&mut Ui), mut between: impl FnMut(&mut Frames, &mut dyn FnMut(&mut Ui))) {
    frames.frame(vec![pointer::moved(from)], &mut *show);
    frames.frame(vec![pointer::press(from)], &mut *show);
    for step in 1..=4 {
        frames.frame(vec![pointer::moved(from + Vec2::new(travel * step as f32 / 4.0, 0.0))], &mut *show);
        if step == 2 {
            between(frames, &mut *show);
        }
    }
    frames.frame(vec![pointer::release(from + Vec2::new(travel, 0.0))], &mut *show);
    frames.idle(2, &mut *show);
}

/// The value after grabbing the slider's thumb `offset` from its centre and dragging `travel`.
fn dragged_from(offset: f32, travel: f32) -> f64 {
    let slide = Slide::new(START);
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| slide.show(ui);
    frames.idle(3, &mut show);
    drag(&mut frames, FIELD.center() + Vec2::new(offset, 0.0), travel, &mut show, |_, _| {});
    slide.value()
}

#[trial]
fn a_grabbed_slider_thumb_moves_by_the_pointer_travel_and_a_click_on_it_changes_nothing() {
    let expected = START + f64::from(10.0 / TRAVEL) * 100.0;
    let centred = dragged_from(0.0, 10.0);
    assert!((centred - expected).abs() < 1e-3, "10 px of travel is 10 px of value: {centred}");
    let off_centre = dragged_from(3.0, 10.0);
    assert!((off_centre - expected).abs() < 1e-3, "a press 3 px off the centre does not add the offset: {off_centre}");

    let slide = Slide::new(START);
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| slide.show(ui);
    frames.idle(3, &mut show);
    pointer::click(&mut frames, FIELD.center() + Vec2::new(3.0, 0.0), &mut show);
    assert_eq!(slide.value(), START, "a click on the thumb leaves the value");
}

#[trial]
fn an_arrow_key_during_a_thumb_drag_changes_nothing_and_the_drag_commits_as_one_edit() {
    let run = |arrow: bool| {
        let slide = Slide::new(START);
        let mut frames = Frames::new();
        let mut show = |ui: &mut Ui| slide.show(ui);
        focused(&mut frames, &mut show);
        let from = FIELD.center();
        drag(&mut frames, from, 20.0, &mut show, |frames, show| {
            if arrow {
                frames.frame(vec![key_press(Key::ArrowRight, Modifiers::NONE)], show);
            }
        });
        (slide.value(), slide.commits(), slide.sets())
    };
    let (plain, plain_commits, _) = run(false);
    let (with_arrow, commits, sets) = run(true);
    assert_eq!(with_arrow, plain, "the arrow did not move the value");
    assert_eq!((commits, plain_commits), (1, 1), "the drag is one gesture");
    assert_eq!(sets, 0, "the arrow did not set a value of its own");
}

#[trial]
fn an_alt_click_on_the_track_sets_the_clicked_value_and_two_arrows_in_one_pass_step_twice() {
    let slide = Slide::new(START);
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| slide.show(ui);
    frames.idle(3, &mut show);
    // The thumb's centre is `TRAVEL / 2` from the track's start; a click a fifth of the way along sets 20.
    let at = Pos2::new(FIELD.left() + (FIELD.height() / 2.0) + 0.2 * TRAVEL, FIELD.center().y);
    frames.hold(Modifiers::ALT);
    pointer::click(&mut frames, at, &mut show);
    frames.hold(Modifiers::NONE);
    const CLICKED: f64 = 20.0;
    assert!((slide.value() - CLICKED).abs() < 1e-3, "Alt does not scale a click: {}", slide.value());

    let stepping = Slide { step: Some(5.0), ..Slide::new(START) };
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| stepping.show(ui);
    focused(&mut frames, &mut show);
    frames.frame(vec![key_press(Key::ArrowRight, Modifiers::NONE), key_press(Key::ArrowRight, Modifiers::NONE)], &mut show);
    frames.idle(2, &mut show);
    const ARROWS: f64 = 2.0;
    assert_eq!(stepping.value(), START + ARROWS * STEP, "each arrow of the pass steps from the value the last one gave");
}

#[trial]
fn a_zero_scroll_wheel_changes_nothing_and_a_wheel_notch_is_the_wheel_step() {
    let wheel = |dy: f32, wheel_step: Option<f64>| {
        let slide = Slide { range: 0.0..=1.0, step: Some(0.0001), wheel_step, ..Slide::new(0.5) };
        let mut frames = Frames::new();
        let mut show = |ui: &mut Ui| slide.show(ui);
        focused(&mut frames, &mut show);
        frames.frame(vec![pointer::moved(FIELD.center())], &mut show);
        frames.frame(vec![pointer::wheel(dy)], &mut show);
        frames.idle(2, &mut show);
        slide.value()
    };
    const MIDDLE: f64 = 0.5;
    assert_eq!(wheel(0.0, None), MIDDLE, "a wheel event with no scroll moves nothing");
    let notch = (wheel(30.0, Some(NOTCH)) - MIDDLE).abs();
    assert!((notch - NOTCH).abs() < 1e-9, "one notch moves the wheel step, not the 0.0001 step: {notch}");
}
