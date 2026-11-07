//! Stories of `WindowEdges`.

use super::WindowEdges;
use crate::contract::Show;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use egui::{Ui, ViewportId};

#[story(WindowEdges, covers = [Default], drive = Drive::Never)]
fn window_edges(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Show::show(WindowEdges::new(ViewportId::ROOT), ui).into()
}
