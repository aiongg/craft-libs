//! Stories of `AngleDial`.

use super::{AngleDial, FACE};
use crate::contract::{Bind, Props as _, Show as _, Validity};
use crate::runtime::env::Message;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_behavior::Mixed;
use egui::Ui;

const ANGLE: f64 = 120.0;
const CAP: f64 = 90.0;

#[story(AngleDial<'static>, covers = [Default], drive = Drive::Step)]
fn angle_dial(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    AngleDial::new(Bind::bound(data.value("angle", ANGLE))).label("Angle").show(ui).into()
}

#[story(AngleDial<'static>, covers = [], drive = Drive::Drag(FACE))]
fn angle_dial_drag(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    AngleDial::new(Bind::bound(data.value("angle", ANGLE))).label("Angle").show(ui).into()
}

#[story(AngleDial<'static>, covers = [Mixed], drive = Drive::Never)]
fn angle_dial_mixed(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    AngleDial::new(Bind::bound_mixed(data.value("mixed", Mixed::Mixed))).label("Angle").show(ui).into()
}

#[story(AngleDial<'static>, covers = [Invalid], drive = Drive::Never)]
fn angle_dial_invalid(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let refuse_over = |angle: &f64| if *angle > CAP { Validity::Refuse(Message::new("Above the limit")) } else { Validity::Ok };
    AngleDial::new(Bind::bound(data.value("over", ANGLE)).check(refuse_over)).label("Angle").show(ui).into()
}

#[story(AngleDial<'static>, covers = [], drive = Drive::Never)]
fn angle_dial_disabled(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    AngleDial::new(Bind::bound(data.value("angle", ANGLE))).label("Angle").enabled(false).show(ui).into()
}
