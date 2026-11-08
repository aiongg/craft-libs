//! The story host of canvas tools: a tool is no component, so its story shows it on a canvas.

use super::StoryOutput;
use crate::canvas::{Canvas, CanvasTool, ToolHost};
use crate::contract::ShowContainer as _;
use egui::Ui;

/// Shows `tool` on a canvas named `label` over the room left in `ui`, and runs it as the canvas's active tool. The
/// output is the canvas's, so the suite reads the nodes the tool and the canvas left. The story builds the tool
/// anew each pass: it shows the tool's idle state, and a gesture belongs in a trial on [`ToolFixture`](super::ToolFixture).
///
/// The tool's type registers as a story host with `#[derive(Pattern)]` and `#[pattern(tool)]`.
pub fn tool_story(ui: &mut Ui, label: &'static str, tool: &mut dyn CanvasTool) -> StoryOutput {
    let shown = Canvas::new(label).show(ui, |_, _| ());
    ToolHost::new().run(ui, &shown.output, tool);
    shown.output.into()
}
