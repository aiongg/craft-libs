//! Stories of `NewDocumentDialog`.

use super::NewDocumentDialog;
use crate::contract::Bind;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

#[story(NewDocumentDialog<'static>, covers = [], drive = Drive::Never)]
fn new_document_dialog(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    NewDocumentDialog::new().opened(Bind::bound(data.value("open", true))).show(ui).into()
}
