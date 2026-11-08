//! Stories of `CommandForm`.

use super::CommandForm;
use crate::contract::Bind;
use crate::patterns::form_dialogs::Param;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

const SPEC: &str = r#"{"name":text,"size":1..100=10,"method":"fit|fill","keep":bool}"#;

#[story(CommandForm<'static>, covers = [], drive = Drive::Never)]
fn command_form(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    CommandForm::new("fit-image", "Fit Image", Param::parse_list(SPEC)).opened(Bind::bound(data.value("open", true))).show(ui).into()
}
