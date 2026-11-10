//! Stories of the kit's stand-in `Block`.

use super::sample::words;
use super::{Drive, Grip, StoryData, StoryOutput, story};
use crate::contract::{Bind, Props as _, Show as _, ShowContainer as _};
use crate::form::Field;
use crate::sections::FooterBar;
use crate::testing::layout::Block;
use crate::value_drag::Slider;
use egui::{Ui, Vec2};

/// The size of a trial block, in points.
const CELL: Vec2 = Vec2::new(40.0, 20.0);

#[story(Block, covers = [Default])]
fn block(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Block::new(CELL).show(ui).into()
}

#[story(FooterBar, covers = [Default])]
fn footer_bar(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    FooterBar::body().show(ui, |ui, _| words(ui, 0..1)).into()
}

/// A slider shown in a field: the story's subject is the slider, and its value changes through its own thumb.
#[story(Slider<'static>, covers = [], drive = Drive::Drag(Grip::new(Slider::THUMB)))]
fn slider_in_a_field(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let shown = Field::new("Opacity").show(ui, |ui, _| Slider::new(Bind::bound(data.value("value", 40.0)), 0.0..=100.0).label("Opacity").show(ui));
    shown.inner.map(StoryOutput::from).unwrap_or_default()
}
