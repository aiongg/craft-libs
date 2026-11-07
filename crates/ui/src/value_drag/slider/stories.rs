//! Stories of `Slider`.

use super::{Slider, SliderScale, SliderThumb, THUMB};
use crate::contract::{Bind, Props as _, Show as _, Validity};
use crate::display::Progress;
use crate::runtime::env::Message;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use crate::value_drag::TrackFace;
use craft_behavior::Mixed;
use egui::Ui;

const RANGE: std::ops::RangeInclusive<f64> = 0.0..=100.0;
const CAP: f64 = 80.0;

#[story(Slider<'static>, covers = [Default], drive = Drive::Step)]
fn slider(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Slider::new(Bind::bound(data.value("value", 40.0)), RANGE).label("Opacity").show(ui).into()
}

#[story(Slider<'static>, covers = [], drive = Drive::Drag(THUMB))]
fn slider_drag(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Slider::new(Bind::bound(data.value("value", 40.0)), RANGE).label("Opacity").show(ui).into()
}

#[story(Slider<'static>, covers = [], drive = Drive::Step)]
fn slider_log(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Slider::new(Bind::bound(data.value("size", 8.0)), 1.0..=200.0).scale(SliderScale::Log).label("Brush size").show(ui).into()
}

#[story(Slider<'static>, covers = [Mixed], drive = Drive::Never)]
fn slider_mixed(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Slider::new(Bind::bound_mixed(data.value("mixed", Mixed::Mixed)), RANGE).label("Opacity").show(ui).into()
}

#[story(Slider<'static>, covers = [Invalid], drive = Drive::Never)]
fn slider_invalid(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let refuse_over = |value: &f64| if *value > CAP { Validity::Refuse(Message::new("Above the limit")) } else { Validity::Ok };
    Slider::new(Bind::bound(data.value("over", 95.0)).check(refuse_over), RANGE).label("Opacity").show(ui).into()
}

#[story(Slider<'static>, covers = [], drive = Drive::Never)]
fn slider_marks(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Slider::new(Bind::bound(data.value("value", 25.0)), RANGE).marks(&[0.0, 25.0, 50.0, 100.0]).label("Opacity").show(ui).into()
}

#[story(Slider<'static>, covers = [], drive = Drive::Never)]
fn slider_labelled_marks(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let marks = [(0.0, "Off"), (50.0, "Half"), (100.0, "Full")];
    Slider::new(Bind::bound(data.value("value", 50.0)), RANGE).marks_labelled(marks).label("Opacity").show(ui).into()
}

#[story(Slider<'static>, covers = [], drive = Drive::Never)]
fn slider_ticks(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Slider::new(Bind::bound(data.value("value", 40.0)), RANGE).ticks(10).label("Opacity").show(ui).into()
}

#[story(Slider<'static>, covers = [], drive = Drive::Never)]
fn slider_triangle(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Slider::new(Bind::bound(data.value("value", 40.0)), RANGE).thumb(SliderThumb::Triangle).label("Hue").show(ui).into()
}

#[story(Slider<'static>, covers = [], drive = Drive::Never)]
fn slider_unfilled(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Slider::new(Bind::bound(data.value("value", 40.0)), RANGE).fill(false).label("Opacity").show(ui).into()
}

#[story(Slider<'static>, covers = [], drive = Drive::Never)]
fn slider_value_text(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Slider::new(Bind::bound(data.value("value", 40.0)), RANGE).value_text("40 percent").label("Opacity").show(ui).into()
}

#[story(Slider<'static>, covers = [], drive = Drive::Never)]
fn slider_track_face(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Slider::new(Bind::bound(data.value("value", 40.0)), RANGE).track_face(TrackFace::new(Progress::new(1.0))).label("Hue").show(ui).into()
}
