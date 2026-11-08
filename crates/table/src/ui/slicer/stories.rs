//! Stories of `Slicer`.

use super::Slicer;
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

fn colours() -> Vec<(String, bool)> {
    ["Red", "Green", "Blue", "Cyan", "Magenta"].iter().map(|name| ((*name).to_owned(), *name == "Green")).collect()
}

#[story(Slicer<'static>, covers = [Default], drive = Drive::Never)]
fn slicer(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let items = colours();
    let rect = ui.available_rect_before_wrap();
    Slicer::new("Colour", &items).columns(2).filtered(true).show(ui, rect).into()
}
