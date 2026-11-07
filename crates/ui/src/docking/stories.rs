//! Stories of the docking family.

use super::{DockGroup, HANDLE, PanelKey, PanelTitle, SplitArea, Splitter, group_rects};
use crate::contract::{Bind, Props as _, Show as _, ShowContainer as _, Validity};
use crate::runtime::env::Message;
use crate::testing::{Drive, Fixture, StoryData, StoryOutput, story};
use craft_behavior::{Axis, DockTree, GroupId, Mixed};
use egui::{Ui, Vec2};

fn two_columns() -> DockTree {
    DockTree::split(Axis::X, vec![(1.0, DockTree::group(GroupId::new(0))), (2.0, DockTree::group(GroupId::new(1)))])
}

fn panels_of(group: GroupId) -> Vec<PanelTitle> {
    let key = |at: u64| PanelKey(u64::from(group.0) * 10 + at);
    match group.0 {
        0 => vec![PanelTitle::new(key(0), "Layers"), PanelTitle::new(key(1), "Channels")],
        _ => vec![PanelTitle::new(key(0), "Properties")],
    }
}

fn active_of(panels: &[PanelTitle]) -> PanelKey {
    panels.first().map_or(PanelKey(0), |panel| panel.key)
}

/// A split area over `bind` whose body shows the groups of `shown`.
fn dock(ui: &mut Ui, bind: Bind<'_, DockTree>, shown: &DockTree) -> StoryOutput {
    let area = SplitArea::new("Panels", bind).show(ui, |ui, body| {
        for (group, rect) in group_rects(shown, body.rect, body.env.direction().is_rtl()) {
            let panels = panels_of(group);
            DockGroup::new(group, &panels, active_of(&panels)).show_at(ui, rect, |_, _| ());
        }
    });
    area.output.edit.into()
}

/// Drags the handle after the first group by twice the drag threshold.
fn drag_handle(fixture: &mut Fixture) {
    let Some(id) = fixture.root_id() else { return };
    let Some(grip) = fixture.part(id, HANDLE.at(0)) else { return };
    let travel = 2.0 * crate::testing::input::max_click_dist(fixture.ctx());
    fixture.drag(&grip, Vec2::new(travel, 0.0));
}

#[story(SplitArea<'static>, covers = [Default], drive = Drive::Custom(drag_handle))]
fn columns(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let tree = data.value("dock", two_columns());
    let shown = tree.clone();
    dock(ui, Bind::bound(tree), &shown)
}

#[story(SplitArea<'static>, covers = [Mixed], drive = Drive::Custom(drag_handle))]
fn trees_that_differ(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let tree = data.value("dock", Mixed::<DockTree>::Mixed);
    dock(ui, Bind::bound_mixed(tree), &two_columns())
}

#[story(SplitArea<'static>, covers = [Invalid], drive = Drive::Custom(drag_handle))]
fn a_dock_of_one_group(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let tree = data.value("dock", DockTree::group(GroupId::new(0)));
    let shown = tree.clone();
    let needs_two = |tree: &DockTree| {
        if tree.groups().len() < 2 { Validity::Refuse(Message::new("A dock needs two groups")) } else { Validity::Ok }
    };
    dock(ui, Bind::bound(tree).check(needs_two), &shown)
}

#[story(DockGroup<'static>, covers = [Default])]
fn two_panels(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let panels = panels_of(GroupId::new(0));
    DockGroup::new(GroupId::new(0), &panels, active_of(&panels)).show(ui, |_, _| ()).output.into()
}

#[story(Splitter, covers = [Default])]
fn between_columns(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Splitter::new(Axis::X).label("Resize panels").show(ui).into()
}

#[story(Splitter, covers = [Default])]
fn between_rows(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Splitter::new(Axis::Y).label("Resize panels").show(ui).into()
}
