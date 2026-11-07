//! Stories of `SelectFace`.

use super::SelectFace;
use crate::contract::Show as _;
use crate::list::ThumbAspect;
use crate::testing::{StoryData, StoryOutput, story};
use egui::Ui;

#[story(SelectFace<'static>, covers = [])]
fn face_with_thumbnail_and_name(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    SelectFace::new("Soft light").thumb(ThumbAspect::SQUARE, |_, _| {}).show(ui).into()
}

#[story(SelectFace<'static>, covers = [])]
fn face_with_thumbnail_alone(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    SelectFace::new("Soft light").thumb(ThumbAspect::new(3, 2), |_, _| {}).name(false).show(ui).into()
}
