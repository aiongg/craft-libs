//! Docking (20.6): a panel dragged from a tab lands on the group and side the pointer is over.

use craft_behavior::dock::{Layout, Node, SplitSize};
use craft_behavior::{Axis, DockSide};
use craft_theme::{Contrast, ControlSize, Mode, Variant};
use craft_ui::contract::{Bind, Role, ShowContainer as _};
use craft_ui::docking::tree::FrameRules;
use craft_ui::docking::{DockDrop, DockGroup, DockSizes, PanelKey, PanelLimits, PanelTitle, SplitArea};
use craft_ui::testing::style::{tokens, with_cx};
use craft_ui::testing::{Fixture, trial};
use egui::Pos2;
use std::cell::RefCell;
use std::rc::Rc;

mod render;

mod split;

mod drop;

mod floating;
mod spec;
mod targets;

const LAYERS: usize = 0;
const CHANNELS: usize = 1;
const PROPERTIES: usize = 10;

mod columns;

/// The docking sizes of the default theme with the given divider gap, least pane and strip height.
fn sizes(gap: f32, least_pane: f32, strip: f32) -> DockSizes {
    with_cx(&egui::Context::default(), &tokens(Mode::Light, Contrast::Standard), Variant::SECONDARY, ControlSize::Md, |cx| {
        let mut sizes = DockSizes::style(cx);
        sizes.gap = gap;
        sizes.least_pane = least_pane;
        sizes.strip = strip;
        sizes
    })
}

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
        let rules = FrameRules::new(&limits);
        let area = SplitArea::new("Panels", Bind::bound(&mut *layout), rules).show(ui, |ui, body| {
            for (anchor, rect) in body.group_frames::<usize>() {
                let Some(Node::Tabs { panels, .. }) = shown.place(&anchor).and_then(|place| shown.node_of(&place)) else { continue };
                let panels = titles(panels);
                let active = panels.first().map_or(PanelKey::new(0), |panel| panel.key);
                DockGroup::new(anchor, &panels, active).show_at(ui, rect, |_, _| ());
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
