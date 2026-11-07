//! Stories of `FloatingGroup`.

use super::FloatingGroup;
use crate::buttons::Button;
use crate::contract::Show as _;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use egui::{Pos2, Rect, Ui, Vec2};

const START: Rect = Rect { min: Pos2::new(40.0, 40.0), max: Pos2::new(280.0, 220.0) };
const LEAST: Vec2 = Vec2::new(120.0, 80.0);

#[story(FloatingGroup, covers = [Default], drive = Drive::Never)]
fn floating_group(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let rect = *data.value("rect", START);
    let shown = FloatingGroup::new("Layers", rect).limits(super::PanelLimits::at_least(LEAST)).show_group(ui, |ui, _| {
        Button::new("Add layer").show(ui);
    });
    if let Some(change) = shown.output.placed {
        *data.value("rect", START) = change.rect;
    }
    shown.output.into()
}
