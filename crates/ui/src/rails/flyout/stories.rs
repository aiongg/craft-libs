//! Stories of `FlyoutButton`.

use super::FlyoutButton;
use crate::contract::Show as _;
use crate::rails::tool::stories::{BRUSH, tools};
use crate::testing::{Drive, StoryData, StoryOutput, story};
use egui::Ui;

#[story(FlyoutButton<'static, u8>, covers = [Default], drive = Drive::Activate)]
fn flyout_button(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let tools = tools(&[BRUSH, BRUSH + 1, BRUSH + 2]);
    FlyoutButton::new(&tools, &BRUSH).show(ui).into()
}
