//! Stories of `WindowDrag`.

use super::WindowDrag;
use crate::contract::Show as _;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use egui::{Rect, Ui, ViewportId};

#[story(WindowDrag, covers = [Default], drive = Drive::Never)]
fn window_drag(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let strip = Rect::from_min_size(ui.max_rect().min, egui::vec2(ui.available_width(), ui.spacing().interact_size.y));
    WindowDrag::new(ViewportId::ROOT).show_at(ui, strip).into()
}
