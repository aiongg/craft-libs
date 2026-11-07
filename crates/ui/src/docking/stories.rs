//! Stories of the docking family.

use super::tree::{self, FrameRules, FrameSizes};
use super::{DockGroup, GroupGrip, HANDLE, PanelKey, PanelLimits, PanelTitle, SplitArea, Splitter};
use crate::contract::{Bind, Props as _, Show as _, ShowContainer as _, Validity};
use crate::runtime::env::Message;
use crate::testing::{Drive, Fixture, StoryData, StoryOutput, story};
use craft_behavior::dock::{Layout, Node, SplitSize};
use craft_behavior::{Axis, Mixed};
use egui::{Ui, Vec2};

const SIZES: FrameSizes = FrameSizes::new(1.0, 1.0, 40.0, 24.0);

const LAYERS: usize = 0;
const CHANNELS: usize = 1;
const PROPERTIES: usize = 10;

fn two_columns() -> Layout<usize> {
    let root = Node::split(Axis::X, SplitSize::Ratio(0.33), Node::tabs(vec![LAYERS, CHANNELS], 0), Node::tabs(vec![PROPERTIES], 0));
    Layout::new(Some(root), Vec::new())
}

fn title_of(panel: usize) -> PanelTitle {
    let name = match panel {
        LAYERS => "Layers",
        CHANNELS => "Channels",
        _ => "Properties",
    };
    PanelTitle::new(PanelKey::of(&panel), name)
}

fn titles(panels: &[usize]) -> Vec<PanelTitle> {
    panels.iter().map(|panel| title_of(*panel)).collect()
}

fn dock(ui: &mut Ui, bind: Bind<'_, Layout<usize>>, shown: &Layout<usize>) -> StoryOutput {
    let limits = |_: &usize| PanelLimits::default();
    let rules = FrameRules::new(SIZES, &limits);
    let area = SplitArea::new("Panels", bind, rules).show(ui, |ui, body| {
        let Some(root) = shown.root.as_ref() else { return };
        for group in tree::frames(root, body.rect, &rules, body.env.direction().is_rtl()).groups {
            let (Some(anchor), Node::Tabs { panels, .. }) = (group.anchor().copied(), group.node) else { continue };
            let panels = titles(panels);
            let active = panels.first().map_or(PanelKey::new(0), |panel| panel.key);
            DockGroup::new(anchor, &panels, active).show_at(ui, group.rect, |_, _| ());
        }
    });
    area.output.edit.into()
}

fn drag_handle(fixture: &mut Fixture) {
    let Some(id) = fixture.root_id() else { return };
    let Some(grip) = fixture.part(id, HANDLE.at(0)) else { return };
    let travel = 2.0 * crate::testing::input::max_click_dist(fixture.ctx());
    fixture.drag(&grip, Vec2::new(travel, 0.0));
}

#[story(SplitArea<'static, usize>, covers = [Default], drive = Drive::Custom(drag_handle))]
fn columns(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let layout = data.value("dock", two_columns());
    let shown = layout.clone();
    dock(ui, Bind::bound(layout), &shown)
}

#[story(SplitArea<'static, usize>, covers = [Mixed], drive = Drive::Custom(drag_handle))]
fn layouts_that_differ(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let layout = data.value("dock", Mixed::<Layout<usize>>::Mixed);
    dock(ui, Bind::bound_mixed(layout), &two_columns())
}

#[story(SplitArea<'static, usize>, covers = [Invalid], drive = Drive::Custom(drag_handle))]
fn a_dock_of_one_group(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    let layout = data.value("dock", Layout::new(Some(Node::tabs(vec![LAYERS], 0)), Vec::new()));
    let shown = layout.clone();
    let needs_two = |layout: &Layout<usize>| {
        if layout.panels().len() < 2 { Validity::Refuse(Message::new("A dock needs two panels")) } else { Validity::Ok }
    };
    dock(ui, Bind::bound(layout).check(needs_two), &shown)
}

#[story(DockGroup<'static, usize>, covers = [Default])]
fn two_panels(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let panels = titles(&[LAYERS, CHANNELS]);
    DockGroup::new(LAYERS, &panels, panels[0].key).show(ui, |_, _| ()).output.into()
}

#[story(GroupGrip, covers = [Default])]
fn a_group_grip(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    let key = PanelKey::of(&LAYERS);
    GroupGrip::new(key, key).label("Move tab group").show(ui).into()
}

#[story(Splitter, covers = [Default])]
fn between_columns(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Splitter::new(Axis::X).label("Resize panels").show(ui).into()
}

#[story(Splitter, covers = [Default])]
fn between_rows(ui: &mut Ui, _: &mut StoryData) -> StoryOutput {
    Splitter::new(Axis::Y).label("Resize panels").show(ui).into()
}
