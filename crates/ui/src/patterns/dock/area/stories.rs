//! Stories of the dock area.

use super::DockArea;
use crate::contract::{Bind, BodyCx, Show as _};
use crate::display::Text;
use crate::docking::{DockConfig, DockPanels, PanelLimits, PanelSpec};
use crate::runtime::memory::unique;
use crate::testing::{Drive, StoryData, StoryOutput, story};
use craft_behavior::Axis;
use craft_behavior::dock::{Column, DockEdge, Floating, Group, Layout, Node, Permissions, SplitSize, Workspace};
use egui::{Pos2, Rect, Ui, Vec2};

const LAYERS: usize = 0;
const CHANNELS: usize = 1;
const PROPERTIES: usize = 2;
const HISTORY: usize = 3;
const CANVAS: usize = 4;
const SWATCHES: usize = 5;
const SIDE: f32 = 220.0;

/// Panels that show their own name.
struct Demo {
    /// The canvas group is drawn by the host.
    canvas_view: bool,
}

fn name(panel: usize) -> &'static str {
    match panel {
        LAYERS => "Layers",
        CHANNELS => "Channels",
        PROPERTIES => "Properties",
        HISTORY => "History",
        SWATCHES => "Swatches",
        _ => "Canvas",
    }
}

impl DockPanels<usize> for Demo {
    fn spec(&self, panel: &usize) -> PanelSpec {
        let spec = PanelSpec::new(name(*panel)).limits(PanelLimits::at_least(Vec2::new(120.0, 60.0)));
        if *panel == CANVAS { spec.canvas() } else { spec }
    }

    fn permissions(&self, panel: &usize) -> Permissions {
        if *panel == CANVAS { Permissions::PROTECTED } else { Permissions::ALL }
    }

    fn body(&mut self, panel: &usize, ui: &mut Ui, _: &mut BodyCx<'_>) {
        Text::new(name(*panel)).show(ui);
    }

    fn group_view(&mut self, panels: &[usize], _: usize, ui: &mut Ui, _: Rect, _: &mut BodyCx<'_>) -> bool {
        let shown = self.canvas_view && panels.contains(&CANVAS);
        if shown {
            Text::new("Document").show(ui);
        }
        shown
    }
}

/// A canvas between a docked column of panels at the start and an iconic column at the end, with one floating group.
fn workspace() -> Workspace<usize> {
    let side = Column::new(vec![Group::new(vec![LAYERS, CHANNELS], 0), Group::new(vec![PROPERTIES], 0)], SIDE);
    let mut icons = Column::new(vec![Group::new(vec![SWATCHES], 0)], SIDE);
    icons.iconic = true;
    let centre = Node::split(Axis::X, SplitSize::Ratio(0.8), Node::tabs(vec![CANVAS], 0), Node::dock(DockEdge::End, vec![icons]));
    let root = Node::split(Axis::X, SplitSize::FixedFirst(SIDE), Node::dock(DockEdge::Start, vec![side]), centre);
    let window = Floating::single(vec![HISTORY], 0, Rect::from_min_size(Pos2::new(320.0, 80.0), Vec2::new(200.0, 140.0))).with_id(1);
    Workspace::new(Layout::new(Some(root), vec![window]))
}

fn dock(ui: &mut Ui, data: &mut StoryData, demo: &mut Demo) -> StoryOutput {
    let workspace = data.value("workspace", workspace());
    let id = unique(("dock area story", demo.canvas_view));
    let output = DockArea::new(id, Bind::bound(&mut *workspace), demo, DockConfig::default()).show(ui);
    for action in output.actions.iter().cloned() {
        // A refused action leaves the workspace as it was.
        let _ = workspace.apply(action, false, |panel| if *panel == CANVAS { Permissions::PROTECTED } else { Permissions::ALL });
    }
    output.into()
}

#[story(DockArea<'static, usize>, covers = [Default], drive = Drive::Never)]
fn docked_columns_a_floating_group_and_a_flyout(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    dock(ui, data, &mut Demo { canvas_view: false })
}

#[story(DockArea<'static, usize>, covers = [Default], drive = Drive::Never)]
fn a_canvas_group_the_host_draws(ui: &mut Ui, data: &mut StoryData) -> StoryOutput {
    dock(ui, data, &mut Demo { canvas_view: true })
}
