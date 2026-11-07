//! Docking (20.6): a panel dragged from a tab lands on the group and side the pointer is over.

use craft_behavior::dock::{Layout, Node, SplitSize};
use craft_behavior::{Axis, DockSide};
use craft_ui::contract::{Bind, Role, ShowContainer as _};
use craft_ui::docking::tree::{self, FrameRules, FrameSizes};
use craft_ui::docking::{DockDrop, DockGroup, PanelKey, PanelLimits, PanelTitle, SplitArea};
use craft_ui::testing::{Fixture, trial};
use egui::Pos2;
use std::cell::RefCell;
use std::rc::Rc;

mod render;

mod drop;

mod floating;
mod spec;
mod targets;

const SIZES: FrameSizes = FrameSizes::new(1.0, 1.0, 40.0, 24.0);
const LAYERS: usize = 0;
const CHANNELS: usize = 1;
const PROPERTIES: usize = 10;

mod columns;

fn two_columns() -> Layout<usize> {
    let root = Node::split(Axis::X, SplitSize::Ratio(0.33), Node::tabs(vec![LAYERS, CHANNELS], 0), Node::tabs(vec![PROPERTIES], 0));
    Layout::new(Some(root), Vec::new())
}

fn titles(panels: &[usize]) -> Vec<PanelTitle> {
    let name = |panel: usize| match panel {
        LAYERS => "Layers",
        CHANNELS => "Channels",
        _ => "Properties",
    };
    panels.iter().map(|panel| PanelTitle::new(PanelKey::of(panel), name(*panel))).collect()
}

type Dropped = Rc<RefCell<Option<(PanelKey, DockSide)>>>;

/// A dock of two groups that records the drop of a panel, as an app's `DockArea` would read it.
fn dock() -> (Fixture, Dropped) {
    let layout = Rc::new(RefCell::new(two_columns()));
    let dropped = Rc::new(RefCell::new(None));
    let shared_drop = Rc::clone(&dropped);
    let fixture = Fixture::new(move |ui| {
        let mut layout = layout.borrow_mut();
        let shown = layout.clone();
        let limits = |_: &usize| PanelLimits::default();
        let rules = FrameRules::new(SIZES, &limits);
        let area = SplitArea::new("Panels", Bind::bound(&mut *layout), rules).show(ui, |ui, body| {
            let Some(root) = shown.root.as_ref() else { return };
            for group in tree::frames(root, body.rect, &rules, body.env.direction().is_rtl()).groups {
                let (Some(anchor), Node::Tabs { panels, .. }) = (group.anchor().copied(), group.node) else { continue };
                let panels = titles(panels);
                let active = panels.first().map_or(PanelKey::new(0), |panel| panel.key);
                DockGroup::new(anchor, &panels, active).show_at(ui, group.rect, |_, _| ());
            }
        });
        if let Some(DockDrop { group, side, .. }) = area.output.dropped {
            *shared_drop.borrow_mut() = Some((group, side));
        }
    });
    (fixture, dropped)
}

#[trial]
fn a_panel_dropped_on_a_group_edge_docks_beside_it_and_in_the_middle_joins_its_tabs() {
    let (mut fixture, dropped) = dock();
    fixture.settle();
    let first_tab = |fixture: &Fixture| {
        fixture.by_role(Role::TAB).into_iter().min_by(|a, b| a.rect.min.x.total_cmp(&b.rect.min.x)).expect("the group shows its tabs")
    };
    let tab = first_tab(&fixture);

    let middle = Pos2::new(260.0, 150.0);
    fixture.drag(&tab, middle - tab.center());
    fixture.idle(2);
    assert_eq!(*dropped.borrow(), Some((PanelKey::of(&PROPERTIES), DockSide::Center)), "the middle of the right group joins its tabs");

    let edge = Pos2::new(395.0, 150.0);
    fixture.drag(&tab, edge - tab.center());
    fixture.idle(2);
    assert_eq!(*dropped.borrow(), Some((PanelKey::of(&PROPERTIES), DockSide::End)), "the end edge of the right group docks beside it");
}
