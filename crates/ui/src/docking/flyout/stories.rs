//! Stories of `Flyout`.

use super::Flyout;
use crate::contract::{Props as _, ShowContainer as _};
use crate::docking::{IconicColumn, PanelSpec};
use crate::sections::Panel;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_behavior::dock::{Column, Group};
use egui::{Id, Ui};

fn spec(panel: &u8) -> PanelSpec {
    PanelSpec::new(format!("Panel {panel}"))
}

/// The flyout of the first icon, open from the first pass.
#[story(Flyout, covers = [Default], drive = Drive::Never)]
fn open_beside_its_column(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let column = Column::new(vec![Group::new(vec![0, 1], 0)], 240.0);
    IconicColumn::new("Panels", Id::new("flyout story"), &column, &spec)
        .open(0u8)
        .show_flyouts(ui, |group, rect, ui, _| {
            let title = group.active_panel().map_or_else(String::new, |panel| spec(panel).label.as_ref().to_owned());
            Panel::new(title).show_at(ui, rect, |_, _| ());
        })
        .into()
}
