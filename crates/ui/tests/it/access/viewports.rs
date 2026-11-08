//! The order of a node's children, per viewport: a second window open while the main window's tree is checked.

use crate::body::kit::Slab;
use crate::contract::kit::{Panel, Probe, Seen};
use crate::windows_kit::Windows;
use craft_ui::contract::{ContainerProps, Props as _, Scroll, Show as _, ShowContainer as _};
use craft_ui::testing::access::NodeKey;
use craft_ui::testing::{Frames, trial};
use egui::{Id, Vec2, ViewportBuilder, ViewportId};
use std::cell::RefCell;

const CHILD: &str = "second window";
const LINE: Vec2 = Vec2::new(100.0, 40.0);

/// The node of `key`'s ancestors that is a direct child of `top`: the container's region that holds `key`.
fn child_holding(tree: &craft_ui::testing::access::AccessTree, key: NodeKey, top: NodeKey) -> Option<NodeKey> {
    let mut at = key;
    while let Some(parent) = tree.parent_of_key(at) {
        if parent == top {
            return Some(at);
        }
        at = parent;
    }
    None
}

fn child() -> ViewportId {
    ViewportId::from_hash_of(CHILD)
}

/// The nodes of three probes in show order below a container whose pass opens a second window between the second
/// and the third probe: the second window's pass runs nested inside the main window's.
#[trial]
fn a_second_window_adds_no_nodes_to_the_main_windows_tree_and_leaves_its_order() {
    let windows = Windows::new();
    let (first, second, third, kid) =
        (RefCell::new(Seen::default()), RefCell::new(Seen::default()), RefCell::new(Seen::default()), RefCell::new(Seen::default()));
    let mut panel = None;
    let mut out = None;
    for _ in 0..3 {
        out = Some(windows.frame(vec![], vec![], |ui| {
            let shown = Panel::new().label("Main").id_salt("main").show(ui, |ui, _| {
                Probe::new(&first).label("First").show(ui);
                Probe::new(&second).label("Second").show(ui);
                ui.ctx().show_viewport_immediate(child(), ViewportBuilder::default(), |cui, _| {
                    Probe::new(&kid).label("Kid").show(cui);
                });
                Probe::new(&third).label("Third").show(ui);
            });
            panel = Some(shown.output.id);
        }));
    }
    let out = out.expect("a frame ran");
    let (panel, ids) = (panel.expect("shown"), [&first, &second, &third].map(|seen| seen.borrow().id.expect("updated")));
    let kid = kid.borrow().id.expect("the second window ran");
    let (_, main) = out.trees.iter().find(|(id, _)| *id == ViewportId::ROOT).expect("the main window made a tree");
    let (_, other) = out.trees.iter().find(|(id, _)| *id == child()).expect("the second window made a tree");
    assert!(!main.has(kid), "the second window's node is not in the main window's tree");
    assert!(other.has(kid), "the second window's tree holds its own node");
    assert!(ids.iter().all(|id| !other.has(*id)), "the main window's nodes are not in the second window's tree");
    let keys = ids.map(NodeKey::of);
    let region = main.parent_of_key(keys[0]).expect("the first probe has a parent");
    assert!(keys.iter().all(|key| main.parent_of_key(*key) == Some(region)), "the probes share one parent, the container's body");
    assert!(child_holding(main, region, NodeKey::of(panel)).is_some() || region == NodeKey::of(panel), "the body is part of the container");
    let order: Vec<NodeKey> = main.children_of(region).into_iter().filter(|key| keys.contains(key)).collect();
    assert_eq!(order, keys, "the probes keep their show order below the container");
}

/// A scrolling container emits its scroll-bar part after the body: the body's nodes keep their show order among the
/// container's children, and every hidden node (a part's hit-test widget) that lies right of them follows them.
#[trial]
fn a_scroll_bar_part_keeps_its_emit_order_after_the_body() {
    let mut frames = Frames::new();
    let seen = [(); 4].map(|()| RefCell::new(Seen::default()));
    let mut slab = None;
    let mut show = |ui: &mut egui::Ui| {
        let shown = Slab::new(ContainerProps::new().scroll(Scroll::vertical())).label("Rows").id_salt("rows").show(ui, |ui, _| {
            for seen in &seen {
                Probe::new(seen).size(LINE * 2.0).label("Row").show(ui);
            }
        });
        slab = Some(shown.output.id);
    };
    frames.idle(4, &mut show);
    let tree = frames.tree();
    let (slab, rows) = (slab.expect("shown"), seen.each_ref().map(|seen| seen.borrow().id.expect("updated")));
    let slab = NodeKey::of(slab);
    let children = tree.children_of(slab);
    let region = |id: &Id| child_holding(&tree, NodeKey::of(*id), slab);
    let body = region(&rows[0]).expect("the first row is below the container");
    assert!(rows.iter().all(|id| region(id) == Some(body)), "every row is in the body's region");
    let holder = tree.parent_of_key(NodeKey::of(rows[0])).expect("the first row has a parent");
    assert!(rows.iter().all(|id| tree.parent_of_key(NodeKey::of(*id)) == Some(holder)), "the rows share one parent");
    let in_body = tree.children_of(holder);
    let positions: Vec<Option<usize>> = rows.iter().map(|id| in_body.iter().position(|key| *key == NodeKey::of(*id))).collect();
    assert!(positions.iter().all(Option::is_some) && positions.is_sorted(), "the rows keep their show order: {positions:?}");
    let at = |key: NodeKey| children.iter().position(|child| *child == key);
    let last_row = at(body).expect("the body's region is a child of the container");
    let bar = children.iter().enumerate().filter(|(_, key)| tree.is_hidden(**key)).filter(|(_, key)| {
        tree.bounds_of(**key)
            .is_some_and(|bounds| rows.iter().all(|id| tree.bounds_of(NodeKey::of(*id)).is_some_and(|row| bounds.min.x >= row.max.x)))
    });
    assert!(bar.into_iter().all(|(index, _)| index > last_row), "the scroll-bar part's node follows the body's nodes");
}
