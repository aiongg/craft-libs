//! Dock transactions, workspaces and native placement.

use craft_behavior::dock::{
    Action, Column, DockEdge, DockError, DockPlacement, Floating, Group, Layout, NamedWorkspace, Node, PanelPlace, Permissions, SplitSize, Workspace,
    WorkspaceChrome, WorkspaceSet, kept_reachable, node_at,
};
use craft_behavior::{Axis, Monitor};
use emath::{Rect, pos2, vec2};

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_min_size(pos2(x, y), vec2(w, h))
}

fn window() -> Rect {
    rect(100.0, 100.0, 300.0, 400.0)
}

fn g(panels: &[u32], active: usize) -> Group<u32> {
    Group::new(panels.to_vec(), active)
}

fn tabs(panels: &[u32], active: usize) -> Node<u32> {
    Node::tabs(panels.to_vec(), active)
}

/// Canvas 1 above frame tabs [11, 12]; an end dock with column A ([2, 3], [4]) and column B ([5]); a floating
/// window with groups [6, 7] and [8].
fn docked() -> Layout<u32> {
    let middle = Node::split(Axis::Y, SplitSize::Ratio(0.7), tabs(&[1], 0), tabs(&[11, 12], 1));
    let dock = Node::dock(
        DockEdge::End,
        vec![Column::new(vec![g(&[2, 3], 1), g(&[4], 0)]).with_width(180.0), Column::new(vec![g(&[5], 0)]).with_width(120.0)],
    );
    let root = Node::split(Axis::X, SplitSize::FixedSecond(300.0), middle, dock);
    Layout::new(Some(root), vec![Floating::stack(vec![g(&[6, 7], 1), g(&[8], 0)], window()).unwrap()])
}

const ALL: [u32; 10] = [1, 2, 3, 4, 5, 6, 7, 8, 11, 12];

fn allow(_: &u32) -> Permissions {
    Permissions::ALL
}

fn sorted(layout: &Layout<u32>) -> Vec<u32> {
    let mut panels: Vec<u32> = layout.panels().into_iter().copied().collect();
    panels.sort_unstable();
    panels
}

fn group_members(layout: &Layout<u32>, panel: u32) -> Vec<u32> {
    match layout.place(&panel).unwrap() {
        place @ (PanelPlace::Node(_) | PanelPlace::Floating { .. }) => match layout.node_of(&place).unwrap() {
            Node::Tabs { panels, .. } => panels.clone(),
            other => panic!("{other:?}"),
        },
        place => layout.group(&place).unwrap().panels.clone(),
    }
}

fn end_dock(layout: &Layout<u32>) -> &Vec<Column<u32>> {
    let path = layout.dock_path(DockEdge::End).unwrap();
    let Some(Node::Dock { columns, .. }) = node_at(layout.root.as_ref().unwrap(), &path) else { panic!() };
    columns
}

#[test]
fn apply_all_changes_everything_or_nothing_and_reports_the_closed_panels() {
    let mut layout = docked();
    let closed = layout.apply_all([Action::close_group(3), Action::activate(4)], allow).unwrap();
    assert_eq!(closed, [2, 3]);
    assert!(!layout.contains(&2));

    let before = layout.clone();
    let keep = |p: &u32| if *p == 4 { Permissions::ALL.closable(false) } else { Permissions::ALL };
    assert_eq!(layout.apply_all([Action::close(8), Action::close(4)], keep), Err(DockError::NotAllowed));
    assert_eq!(layout, before);
}

#[test]
fn reconcile_drops_unknown_panels_and_places_missing_registered_ones() {
    let default = Workspace::new(docked());
    let mut workspace = default.clone();
    workspace.layout.apply(Action::close(5)).unwrap();
    workspace.layout.apply(Action::close(12)).unwrap();
    workspace.layout.apply(Action::open_at(99, 2, DockPlacement::tab(None))).unwrap();
    let stray = workspace.layout.location(&99).unwrap();
    workspace.hidden.push((98, stray));
    let registered = |p: &u32| *p < 90;
    workspace.reconcile(registered, allow, &default);
    workspace.validate().unwrap();
    // Unregistered panels wait in `hidden` and come back once registered.
    assert_eq!(workspace.hidden.iter().map(|(p, _)| *p).collect::<Vec<_>>(), [98, 99]);
    assert_eq!(sorted(&workspace.layout), ALL);
    workspace.set_visible(99, true, None, false, allow).unwrap();
    assert_eq!(group_members(&workspace.layout, 2), [2, 3, 99]);
    assert_eq!(end_dock(&workspace.layout).len(), 2);
    assert_eq!(group_members(&workspace.layout, 11), [11, 12]);

    let mut broken = default.clone();
    broken.layout.floating[0].rect = Rect::from_min_size(pos2(100.0, 100.0), vec2(-1.0, 400.0));
    broken.reconcile(registered, allow, &default);
    assert_eq!(broken.layout, default.layout);
}

#[test]
fn a_workspace_set_with_a_floating_group_round_trips_through_json() {
    let layout = Layout::new(Some(Node::tabs(vec![1u32, 2], 0)), vec![Floating::single(vec![3u32], 0, rect(1.0, 2.0, 300.0, 200.0)).with_id(5)]);
    let mut set = WorkspaceSet::new(vec![NamedWorkspace::new("essentials", "Essentials", Workspace::default())]);
    set.save("Mine", Workspace::new(layout), WorkspaceChrome::default()).unwrap();
    let saved = serde_json::to_string(&set).unwrap();
    let mut loaded: WorkspaceSet<u32> = serde_json::from_str(&saved).unwrap();
    assert!(loaded.builtin.is_empty(), "built-ins are code, not stored");
    loaded.builtin = set.builtin.clone();
    assert_eq!(loaded, set);
}

const BAR: f32 = 18.0;

fn monitor(name: &str, area: Rect, scale: f32) -> Monitor {
    Monitor::new(Some(name.into()), area, scale)
}

fn left() -> Rect {
    rect(0.0, 0.0, 1920.0, 1080.0)
}

fn right() -> Rect {
    rect(1920.0, 0.0, 2560.0, 1440.0)
}

#[test]
fn a_window_is_kept_where_its_bar_can_be_grabbed() {
    let monitors = [monitor("left", left(), 1.0), monitor("right", right(), 2.0)];
    for across in [rect(1800.0, 100.0, 400.0, 300.0), rect(-360.0, 1071.0, 400.0, 300.0)] {
        assert_eq!(kept_reachable(across, &monitors, BAR), across);
    }
    assert_eq!(kept_reachable(rect(-370.0, 100.0, 400.0, 300.0), &monitors, BAR), rect(0.0, 100.0, 400.0, 300.0));
    assert_eq!(kept_reachable(rect(500.0, 1075.0, 400.0, 300.0), &monitors, BAR), rect(500.0, 780.0, 400.0, 300.0));
    assert_eq!(kept_reachable(rect(9000.0, 100.0, 400.0, 300.0), &monitors, BAR), rect(4080.0, 100.0, 400.0, 300.0));
    // No monitor list: only made safe. A non-finite edge leaves that extent unknown, so it falls
    // back.
    assert_eq!(kept_reachable(rect(9000.0, f32::NAN, 400.0, 300.0), &[], BAR), rect(9000.0, 0.0, 400.0, 320.0));
}
