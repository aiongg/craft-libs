//! Stories of the value controls: `Slider`, `Disclosure` and `Select`.

use super::disclosure::Disclosure;
use super::slider::{Slider, THUMB};
use crate::contract::{Bind, Props as _, Show as _, Validity};
use crate::runtime::env::Message;
use crate::testing::{Drive, StoryData, StoryOutput, story};
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
fn slider_disabled(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Slider::new(Bind::bound(data.value("value", 40.0)), RANGE).label("Opacity").enabled(false).show(ui).into()
}

#[story(Disclosure<'static>, covers = [Default])]
fn disclosure(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Disclosure::new(Bind::bound(data.value("open", false))).label("Details").show(ui).into()
}

#[story(Disclosure<'static>, covers = [Mixed], drive = Drive::Never)]
fn disclosure_mixed(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    Disclosure::new(Bind::bound_mixed(data.value("mixed", Mixed::Mixed))).label("Details").show(ui).into()
}

#[story(Disclosure<'static>, covers = [Invalid], drive = Drive::Never)]
fn disclosure_invalid(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let refuse_open = |open: &bool| if *open { Validity::Refuse(Message::new("Nothing to show")) } else { Validity::Ok };
    Disclosure::new(Bind::bound(data.value("open", true)).check(refuse_open)).label("Details").show(ui).into()
}
