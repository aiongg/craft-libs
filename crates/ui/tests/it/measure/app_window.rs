//! The bench's `app-window` shape: every body's contents run in at most one measuring run per frame.

use craft_behavior::Axis;
use craft_behavior::dock::{Column, DockEdge, Group, Layout, Node, Permissions, SplitSize, Workspace};
use craft_ui::contract::{Bind, BodyCx, Props as _, Show as _, ShowContainer as _};
use craft_ui::docking::{DockConfig, DockPanels, PanelSpec};
use craft_ui::layout_components::Stack;
use craft_ui::patterns::dock::DockArea;
use craft_ui::testing::layout::Block;
use craft_ui::testing::{Frames, measuring_runs, trial};
use egui::{Ui, Vec2};

struct Panels;

impl DockPanels<usize> for Panels {
    fn spec(&self, panel: &usize) -> PanelSpec {
        PanelSpec::new(format!("Panel {panel}"))
    }

    fn permissions(&self, _: &usize) -> Permissions {
        Permissions::ALL
    }

    fn body(&mut self, _: &usize, ui: &mut Ui, _: &mut BodyCx<'_>) {
        for i in 0..20 {
            Block::new(Vec2::new(100.0, 16.0)).id_salt(i).show(ui);
        }
    }
}

#[trial]
fn an_app_window_measures_each_body_at_most_once_per_frame() {
    let side = |panel| Node::dock(DockEdge::Start, vec![Column::new(vec![Group::new(vec![panel], 0)]).with_width(260.0)]);
    let root = Node::split(
        Axis::X,
        SplitSize::FixedFirst(260.0),
        side(1),
        Node::split(Axis::X, SplitSize::FixedFirst(260.0), side(2), Node::tabs(vec![3], 0)),
    );
    let mut workspace = Workspace::new(Layout::new(Some(root), Vec::new()));
    let mut frames = Frames::new();
    frames.screen(Vec2::new(1280.0, 800.0));
    let mut show = |ui: &mut Ui| {
        Stack::new().fill().show(ui, |ui, _| {
            let _ = DockArea::new("Dock", Bind::bound(&mut workspace), &mut Panels, DockConfig::default()).show(ui);
            let _ = Block::new(Vec2::new(1280.0, 22.0)).id_salt("status").show(ui);
        });
    };
    frames.idle(3, &mut show);
    let passes = frames.frame(vec![], &mut show);
    let runs = measuring_runs(&frames.ctx);
    eprintln!("passes {passes}; measuring runs per body: {runs:?}");
    assert!(runs.iter().all(|(_, count)| *count == 1), "a body measured more than once: {runs:?}");
}
