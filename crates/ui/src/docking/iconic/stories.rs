//! Stories of `IconicColumn`.

use crate::contract::ShowContainer as _;
use crate::docking::{IconicColumn, PanelSpec};
use crate::sections::Panel;
use crate::testing::memory::global;
use crate::testing::{Drive, StoryData, StoryOutput, story};
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
