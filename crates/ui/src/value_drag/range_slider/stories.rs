//! Stories of `RangeSlider`.

use super::{LOWER, RangeSlider};
use crate::contract::{Bind, Props as _, Show as _, Validity};
use crate::runtime::env::Message;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_behavior::Mixed;
use egui::Ui;

const RANGE: std::ops::RangeInclusive<f64> = 0.0..=100.0;
const CAP: f64 = 80.0;

#[story(RangeSlider<'static>, covers = [Default], drive = Drive::Drag(LOWER))]
fn range_slider(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    RangeSlider::new(Bind::bound(data.value("range", (20.0, 60.0))), RANGE).label("Level").show(ui).into()
}

#[story(RangeSlider<'static>, covers = [Mixed], drive = Drive::Never)]
fn range_slider_mixed(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    RangeSlider::new(Bind::bound_mixed(data.value("mixed", Mixed::Mixed)), RANGE).label("Level").show(ui).into()
}

#[story(RangeSlider<'static>, covers = [Invalid], drive = Drive::Never)]
fn range_slider_invalid(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let refuse_over = |range: &(f64, f64)| if range.1 > CAP { Validity::Refuse(Message::new("Above the limit")) } else { Validity::Ok };
    RangeSlider::new(Bind::bound(data.value("over", (30.0, 95.0))).check(refuse_over), RANGE).label("Level").show(ui).into()
}
