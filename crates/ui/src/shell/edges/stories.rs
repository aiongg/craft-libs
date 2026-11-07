//! Stories of `WindowEdges`.

use super::WindowEdges;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use egui::{Ui, ViewportId};

#[story(WindowEdges, covers = [Default], drive = Drive::Never)]
fn window_edges(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    WindowEdges::new(ViewportId::ROOT).place(ui).into()
}
