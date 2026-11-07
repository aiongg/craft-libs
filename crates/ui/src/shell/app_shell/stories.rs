//! Stories of `AppShell` and `ShellRegion`.

use super::{AppShell, ShellRegion};
use crate::contract::{Show as _, ShowContainer as _};
use crate::display::Text;
use crate::runtime::env::WindowKey;
use crate::shell::Slot;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

#[story(AppShell<'static>, covers = [Default], drive = Drive::Never)]
fn app_shell(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    AppShell::new(WindowKey::main())
        .title_bar(|ui, _| {
            Text::new("Title").show(ui);
        })
        .top(Slot::new("options").reveal().label("Options"), |ui, _| {
            Text::new("Options").show(ui);
        })
        .bottom(Slot::new("status").label("Status"), |ui, _| {
            Text::new("Status").show(ui);
        })
        .start(Slot::new("tools").reveal().label("Tools"), |ui, _| {
            Text::new("Tools").show(ui);
        })
        .body(|ui, _| {
            Text::new("Document").show(ui);
        })
        .show(ui)
        .into()
}

#[story(ShellRegion, covers = [Default], drive = Drive::Never)]
fn shell_region(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    ShellRegion::window()
        .show(ui, |ui, _| {
            Text::new("Window").show(ui);
        })
        .into()
}
