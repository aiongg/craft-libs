//! `Slider` and `RangeSlider`: the value maths that is the family's own: stepping in the reading direction, the scales
//! that spread a range over a track, and thumbs that never cross.

mod dials;

mod thumb_row;

use craft_behavior::{EditEvent, GesturePhase};
use craft_ui::contract::{Bind, Props as _, Show as _, ShowContainer as _};
use craft_ui::testing::input::key_press;
use craft_ui::testing::pointer;
use craft_ui::testing::{Frames, StoryData, StoryOutput, trial};
use craft_ui::value_drag::{RangeSlider, Slider};
use egui::{Key, Modifiers, Pos2, Rect, Ui, Vec2};
use std::cell::RefCell;

const START: f64 = 50.0;

const STEP: f64 = 5.0;

fn show(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Slider::new(Bind::bound(data.value("value", START)), 0.0..=100.0).step(STEP).label("Opacity").show(ui).into()
}

const FIELD: Rect = Rect { min: Pos2::new(20.0, 20.0), max: Pos2::new(220.0, 50.0) };

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
        self.events.borrow_mut().extend(slider.show_at(ui, FIELD).events);
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
    let centred = dragged_from(0.0, 10.0);
    assert!(centred > START, "the drag moved the value: {centred}");
    let twice = dragged_from(0.0, 20.0);
    assert!(((twice - centred) - (centred - START)).abs() < 1e-3, "twice the travel is twice the change: {centred} then {twice}");
    let off_centre = dragged_from(3.0, 10.0);
    assert!((off_centre - centred).abs() < 1e-3, "a press 3 px off the centre does not add the offset: {off_centre}");

    let slide = Slide::new(START);
    let mut frames = Frames::new();
    let mut show = |ui: &mut Ui| slide.show(ui);
    frames.idle(3, &mut show);
    pointer::click(&mut frames, FIELD.center() + Vec2::new(3.0, 0.0), &mut show);
    assert_eq!(slide.value(), START, "a click on the thumb leaves the value");
}
