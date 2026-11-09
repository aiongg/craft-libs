//! Dock transactions, workspaces, arrangements, document windows and native placement.

use craft_behavior::dock::{
    Action, Arrangement, Column, DockEdge, DockError, DockPlacement, DockSide, DocumentGroup, DocumentWindows, Floating, Group, Layout,
    NamedWorkspace, NativeSpace, Node, PanelPlace, PanelState, Permissions, SplitSize, Target, Workspace, WorkspaceChrome, WorkspaceError,
    WorkspaceSet, arrange_rects, kept_reachable, node_at, restore_rect,
};
use craft_behavior::placement::cascade_rects;
use craft_behavior::{Axis, Monitor};
use emath::{Rect, Vec2, pos2, vec2};

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

fn members_of(groups: &[Group<u32>]) -> Vec<Vec<u32>> {
    groups.iter().map(|g| g.panels.clone()).collect()
}

#[test]
fn a_floating_destination_takes_only_floatable_panels_and_a_group_there_needs_splits() {
    let before = docked();
    // Panel 4 may move but not float; panel 6 (floating) takes no splits.
    let policy = |p: &u32| match p {
        4 => Permissions::ALL.floatable(false),
        6 => Permissions::ALL.accepts_splits(false),
        _ => Permissions::ALL,
    };
    let tab = DockPlacement::tab(None);
    let group = DockPlacement::group(true, None);
    for action in [
        Action::move_beside(4, 8, tab.clone()),
        Action::move_panel(4, Target::at(8, tab.clone())),
        Action::move_group(4, Target::at(8, tab.clone())),
        Action::move_group(11, Target::at(6, group.clone())),
    ] {
        let mut layout = before.clone();
        assert_eq!(layout.apply_with_permissions(action.clone(), policy), Err(DockError::NotAllowed), "{action:?}");
        assert_eq!(layout, before);
    }
    let mut layout = before.clone();
    layout.apply_with_permissions(Action::move_beside(5, 8, tab), policy).unwrap();
    let mut layout = before.clone();
    layout.apply_with_permissions(Action::move_group(11, Target::at(2, group)), policy).unwrap();
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
fn close_group_and_close_window_record_a_restorable_location_per_member() {
    let mut workspace = Workspace::new(docked());
    assert_eq!(workspace.apply(Action::close_group(3), false, allow).unwrap(), [2, 3]);
    assert_eq!(workspace.hidden.iter().map(|(p, _)| *p).collect::<Vec<_>>(), [2, 3]);
    assert_eq!(members_of(&end_dock(&workspace.layout)[0].groups), [vec![4]]);
    workspace.set_visible(2, true, None, false, allow).unwrap();
    workspace.set_visible(3, true, None, false, allow).unwrap();
    assert_eq!(members_of(&end_dock(&workspace.layout)[0].groups), [vec![2, 3], vec![4]]);

    assert_eq!(workspace.layout.closed_by(&Action::close_window(7)).unwrap(), [6, 7, 8]);
    assert_eq!(workspace.apply(Action::close_window(7), false, allow).unwrap(), [6, 7, 8]);
    assert!(workspace.layout.floating.is_empty());
    assert_eq!(workspace.hidden.len(), 3);
    for panel in [6, 7, 8] {
        workspace.set_visible(panel, true, None, false, allow).unwrap();
    }
    assert_eq!(workspace.layout.floating.len(), 1);
    assert_eq!(members_of(&workspace.layout.floating[0].groups()), [vec![6, 7], vec![8]]);
    assert_eq!(workspace.layout.floating[0].rect, window());

    workspace.apply(Action::close_others(12), false, allow).unwrap();
    assert_eq!(group_members(&workspace.layout, 12), [12]);
    workspace.set_visible(11, true, None, false, allow).unwrap();
    assert_eq!(group_members(&workspace.layout, 12), [11, 12]);
    assert!(workspace.hidden.is_empty());
    assert_eq!(sorted(&workspace.layout), ALL);

    let keep = |p: &u32| if *p == 4 { Permissions::ALL.closable(false) } else { Permissions::ALL };
    let before = workspace.clone();
    assert_eq!(workspace.apply(Action::close_group(4), false, keep), Err(DockError::NotAllowed));
    assert_eq!(workspace, before);
}

#[test]
fn a_locked_set_rejects_structure_but_allows_activate_open_minimize_resize_and_native_windows() {
    let mut workspace = Workspace::new(docked());
    workspace.apply(Action::close(12), false, allow).unwrap();
    let mut set = WorkspaceSet::<u32>::default();
    set.locked = true;
    let locked = set.locked;
    let before = workspace.clone();
    let spot = rect(0.0, 0.0, 100.0, 100.0);
    let refused = [
        Action::move_beside(2, 5, DockPlacement::tab(None)),
        Action::move_group(2, Target::float(spot)),
        Action::move_column(5, Target::new_dock(DockEdge::Start)),
        Action::move_window(6, Target::at(4, DockPlacement::group(true, None))),
        Action::float(4, spot),
        Action::move_floating(6, spot),
        Action::close(4),
        Action::close_group(4),
        Action::close_window(6),
    ];
    for action in refused {
        assert_eq!(workspace.apply(action.clone(), locked, allow), Err(DockError::NotAllowed), "{action:?}");
        assert_eq!(workspace, before);
    }
    assert_eq!(workspace.set_visible(4, false, None, locked, allow), Err(DockError::NotAllowed));
    let allowed = [
        Action::activate(2),
        Action::open(12, Some(11)),
        Action::set_minimized(4, true),
        Action::set_iconic(5, true),
        Action::resize_column(5, 160.0),
        Action::resize_group(4, Some(120.0)),
        Action::resize_iconic(5, Some(60.0)),
        Action::resize_split(Vec::new(), SplitSize::FixedSecond(320.0)),
    ];
    workspace.apply_all(allowed, locked, allow).unwrap();
    assert!(workspace.hidden.is_empty());
    assert_eq!(workspace.layout.panel_state(&12), PanelState::Front);
    assert_eq!(workspace.layout.panel_state(&4), PanelState::Open);
    assert_eq!(workspace.layout.panel_state(&2), PanelState::Front);
    // Opening at a chosen place works too; moving a shown panel there does not.
    workspace.apply(Action::open_at(13, 11, DockPlacement::Split(DockSide::Bottom)), locked, allow).unwrap();
    assert!(workspace.layout.contains(&13));
    assert_eq!(workspace.apply(Action::open_at(2, 11, DockPlacement::Split(DockSide::Bottom)), locked, allow), Err(DockError::NotAllowed));

    for native in [rect(0.0, 0.0, 300.0, 400.0), rect(40.0, 30.0, 300.0, 400.0)] {
        workspace.apply(Action::set_native(6, Some(native)), locked, allow).unwrap();
        assert_eq!(workspace.layout.floating[0].native, Some(native));
    }
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

fn named(id: &str, name: &str) -> NamedWorkspace<&'static str> {
    NamedWorkspace::new(id, name, Workspace::default())
}

fn set() -> WorkspaceSet<&'static str> {
    WorkspaceSet::new(vec![named("essentials", "Essentials"), named("painting", "Painting")])
}

fn edited() -> Workspace<&'static str> {
    Workspace::new(Layout::new(Some(Node::tabs(vec!["layers"], 0)), Vec::new()))
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

#[test]
fn a_set_file_of_format_version_2_loads_with_its_lock_and_saved_layout_in_the_set() {
    let kept = Layout::new(Some(Node::tabs(vec![1u32], 0)), Vec::new());
    let v2 = |saved: Option<&Layout<u32>>, locked: bool| {
        let mut file = serde_json::to_value(Workspace::<u32>::default()).unwrap();
        file["version"] = 2.into();
        file["saved"] = serde_json::to_value(saved).unwrap();
        file["locked"] = locked.into();
        file
    };
    let old = serde_json::json!({
        "saved": [{
            "id": "mine",
            "name": "Mine",
            "layout": v2(Some(&kept), true),
            "chrome": WorkspaceChrome::default(),
            "keeps_shortcuts": false,
            "keeps_menus": false
        }],
        "current": "mine",
        "locked": false,
        "edited": { "mine": v2(None, false) }
    });
    let loaded: WorkspaceSet<u32> = serde_json::from_value(old).unwrap();
    assert!(loaded.locked, "a lock on a stored workspace locks the set");
    assert_eq!(loaded.current, "mine");
    assert_eq!(loaded.saved[0].layout.layout, kept, "the saved arrangement is the stored layout Reset returns to");
    loaded.saved[0].layout.validate().unwrap();
}

#[test]
fn saved_names_ignore_case_and_new_workspaces_show_every_bar() {
    let mut set = set();
    assert_eq!(set.save("PAINTING", Workspace::default(), WorkspaceChrome::default()), Err(WorkspaceError::NameTaken));
    let id = set.save("My Layout", edited(), WorkspaceChrome::default()).unwrap();
    assert_eq!(set.save("my layout", Workspace::default(), WorkspaceChrome::default()), Ok(id));
    assert_eq!(set.saved.len(), 1, "the same name in another case replaces the saved workspace");
    let chrome = &named("a", "A").chrome;
    assert!(chrome.menu_bar && chrome.tool_rail && chrome.options_bar && chrome.status_bar);
}

#[test]
fn n_up_cells_split_the_area_and_cascade_steps_then_wraps() {
    let area = Rect::from_min_max(pos2(10.0, 20.0), pos2(210.0, 220.0));
    let two = arrange_rects(Arrangement::TwoUpVertical, area, 5).unwrap();
    assert_eq!(two, [Rect::from_min_max(pos2(10.0, 20.0), pos2(110.0, 220.0)), Rect::from_min_max(pos2(110.0, 20.0), pos2(210.0, 220.0))]);
    let stacked = arrange_rects(Arrangement::ThreeUpStacked, area, 3).unwrap();
    assert_eq!(stacked[2], Rect::from_min_max(pos2(110.0, 120.0), pos2(210.0, 220.0)));
    assert_eq!(arrange_rects(Arrangement::TileAllHorizontally, area, 4).unwrap()[3].min, pos2(10.0, 170.0));
    assert_eq!(arrange_rects(Arrangement::Tile, area, 5).unwrap().len(), 5);
    assert!(arrange_rects(Arrangement::FourUp, area, 1).is_none());
    assert!(arrange_rects(Arrangement::Cascade, area, 4).is_none());

    let rects = cascade_rects(area, vec2(120.0, 120.0), 4, 28.0);
    assert_eq!([rects[0].min, rects[1].min, rects[2].min], [pos2(10.0, 20.0), pos2(38.0, 48.0), pos2(66.0, 76.0)]);
    // 94 + 120 > 210: both axes start over at the edge.
    assert_eq!(rects[3].min, pos2(10.0, 20.0));
}

fn windows(documents: &[&'static str]) -> DocumentWindows<&'static str> {
    let mut windows = DocumentWindows::default();
    let mut group = DocumentGroup::new(1, documents.to_vec());
    group.shown = documents.first().copied();
    windows.docked = vec![group];
    windows
}

#[test]
fn floating_and_consolidating_document_windows_returns_every_document_to_one_group() {
    let spot = rect(40.0, 40.0, 300.0, 200.0);
    let mut w = windows(&["a", "b", "c"]);
    let id = w.float(&"b", spot).unwrap();
    assert_eq!(w.docked[0].documents, ["a", "c"]);
    assert_eq!(w.floating[0].0.id, id);
    assert_ne!(id, w.docked[0].id);
    assert!(w.float(&"zzz", spot).is_none());
    w.new_window_for(&"a", spot);
    w.float_all(rect(0.0, 0.0, 800.0, 600.0), vec2(300.0, 200.0), 28.0);
    assert!(w.docked.is_empty());
    assert_eq!(w.floating.len(), 3, "a document open twice floats once");
    assert_eq!(w.floating[1].1.min, pos2(28.0, 28.0));
    w.consolidate(Some(&"c"));
    assert!(w.floating.is_empty());
    assert_eq!(w.docked.len(), 1);
    assert_eq!(w.docked[0].documents, ["a", "c", "b"]);
    assert_eq!(w.docked[0].shown, Some("c"));
}

#[test]
fn splitting_into_n_groups_keeps_the_order_and_gives_the_first_groups_the_extra_documents() {
    let mut w = windows(&["a", "b", "c"]);
    w.docked[0].shown = Some("c");
    w.split_into(2);
    let groups: Vec<_> = w.docked.iter().map(|g| (g.documents.clone(), g.shown)).collect();
    assert_eq!(groups, [(vec!["a", "b"], Some("a")), (vec!["c"], Some("c"))]);
    assert_ne!(w.docked[0].id, w.docked[1].id);
}

#[test]
fn floating_document_groups_become_dock_actions() {
    let spot = rect(40.0, 40.0, 300.0, 200.0);
    let mut w = windows(&["a", "b"]);
    let id = w.float(&"b", spot).unwrap();
    let mut layout = Layout::<u64>::default();
    assert_eq!(w.to_actions(&layout, |g| g), [Action::open(id, None), Action::float(id, spot)]);

    layout.apply(Action::open(id, None)).unwrap();
    layout.apply(Action::float(id, rect(0.0, 0.0, 300.0, 200.0))).unwrap();
    layout.apply(Action::open(w.docked[0].id, None)).unwrap();
    assert_eq!(w.to_actions(&layout, |g| g), [Action::move_floating(id, spot), Action::close(w.docked[0].id)]);
}

const BAR: f32 = 18.0;

fn monitor(name: &str, area: Rect, scale: f32) -> Monitor {
    Monitor::new(Some(name.into()), area, scale)
}

fn space(monitors: &[Monitor], main: Option<Rect>, monitor_size: Option<Vec2>) -> NativeSpace<'_> {
    let mut space = NativeSpace::new(monitors, BAR);
    space.main = main;
    space.monitor_size = monitor_size;
    space
}

fn main_at(x: f32, y: f32) -> Option<Rect> {
    Some(rect(x, y, 1200.0, 800.0))
}

fn left() -> Rect {
    rect(0.0, 0.0, 1920.0, 1080.0)
}

fn right() -> Rect {
    rect(1920.0, 0.0, 2560.0, 1440.0)
}

#[test]
fn a_saved_native_window_keeps_its_connected_monitor_and_returns_when_its_bar_is_lost() {
    let monitors = [monitor("left", left(), 1.0), monitor("right", right(), 2.0)];
    let at = |saved: Rect, on: &Monitor| restore_rect(saved, Some(on), space(&monitors, main_at(100.0, 100.0), None));
    let saved = rect(2400.0, 300.0, 400.0, 500.0);
    assert_eq!(at(saved, &monitors[1]), saved);
    // Renamed, but the same rect.
    assert_eq!(at(saved, &monitor("DP-2", right(), 2.0)), saved);
    // Partly off its monitor, as the user left it.
    let edge = rect(4300.0, 300.0, 400.0, 500.0);
    assert_eq!(at(edge, &monitors[1]), edge);
    // Wholly off it, or with only its body on it: back onto it.
    assert_eq!(at(rect(5000.0, 300.0, 400.0, 500.0), &monitors[1]), rect(4080.0, 300.0, 400.0, 500.0));
    assert_eq!(at(rect(2400.0, -10.0, 400.0, 500.0), &monitors[1]), rect(2400.0, 0.0, 400.0, 500.0));
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

#[test]
fn a_missing_monitor_moves_the_window_to_the_main_window_monitor_and_a_resized_one_in_proportion() {
    let gone = monitor("right", right(), 2.0);
    let monitors = [monitor("left", left(), 1.0)];
    let at = |saved: Rect| restore_rect(saved, Some(&gone), space(&monitors, main_at(100.0, 100.0), None));
    // Same offset from the monitor's origin; clamped wholly on screen, shrunk if larger than the monitor.
    assert_eq!(at(rect(2020.0, 50.0, 400.0, 500.0)), rect(100.0, 50.0, 400.0, 500.0));
    assert_eq!(at(rect(4000.0, 1200.0, 400.0, 500.0)), rect(1520.0, 580.0, 400.0, 500.0));
    assert_eq!(at(rect(1920.0, 0.0, 3000.0, 1300.0)), left());

    let old = monitor("left", left(), 1.0);
    let smaller = [monitor("left", rect(0.0, 0.0, 960.0, 540.0), 2.0)];
    let scaled = |saved: Rect| restore_rect(saved, Some(&old), space(&smaller, main_at(0.0, 0.0), None));
    assert_eq!(scaled(rect(960.0, 540.0, 200.0, 200.0)), rect(480.0, 270.0, 200.0, 200.0));
    assert_eq!(scaled(rect(1800.0, 1000.0, 200.0, 200.0)), rect(760.0, 340.0, 200.0, 200.0));
}

#[test]
fn without_a_monitor_list_the_window_goes_on_the_main_window_monitor_by_size() {
    let size = Some(vec2(1920.0, 1080.0));
    let saved = rect(2500.0, 300.0, 400.0, 500.0);
    assert_eq!(restore_rect(saved, None, space(&[], main_at(100.0, 100.0), size)), rect(1520.0, 300.0, 400.0, 500.0));
    assert_eq!(restore_rect(saved, None, space(&[], main_at(2000.0, 100.0), size)), saved);
    assert_eq!(restore_rect(saved, None, space(&[], main_at(-1500.0, 100.0), size)), rect(-400.0, 300.0, 400.0, 500.0));
    // Nothing known: the saved rect, made safe.
    let far = Rect::from_min_size(pos2(50.0, 1e6), vec2(10.0, 400.0));
    assert_eq!(restore_rect(far, None, space(&[], None, None)), rect(50.0, 100_000.0, 160.0, 400.0));
    let hostile = Rect::from_min_size(pos2(f32::NAN, 1e6), vec2(10.0, 400.0));
    assert_eq!(restore_rect(hostile, None, space(&[], None, None)), rect(0.0, 100_000.0, 420.0, 400.0));
}

#[test]
fn clamping_onto_a_fractional_origin_does_not_panic() {
    let area = rect(910.6667, 0.333_33, 1280.0, 720.0);
    let monitors = [Monitor::new(None, area, 1.0)];
    let placed = restore_rect(rect(0.0, 2000.0, 1280.0, 900.0), None, space(&monitors, None, None));
    assert_eq!(placed.size(), vec2(1280.0, 720.0));
    assert!((placed.min - area.min).abs().max_elem() < 0.01, "{placed:?}");
    let placed = restore_rect(rect(5000.0, 0.0, 2000.0, 100.0), None, space(&monitors, None, None));
    assert!((placed.min.x - area.min.x).abs() < 0.01, "{placed:?}");
}
