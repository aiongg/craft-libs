//! Stories of `PointPicker`.

use super::{PointPicker, WELL};
use crate::contract::{Bind, Props as _, Show as _, Validity};
use crate::runtime::env::Message;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_behavior::Mixed;
use egui::Ui;

const RANGE: std::ops::RangeInclusive<f64> = 0.0..=100.0;
const CENTER: [f64; 2] = [50.0, 50.0];
const CAP: f64 = 80.0;

#[story(PointPicker<'static>, covers = [Default], drive = Drive::Step)]
fn point_picker(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    PointPicker::new(Bind::bound(data.value("point", CENTER)), RANGE, RANGE).label("Blur center").show(ui).into()
}

#[story(PointPicker<'static>, covers = [], drive = Drive::Drag(WELL))]
fn point_picker_drag(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    PointPicker::new(Bind::bound(data.value("point", CENTER)), RANGE, RANGE).label("Blur center").show(ui).into()
}

#[story(PointPicker<'static>, covers = [Mixed], drive = Drive::Never)]
fn point_picker_mixed(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    PointPicker::new(Bind::bound_mixed(data.value("mixed", Mixed::Mixed)), RANGE, RANGE).label("Blur center").show(ui).into()
}

#[story(PointPicker<'static>, covers = [Invalid], drive = Drive::Never)]
fn point_picker_invalid(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let refuse_right = |point: &[f64; 2]| if point[0] > CAP { Validity::Refuse(Message::new("Too far right")) } else { Validity::Ok };
    PointPicker::new(Bind::bound(data.value("over", [90.0, 50.0])).check(refuse_right), RANGE, RANGE).label("Blur center").show(ui).into()
}

#[story(PointPicker<'static>, covers = [], drive = Drive::Never)]
fn point_picker_disabled(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    PointPicker::new(Bind::bound(data.value("point", CENTER)), RANGE, RANGE).label("Blur center").enabled(false).show(ui).into()
}
