//! The order of a node's children, per viewport: a second window open while the main window's tree is checked.

use crate::contract::kit::{Panel, Probe, Seen};
use crate::windows_kit::Windows;
use craft_behavior::CanvasView;
use craft_behavior::kurbo::{Point, Rect as DocRect};
use craft_ui::canvas::{Canvas, CanvasScrollbars};
use craft_ui::contract::{Bind, Props as _, Role, Show as _, ShowContainer as _};
use craft_ui::testing::access::{Declared, NodeKey};
use craft_ui::testing::focus::AccessActionKind;
use craft_ui::testing::{Fixture, Frames, trial};
use egui::{Pos2, Rect, Vec2, ViewportBuilder, ViewportId};
use std::cell::RefCell;
use std::rc::Rc;

const CHILD: &str = "second window";
const LINE: Vec2 = Vec2::new(100.0, 40.0);
/// The canvas view and a document much larger than it, so both bars show.
const VIEW: Rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(400.0, 300.0));
const DOCUMENT: f64 = 2000.0;

/// The nodes below `top` in tree order: a container's regions are generic containers that parent's children lists
/// pass through, so the order of a container's children is the order of this walk.
fn below(tree: &craft_ui::testing::access::AccessTree, top: NodeKey) -> Vec<NodeKey> {
    let mut walk = Vec::new();
    for child in tree.children_of(top) {
        walk.push(child);
        walk.extend(below(tree, child));
    }
    walk
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
    let order: Vec<NodeKey> = below(main, NodeKey::of(panel)).into_iter().filter(|key| keys.contains(key)).collect();
    assert_eq!(order, keys, "the probes keep their show order below the container");
}

/// A scroll-bar component shown after the body's nodes: its scroll-bar nodes follow the body's among the canvas's
/// children. (A container's own scroll-bar parts declare no node; a scroll bar a screen reader reaches is a
/// `CanvasScrollbars` component.)
#[trial]
fn a_scroll_bar_node_follows_the_body_nodes_in_tree_order() {
    let mut frames = Frames::new();
    let seen = [(); 2].map(|()| RefCell::new(Seen::default()));
    let edited = RefCell::new(CanvasView::new(VIEW, Point::new(0.0, 0.0), 1.0));
    let document = DocRect::new(0.0, 0.0, DOCUMENT, DOCUMENT);
    let mut canvas = None;
    let mut show = |ui: &mut egui::Ui| {
        let view = *edited.borrow();
        let shown = Canvas::new("Document").view(view).show(ui, |ui, body| {
            for seen in &seen {
                Probe::new(seen).size(LINE).label("Row").show(ui);
            }
            let _ = CanvasScrollbars::new("Scroll bars", Bind::bound(&mut *edited.borrow_mut()), document).show_at(ui, body.rect);
        });
        canvas = Some(shown.output.response.id);
    };
    frames.idle(4, &mut show);
    let tree = frames.tree();
    let (canvas, rows) = (canvas.expect("shown"), seen.each_ref().map(|seen| seen.borrow().id.expect("updated")));
    let walk = below(&tree, NodeKey::of(canvas));
    let at = |key: NodeKey| walk.iter().position(|node| *node == key);
    let positions: Vec<Option<usize>> = rows.iter().map(|id| at(NodeKey::of(*id))).collect();
    assert!(positions.iter().all(Option::is_some), "the rows are below the canvas: {positions:?}");
    let last_row = positions.iter().copied().flatten().max().unwrap_or(0);
    let bars: Vec<Option<usize>> = tree.with_role(Role::SCROLL_BAR).into_iter().map(at).collect();
    assert!(!bars.is_empty(), "the canvas has scroll-bar nodes");
    assert!(
        bars.iter().all(|bar| bar.is_some_and(|index| index > last_row)),
        "the scroll-bar nodes follow the body's nodes: {bars:?} after {last_row}"
    );
}

/// A document just longer than the view, so the thumb covers each track's centre.
const BARELY_LONGER: f64 = 410.0;

/// The canvas scroll bars of a view over a document `extent` wide and high, and the view they edit.
fn scroll_bars(extent: f64) -> (Fixture, Rc<RefCell<CanvasView>>) {
    let edited = Rc::new(RefCell::new(CanvasView::new(VIEW, Point::new(0.0, 0.0), 1.0)));
    let document = DocRect::new(0.0, 0.0, extent, extent);
    let shared = edited.clone();
    let mut fixture = Fixture::new(move |ui| {
        let view = *shared.borrow();
        let _ = Canvas::new("Document").view(view).show(ui, |ui, body| {
            let _ = CanvasScrollbars::new("Scroll bars", Bind::bound(&mut *shared.borrow_mut()), document).show_at(ui, body.rect);
        });
    });
    fixture.settle();
    (fixture, edited)
}

/// A scroll track pages toward its centre; where the thumb covers the centre a click pages nowhere, so the track
/// declares no Click, and where it does page, the track declares Click and a Click pages.
#[trial]
fn a_scroll_track_declares_click_only_where_a_click_pages() {
    let (fixture, _) = scroll_bars(BARELY_LONGER);
    let bars = fixture.by_role(Role::SCROLL_BAR);
    assert!(!bars.is_empty(), "the canvas has scroll-bar nodes");
    assert!(
        bars.iter().all(|bar| !fixture.tree().declares_key(bar.node, Declared::Click)),
        "a track whose thumb covers its centre declares no Click"
    );
    let (mut fixture, edited) = scroll_bars(DOCUMENT);
    let before = *edited.borrow();
    let bars = fixture.by_role(Role::SCROLL_BAR);
    assert!(bars.iter().all(|bar| fixture.tree().declares_key(bar.node, Declared::Click)), "a track that pages declares Click");
    fixture.access(AccessActionKind::Click, &bars[0]);
    fixture.idle(2);
    assert_ne!(*edited.borrow(), before, "a Click on the track pages the view");
}
