//! Stories of the transform overlay: the open session's box in the transform phase.

use super::TransformOverlay;
use crate::ui::transform::{TransformSession, TransformSettings};
use craft_behavior::kurbo::Rect as DocRect;
use craft_ui::canvas::{Canvas, ToolPath};
use craft_ui::contract::ShowContainer as _;
use craft_ui::testing::{StoryData, StoryOutput, story};
use egui::Ui;

/// The layer's source frame, in document units.
const FRAME: DocRect = DocRect::new(200.0, 120.0, 440.0, 280.0);

const FREE_TRANSFORM: ToolPath = ToolPath::Command("the Transform fields of the options bar");

#[story(TransformOverlay<'static, ()>, covers = [Default])]
fn transform_phase(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let session = data.value("session", TransformSession::<()>::new(FRAME));
    let settings = TransformSettings::default();
    let shown = Canvas::new("Document", FREE_TRANSFORM)
        .show(ui, |ui, body| TransformOverlay::new("Free Transform", &*session, &settings).show_at(ui, body.rect));
    if let Some(output) = shown.inner {
        for edit in output.edits {
            session.apply(edit, 0.0);
        }
    }
    StoryOutput::default()
}
