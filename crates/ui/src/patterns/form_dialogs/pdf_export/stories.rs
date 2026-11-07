//! Stories of `PdfExportDialog`.

use super::PdfExportDialog;
use crate::contract::Bind;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_pdf::settings::PdfSettings;
use egui::Ui;

/// The artboards of the story's document.
const ARTBOARDS: usize = 5;

#[story(PdfExportDialog<'static>, covers = [], drive = Drive::Never)]
fn pdf_export_dialog(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let settings = data.value("settings", PdfSettings::default());
    PdfExportDialog::new("Story", Bind::bound(settings)).opened(Bind::bound(data.value("open", true))).artboard_count(ARTBOARDS).show(ui).into()
}
