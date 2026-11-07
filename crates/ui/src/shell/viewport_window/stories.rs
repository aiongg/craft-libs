//! Stories of `ViewportWindow`.

use super::ViewportWindow;
use crate::contract::Show as _;
use crate::display::Text;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

#[story(ViewportWindow, covers = [], drive = Drive::Never)]
fn viewport_window(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    ViewportWindow::new("Export Log")
        .show(ui, |ui, _| {
            let _ = Text::new("The export finished.").show(ui);
        })
        .into()
}

#[story(ViewportWindow, covers = [], drive = Drive::Never)]
fn viewport_window_without_app_modal(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    ViewportWindow::new("Tool Window")
        .app_modal(false)
        .show(ui, |ui, _| {
            let _ = Text::new("Other windows keep taking input.").show(ui);
        })
        .into()
}
