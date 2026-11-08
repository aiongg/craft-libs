//! Stories of the column bar.

use super::ColumnBar;
use craft_ui::contract::{Bind, Show as _};
use craft_ui::testing::{Drive, Fixture, StoryData, StoryOutput, story};
use egui::Ui;

fn drag_first_divider(fixture: &mut Fixture) {
    let Some(divider) = fixture.by_label("Column 1 width") else { return };
    let travel = 2.0 * craft_ui::testing::input::max_click_dist(fixture.ctx());
    fixture.drag(&divider, egui::Vec2::new(travel, 0.0));
}

#[story(ColumnBar<'static>, covers = [Default], drive = Drive::Custom(drag_first_divider))]
fn three_columns(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let widths = data.value("widths", vec![50.0, 30.0, 20.0]);
    ColumnBar::new("Columns", Bind::bound(widths)).show(ui).into()
}
