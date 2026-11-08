//! Stories of the kit's stand-in `Block`.

use super::{Drive, StoryData, StoryOutput, story};
use crate::contract::{Bind, Props as _, Show as _, ShowContainer as _};
use crate::form::Field;
use crate::sections::FooterBar;
use crate::testing::layout::Block;
use crate::value_drag::Slider;
use craft_behavior::{PartKey, PartRef};
use egui::{Ui, Vec2};

/// The size of a trial block, in points.
const CELL: Vec2 = Vec2::new(40.0, 20.0);

#[story(Block, covers = [Default])]
fn block(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Block::new(CELL).show(ui).into()
}

#[story(FooterBar, covers = [Default])]
fn footer_bar(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    FooterBar::body()
        .show(ui, |ui, _| {
            Block::new(CELL).show(ui);
        })
        .into()
}

/// The thumb of the slider a story shows inside a field.
const THUMB: PartRef = PartRef { key: PartKey::new("thumb"), index: None };

/// A slider shown in a field: the value changes only through the slider's own thumb, a part of a component the
/// subject (the field) shows inside itself, which the `values` case reaches with `Drive::DragChild`.
#[story(Slider<'static>, covers = [], drive = Drive::DragChild(THUMB))]
fn slider_in_a_field(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let shown = Field::new("Opacity").show(ui, |ui, _| Slider::new(Bind::bound(data.value("value", 40.0)), 0.0..=100.0).label("Opacity").show(ui));
    let field = StoryOutput::from(shown.output);
    let mut output = shown.inner.map(StoryOutput::from).unwrap_or_default();
    output.id = field.id;
    output
}
