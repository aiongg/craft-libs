//! Stories of the row types of the list family: `ListRow`.

use super::{ListRow, RowInk};
use crate::contract::Show as _;
use crate::primitives::IconSpec;
use crate::testing::{Drive, StoryData, StoryOutput, show_as_only_item, story};
use craft_behavior::ColumnKey;
use egui::{Pos2, Rect, Ui, Vec2};

#[story(ListRow<'static>, covers = [Default], drive = Drive::Activate)]
fn list_row(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    ListRow::new("Background").show(ui).into()
}

#[story(ListRow<'static>, covers = [LongText], drive = Drive::Never)]
fn list_row_long_text(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let label = "A layer whose name is far longer than the room its row gives the label";
    ListRow::new(label).detail("A second line under it").show(ui).into()
}

#[story(ListRow<'static>, covers = [], drive = Drive::Never)]
fn list_row_tree_group(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    show_as_only_item(ui, ListRow::new("Group 1").depth(1).group(true).icon(IconSpec::svg(craft_behavior::IconId::new("eye")))).into()
}

#[story(ListRow<'static>, covers = [], drive = Drive::Never)]
fn list_row_clipped_dimmed(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    ListRow::new("Shadow").clipped(true).ink(RowInk::Dimmed).italic(true).lines("Clipped to the layer below, hidden").show(ui).into()
}

#[story(ListRow<'static>, covers = [], drive = Drive::Never)]
fn list_row_text_cell(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    const NAME: ColumnKey = ColumnKey::new("name");
    const SIZE: ColumnKey = ColumnKey::new("size");
    let (left, width) = (ui.cursor().min.x, ui.available_width());
    let cells = [
        (NAME, Rect::from_min_size(Pos2::new(left, 0.0), Vec2::new(width * 0.6, 20.0))),
        (SIZE, Rect::from_min_size(Pos2::new(left + width * 0.6, 0.0), Vec2::new(width * 0.4, 20.0))),
    ];
    ListRow::new("report.pdf").cells(&cells).cell(SIZE, "2.4 MB").show(ui).into()
}
