//! A flyout over a group the host's `group_view` takes: the flyout measures the host's view, not the dock's own group.

use craft_behavior::dock::{Column, DockEdge, Group, Layout, Node, Permissions, SplitSize, Workspace};
use craft_behavior::{Axis, Proposal};
use craft_ui::contract::{Bind, BodyCx, ShowContainer as _};
use craft_ui::docking::{DockConfig, DockPanels, GroupAt, PanelSpec};
use craft_ui::patterns::dock::DockArea;
use craft_ui::sections::Panel;
use craft_ui::testing::layout::Block;
use craft_ui::testing::{Fixture, trial};
use egui::{Rect, Ui, Vec2};
use std::cell::Cell;
use std::rc::Rc;

const SIDE: usize = 1;
const CANVAS: usize = 2;
const COLUMN_WIDTH: f32 = 200.0;
const AREA: Vec2 = Vec2::new(800.0, 600.0);
const BLOCK: Vec2 = Vec2::new(40.0, 30.0);
const BLOCKS: usize = 10;

/// The host's view of the side group: a panel of blocks, much taller than the flyout's least height.
fn view_body(ui: &mut Ui) -> Rect {
    let mut last = Rect::NOTHING;
    for salt in 0..BLOCKS {
        last = Block::new(BLOCK).id_salt(salt).show(ui).rect;
    }
    last
}

/// Panels whose side group the host shows itself, recording the rect it showed its view in and its last block.
struct HostView {
    seen: Rc<Cell<(Rect, Rect)>>,
}

impl DockPanels<usize> for HostView {
    fn spec(&self, panel: &usize) -> PanelSpec {
        PanelSpec::new(format!("Panel {panel}"))
    }

    fn permissions(&self, panel: &usize) -> Permissions {
        if *panel == CANVAS { Permissions::PROTECTED } else { Permissions::ALL }
    }

    fn body(&mut self, _: &usize, _: &mut Ui, _: &mut BodyCx<'_>) {}

    fn group_view(&mut self, panels: &[usize], _: usize, ui: &mut Ui, at: GroupAt, _: &mut BodyCx<'_>) -> Option<f32> {
        if !panels.contains(&SIDE) {
            return None;
        }
        match at {
            GroupAt::Measure(width) => Some(Panel::new("Host view").measure_in(ui, Proposal::Width(width), |ui, _| view_body(ui)).y),
            GroupAt::Show(rect) => {
                let mut last = Rect::NOTHING;
                let panel = Panel::new("Host view").show_at(ui, rect, |ui, _| last = view_body(ui)).output.response.rect;
                self.seen.set((panel, last));
                Some(rect.height())
            }
            _ => None,
        }
    }
}

#[trial]
fn a_flyout_over_a_host_view_is_as_tall_as_that_view() {
    let mut iconic = Column::new(vec![Group::new(vec![SIDE], 0)]).with_width(COLUMN_WIDTH);
    iconic.iconic = true;
    let root = Node::split(Axis::X, SplitSize::FixedFirst(COLUMN_WIDTH), Node::dock(DockEdge::Start, vec![iconic]), Node::tabs(vec![CANVAS], 0));
    let mut workspace = Workspace::new(Layout::new(Some(root), Vec::new()));
    let seen: Rc<Cell<(Rect, Rect)>> = Rc::new(Cell::new((Rect::NOTHING, Rect::NOTHING)));
    let shown = Rc::clone(&seen);
    let mut fixture = Fixture::new(move |ui| {
        let mut panels = HostView { seen: Rc::clone(&shown) };
        let _ = DockArea::new("Dock area", Bind::bound(&mut workspace), &mut panels, DockConfig::default()).show(ui);
    });
    fixture.room(AREA).settle();
    let icon = fixture.by_label("Panel 1").expect("the iconic column shows the side panel's icon");
    fixture.click(&icon);
    fixture.settle();
    let (panel, last) = seen.get();
    assert!(panel.is_finite(), "the flyout shows the host's view");
    assert!(panel.contains_rect(last), "the flyout holds the host's whole view: {panel:?} {last:?}");
}
