//! Stories of `PropertiesDialog`.

use super::{PropertiesDialog, PropertiesTab, PropertyRow};
use crate::contract::Bind;
use crate::patterns::form_dialogs::{Param, ParamKind};
use crate::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

fn tabs() -> Vec<PropertiesTab> {
    vec![
        PropertiesTab::new(
            "General",
            vec![
                PropertyRow::value("Kind", "Photoshop document"),
                PropertyRow::value("Size", "3000 x 2000 px"),
                PropertyRow::heading("Description"),
                PropertyRow::field(Param::new("title", "Title", ParamKind::text(""))),
                PropertyRow::field(Param::new("locked", "Locked", ParamKind::bool(false))),
            ],
        ),
        PropertiesTab::new("Colour", vec![PropertyRow::value("Mode", "RGB"), PropertyRow::field(Param::number("opacity", "Opacity", 100.0, 1.0, 0))]),
    ]
}

#[story(PropertiesDialog<'static>, covers = [], drive = Drive::Never)]
fn properties_dialog(ui: &mut Ui, _data: &mut StoryData) -> StoryOutput {
    PropertiesDialog::new("Properties", tabs()).opened(Bind::given(true)).show(ui).into()
}

#[story(PropertiesDialog<'static>, covers = [], drive = Drive::Never)]
fn properties_dialog_read_only(ui: &mut Ui, _data: &mut StoryData) -> StoryOutput {
    PropertiesDialog::new("Properties", tabs()).read_only(true).opened(Bind::given(true)).show(ui).into()
}
