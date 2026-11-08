//! Stories of `IconicColumn`.

use super::CELL;
use crate::contract::ShowContainer as _;
use crate::docking::{IconicColumn, PanelSpec};
use crate::sections::Panel;
use crate::testing::memory::global;
use crate::testing::{Drive, Fixture, StoryData, StoryOutput, story};
use craft_behavior::dock::{Column, Group};
use egui::Ui;

fn spec(panel: &u8) -> PanelSpec {
    PanelSpec::new(format!("Panel {panel}"))
}

#[story(IconicColumn<'static, u8>, covers = [Default], drive = Drive::Never)]
fn icon_groups(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let column = Column::new(vec![Group::new(vec![0, 1], 0), Group::new(vec![2], 0)], 240.0);
    IconicColumn::new("Panels", global("iconic column", "story"), &column, &spec)
        .show_flyouts(ui, |group, rect, ui, _| {
            let title = group.active_panel().map_or_else(String::new, |panel| spec(panel).label.as_str().to_owned());
            Panel::new(title).show_at(ui, rect, |_, _| ());
        })
        .into()
}

/// A click on the first icon opens the flyout of its panel.
fn open_first_flyout(fixture: &mut Fixture) {
    let Some(id) = fixture.root_id() else { return };
    let Some(cell) = fixture.part(id, CELL.at(0)) else { return };
    fixture.click(&cell);
    fixture.idle(3);
}

#[story(IconicColumn<'static, u8>, covers = [Open], drive = Drive::Custom(open_first_flyout))]
fn icon_flyout_open(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let column = Column::new(vec![Group::new(vec![0, 1], 0), Group::new(vec![2], 0)], 240.0);
    IconicColumn::new("Panels", global("iconic column", "open story"), &column, &spec)
        .show_flyouts(ui, |group, rect, ui, _| {
            let title = group.active_panel().map_or_else(String::new, |panel| spec(panel).label.as_str().to_owned());
            Panel::new(title).show_at(ui, rect, |_, _| ());
        })
        .into()
}
