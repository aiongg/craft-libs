//! Stories of the page preview.

use super::PagePreview;
use crate::ui::fixtures::DemoPages;
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

#[story(PagePreview<'static>, covers = [Open], drive = Drive::Never)]
fn open(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let at = data.value("at", 1_usize);
    let pages = DemoPages::new(5);
    let shown = [0, 1, 2, 3, 4];
    PagePreview::new(&pages, &shown, at).show(ui).into()
}
