//! Stories of `CanvasShell` and `CanvasPanel`.

use super::{CanvasPanel, CanvasShell};
use crate::contract::Show as _;
use crate::display::Text;
use crate::runtime::env::WindowKey;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

#[story(CanvasShell<'static>, covers = [Default], drive = Drive::Never)]
fn canvas_shell(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    CanvasShell::new(WindowKey::main())
        .title(true, |ui, _| {
            Text::new("Title").show(ui);
        })
        .options_bar(true, |ui, _| {
            Text::new("Options").show(ui);
        })
        .tools(true, |ui, _| {
            Text::new("Tools").show(ui);
        })
        .status(true, |ui, _| {
            Text::new("Status").show(ui);
        })
        .body(|ui, _| {
            CanvasPanel::new()
                .tabs(|ui, _| {
                    Text::new("Tabs").show(ui);
                })
                .show(ui, |ui, _| {
                    Text::new("Document").show(ui);
                });
        })
        .show(ui)
        .into()
}

#[story(CanvasPanel<'static>, covers = [Default], drive = Drive::Never)]
fn canvas_panel(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    CanvasPanel::new()
        .rail("Panels", |ui, _| {
            Text::new("Rail").show(ui);
        })
        .tabs(|ui, _| {
            Text::new("Tabs").show(ui);
        })
        .show(ui, |ui, _| {
            Text::new("Document").show(ui);
        })
        .into()
}
