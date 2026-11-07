//! Stories of `PasswordDialog`.

use super::{PasswordDialog, PasswordPurpose};
use crate::contract::Bind;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

#[story(PasswordDialog<'static>, covers = [], drive = Drive::Never)]
fn password_dialog(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    PasswordDialog::new(PasswordPurpose::open("brochure.pdf")).opened(Bind::bound(data.value("open", true))).show(ui).into()
}

#[story(PasswordDialog<'static>, covers = [], drive = Drive::Never)]
fn password_dialog_new_password(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    PasswordDialog::new(PasswordPurpose::Set).opened(Bind::bound(data.value("open", true))).show(ui).into()
}
