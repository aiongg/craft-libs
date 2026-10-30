//! The "App preview" story: the mock app at a fixed height, so it can be seen before the Theme
//! workspace hosts it.

use craft_ui::story::{Level, Story, StoryArgs, Surface};
use craft_ui::typed_story;
use egui::{Sense, Ui, UiBuilder, vec2};

use super::AppPreview;

/// The story's height in points.
const HEIGHT: f32 = 800.0;

pub(crate) fn stories() -> Vec<Story> {
    vec![typed_story!("patterns/app-preview/photo-editor", Level::Patterns, "AppPreview", "App preview", NoArgs {}, show).on(Surface::Own)]
}

/// The story has no args.
#[derive(StoryArgs, Debug, PartialEq)]
struct NoArgs {}

/// The app's state lives in egui's temporary memory under the story's id: `AppPreview` is
/// `Clone`, and the story function is a plain `fn` with nowhere to keep it.
fn show(ui: &mut Ui, _args: &mut NoArgs) {
    let key = ui.make_persistent_id("app-preview-state");
    let mut preview = ui.data_mut(|d| d.remove_temp::<AppPreview>(key)).unwrap_or_default();
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), HEIGHT), Sense::hover());
    ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| preview.show(ui));
    ui.data_mut(|d| d.insert_temp(key, preview));
}
