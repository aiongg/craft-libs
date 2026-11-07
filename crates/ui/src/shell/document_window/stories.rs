//! Stories of `DocumentWindow`.

use super::DocumentWindow;
use crate::choice::Checkbox;
use crate::contract::{Bind, Props as _, Show as _, ShowContainer as _};
use crate::layout_components::Stack;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

#[story(DocumentWindow<'static>, covers = [Default], drive = Drive::Never)]
fn document_window(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let mut guides = *data.value("guides", true);
    let shown = DocumentWindow::new("Landscape.psd")
        .opened(Bind::bound(data.value("open", true)))
        .show(ui, |ui, _| {
            Stack::new().show(ui, |ui, _| {
                Checkbox::new(Bind::bound(&mut guides)).label("Show guides").show(ui);
            });
        })
        .output;
    *data.value("guides", true) = guides;
    shown.into()
}
