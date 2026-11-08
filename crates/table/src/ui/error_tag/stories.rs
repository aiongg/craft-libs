//! Stories of the error tag.

use super::ErrorTag;
use craft_ui::contract::Show as _;
use craft_ui::runtime::env::Label;
use craft_ui::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

static FIXES: [Label; 3] = [Label::Static("Convert to Number"), Label::Static("Help on this Error"), Label::Static("Ignore Error")];

#[story(ErrorTag<'static>, covers = [Default], drive = Drive::Activate)]
fn error_tag(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    ErrorTag::new("Number Stored as Text", &FIXES).show(ui).into()
}
