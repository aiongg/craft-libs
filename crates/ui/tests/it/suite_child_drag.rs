//! The suite's `values` case drives a story through the part of a component the subject shows inside itself: a
//! slider shown in a field changes only through its own thumb, and the field is the story's subject.

use craft_behavior::{PartKey, PartRef};
use craft_ui::contract::{Bind, Props as _, Show as _, ShowContainer as _};
use craft_ui::form::Field;
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use craft_ui::value_drag::Slider;
use egui::Ui;

const THUMB: PartKey = PartKey::new("thumb");

#[story(Slider<'static>, covers = [], drive = Drive::DragChild(PartRef { key: THUMB, index: None }))]
fn slider_in_a_field(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let shown = Field::new("Opacity").show(ui, |ui, _| Slider::new(Bind::bound(data.value("value", 40.0)), 0.0..=100.0).label("Opacity").show(ui));
    let field = StoryOutput::from(shown.output);
    let mut output: StoryOutput = shown.inner.map(StoryOutput::from).unwrap_or_default();
    output.id = field.id;
    output
}
