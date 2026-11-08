//! Stories of the Crop tool on a canvas.

use super::{CropDocument, CropInput, CropSettings, CropTool};
use crate::ui::crop::CropDocKey;
use craft_behavior::geometry::CropBox;
use craft_behavior::kurbo::{Rect, Size};
use craft_ui::canvas::{Canvas, ToolHost};
use craft_ui::contract::ShowContainer as _;
use craft_ui::testing::{StoryData, StoryOutput, story};
use egui::Ui;

/// The document the stories crop.
const DOCUMENT: Size = Size::new(400.0, 300.0);

/// A frame turned on the document.
fn frame() -> CropBox {
    CropBox::new(Rect::new(60.0, 50.0, 300.0, 220.0), 12.0)
}

/// Shows `tool` on a canvas with `settings`, as an app does.
fn on_canvas(ui: &mut Ui, mut tool: CropTool, settings: &CropSettings) -> StoryOutput {
    let document = CropDocument::new(CropDocKey(1), DOCUMENT);
    let mut pass = tool.pass(CropInput::new(settings, &document));
    let shown = Canvas::new("Document").document(DOCUMENT).show(ui, |ui, body| pass.show_frame(ui, body.rect));
    let _ = ToolHost::new().run(ui, &shown.output, &mut pass);
    shown.output.into()
}

fn classic() -> CropSettings {
    let mut settings = CropSettings::default();
    settings.shield.classic_mode = true;
    settings
}

#[story(Canvas, covers = [Default])]
fn crop_classic(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let mut tool = CropTool::new();
    tool.crop_to(frame());
    on_canvas(ui, tool, &classic())
}

#[story(Canvas, covers = [Default])]
fn crop_image_mode(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let mut tool = CropTool::new();
    tool.crop_to(frame());
    on_canvas(ui, tool, &CropSettings::default())
}

#[story(Canvas, covers = [Default])]
fn crop_default_frame(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    on_canvas(ui, CropTool::new(), &classic())
}

#[story(Canvas, covers = [Default])]
fn crop_straighten_armed(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let mut tool = CropTool::new();
    tool.crop_to(frame());
    tool.toggle_straighten();
    on_canvas(ui, tool, &classic())
}
