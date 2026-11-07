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
    let (settings, open) = data.value("pdf", (PdfSettings::default(), true));
    PdfExportDialog::new("Story", Bind::bound(settings)).opened(Bind::bound(open)).artboard_count(ARTBOARDS).show(ui).into()
}
