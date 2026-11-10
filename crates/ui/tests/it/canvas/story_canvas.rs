//! The story canvas: the document a story of a canvas control shows under its control.

use craft_ui::canvas::{Canvas, RasterSource};
use craft_ui::primitives::Checker;
use craft_ui::testing::canvas_story::{StoryDocument, on_canvas_with, origin_view};
use craft_ui::testing::{Frames, StoryOutput, stage, trial};
use egui::Shape;

/// The dark cells of the transparency grid a story canvas over `document` paints.
fn dark_cells(document: Option<&dyn RasterSource>) -> usize {
    let dark = Checker::PREFERENCE_DEFAULT.dark.map(|color| color.egui());
    let mut frames = Frames::new();
    let mut show = |ui: &mut egui::Ui| {
        let room = stage(ui);
        on_canvas_with(ui, room, Canvas::new("Document").view(origin_view(room, 1.0)), document, |_, _| StoryOutput::default());
    };
    frames.idle(1, &mut show);
    frames.frame_shapes(&mut show).iter().filter(|shape| matches!(shape, Shape::Rect(rect) if Some(rect.fill) == dark)).count()
}

#[trial]
fn a_story_canvas_shows_its_document_on_the_default_transparency_grid_and_a_bare_canvas_without_one() {
    assert!(dark_cells(Some(&StoryDocument::STANDARD)) > 0, "the document shows the transparency grid");
    assert_eq!(dark_cells(None), 0, "a story that draws its own document gets a bare canvas");
}
