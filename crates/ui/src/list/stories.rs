//! Stories of the row types of the list family: `ListRow`.

use super::{ListRow, RowInk};
use crate::contract::Show as _;
use crate::primitives::IconSpec;
use crate::testing::{Drive, StoryData, StoryOutput, show_as_only_item, story};
use egui::Ui;

#[story(ListRow<'static>, covers = [Default], drive = Drive::Activate)]
fn list_row(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    ListRow::new("Background").show(ui).response.into()
}

#[story(ListRow<'static>, covers = [LongText], drive = Drive::Never)]
fn list_row_long_text(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let label = "A layer whose name is far longer than the room its row gives the label";
    ListRow::new(label).detail("A second line under it").show(ui).response.into()
}

#[story(ListRow<'static>, covers = [], drive = Drive::Never)]
fn list_row_tree_group(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    show_as_only_item(ui, ListRow::new("Group 1").depth(1).group(true).icon(IconSpec::svg(craft_behavior::IconId::new("eye")))).response.into()
}

#[story(ListRow<'static>, covers = [], drive = Drive::Never)]
fn list_row_clipped_dimmed(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    ListRow::new("Shadow").clipped(true).ink(RowInk::Dimmed).italic(true).lines("Clipped to the layer below, hidden").show(ui).response.into()
}
