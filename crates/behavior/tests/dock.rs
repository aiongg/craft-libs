//! The dock model: validation, restore to the old place, permissions and the persisted shapes.

use craft_behavior::dock::{
    Column, DockError, Floating, Group, Layout, Location, Monitor, Node, PanelState, Permissions, Placement, Side, SplitAxis, SplitSize,
};
use serde_json::json;

const WINDOW: [f32; 4] = [100.0, 100.0, 300.0, 400.0];

fn g(panels: &[u32], active: usize) -> Group<u32> {
    Group::new(panels.to_vec(), active)
}

fn tabs(panels: &[u32], active: usize) -> Node<u32> {
    Node::tabs(panels.to_vec(), active)
}

fn hsplit(size: SplitSize, first: Node<u32>, second: Node<u32>) -> Node<u32> {
    Node::split(SplitAxis::Horizontal, size, first, second)
}

/// Canvas 1 above frame tabs [11, 12]; an end dock with column A ([2, 3], [4]) and column B
/// ([5]); a floating window with groups [6, 7] and [8].
fn docked() -> Layout<u32> {
    let middle = Node::split(SplitAxis::Vertical, SplitSize::Ratio(0.7), tabs(&[1], 0), tabs(&[11, 12], 1));
    let dock = Node::dock(Side::End, vec![Column::new(vec![g(&[2, 3], 1), g(&[4], 0)], 180.0), Column::new(vec![g(&[5], 0)], 120.0)]);
    Layout::new(Some(hsplit(SplitSize::FixedSecond(300.0), middle, dock)), vec![Floating::stack(vec![g(&[6, 7], 1), g(&[8], 0)], WINDOW).unwrap()])
}

fn with_column(column: Column<u32>) -> Layout<u32> {
    Layout::new(Some(Node::dock(Side::End, vec![column])), Vec::new())
}

#[test]
fn validation_rejects_two_docks_per_side_nested_docks_empty_columns_and_bad_widths() {
    docked().validate().unwrap();
    let empty_dock = |side| Node::dock(side, Vec::new());
    Layout::new(Some(hsplit(SplitSize::FixedFirst(10.0), empty_dock(Side::Start), tabs(&[1], 0))), Vec::new()).validate().unwrap();

    let two = hsplit(SplitSize::Ratio(0.5), empty_dock(Side::Start), empty_dock(Side::Start));
    let nested = Node::split(SplitAxis::Vertical, SplitSize::Ratio(0.5), empty_dock(Side::Start), tabs(&[1], 0));
    let wrong_side = hsplit(SplitSize::Ratio(0.5), empty_dock(Side::End), tabs(&[1], 0));
    for root in [two, nested, wrong_side] {
        assert_eq!(Layout::new(Some(root), Vec::new()).validate(), Err(DockError::InvalidPlacement));
    }

    let mut nan_height = g(&[2], 0);
    nan_height.height = Some(f32::NAN);
    let mut nan_iconic = Column::new(vec![g(&[2], 0)], 100.0);
    nan_iconic.iconic_width = Some(f32::NAN);
    let bad = [
        Column::new(Vec::new(), 100.0),
        Column::new(vec![g(&[], 0)], 100.0),
        Column::new(vec![g(&[2], 1)], 100.0),
        Column::new(vec![nan_height], 100.0),
        Column::new(vec![g(&[2], 0)], 0.0),
        Column::new(vec![g(&[2], 0)], -5.0),
        Column::new(vec![g(&[2], 0)], f32::INFINITY),
        Column::new(vec![g(&[2], 0)], 2.0e6),
        nan_iconic,
    ];
    for column in bad {
        assert!(with_column(column.clone()).validate().is_err(), "{column:?}");
    }
}

#[test]
fn validation_rejects_bad_floating_windows_and_duplicate_panels() {
    let mut duplicate = docked();
    duplicate.floating[0].root = tabs(&[6, 7, 2], 0);
    assert_eq!(duplicate.validate(), Err(DockError::DuplicatePanel));
    let mut empty_window = docked();
    empty_window.floating[0].root = tabs(&[], 0);
    assert_eq!(empty_window.validate(), Err(DockError::InvalidGeometry));
    let mut docked_window = docked();
    docked_window.floating[0].root = Node::dock(Side::End, vec![Column::new(vec![g(&[6, 7], 0)], 100.0)]);
    assert_eq!(docked_window.validate(), Err(DockError::InvalidPlacement));
    let mut native = docked();
    native.floating[0].native = Some([0.0, 0.0, -1.0, 10.0]);
    assert_eq!(native.validate(), Err(DockError::InvalidGeometry));
    let mut monitor = docked();
    monitor.floating[0].monitor = Some(Monitor::new(None, [0.0, 0.0, 1920.0, 1080.0], f32::NAN));
    assert_eq!(monitor.validate(), Err(DockError::InvalidGeometry));
    let mut monitor_size = docked();
    monitor_size.floating[0].monitor_size = Some([f32::NAN, 1080.0]);
    assert_eq!(monitor_size.validate(), Err(DockError::InvalidGeometry));
}

/// The layout of `restore_returns_each_panel_to_the_place_it_was_in` with `skip` absent.
fn without(skip: Option<u32>) -> Layout<u32> {
    let group = |panels: &[u32], active: usize| {
        let kept: Vec<u32> = panels.iter().copied().filter(|p| Some(*p) != skip).collect();
        (!kept.is_empty()).then(|| {
            let active = active.min(kept.len() - 1);
            Group::new(kept, active)
        })
    };
    let mut minimized = group(&[4], 0);
    if let Some(group) = &mut minimized {
        group.minimized = true;
        group.height = Some(150.0);
    }
    let mut tall = group(&[7], 0);
    if let Some(group) = &mut tall {
        group.height = Some(90.0);
    }
    let a: Vec<Group<u32>> = [group(&[2, 3], 1), minimized, group(&[9], 0)].into_iter().flatten().collect();
    let mut b = Column::new(group(&[5], 0).into_iter().collect(), 120.0);
    b.iconic = true;
    b.iconic_width = Some(48.0);
    let columns: Vec<Column<u32>> = [Column::new(a, 180.0), b].into_iter().filter(|c| !c.groups.is_empty()).collect();
    let end = Node::dock(Side::End, columns);
    let windows: Vec<Group<u32>> = [group(&[6], 0), tall].into_iter().flatten().collect();
    Layout::new(Some(hsplit(SplitSize::FixedSecond(300.0), tabs(&[1], 0), end)), vec![Floating::stack(windows, [10.0, 10.0, 200.0, 300.0]).unwrap()])
}

#[test]
fn restore_returns_each_panel_to_the_place_it_was_in() {
    let original = without(None);
    original.validate().unwrap();
    // 4: the seam between [2, 3] and [9]; 9: below [4]; 5: a new column after A; 3: a tab;
    // 7: below a floating group.
    for panel in [4, 9, 5, 3, 7] {
        let location = original.location(&panel).unwrap();
        let location: Location<u32> = serde_json::from_str(&serde_json::to_string(&location).unwrap()).unwrap();
        let mut layout = without(Some(panel));
        layout.validate().unwrap();
        layout.restore(panel, &location).unwrap_or_else(|e| panic!("{panel}: {e} {location:?}"));
        assert_eq!(layout, original, "{panel}: {location:?}");
    }
}

#[test]
fn restore_refuses_a_present_panel_and_a_missing_anchor_and_changes_nothing() {
    let original = without(None);
    let location = original.location(&3).unwrap();
    let mut layout = original.clone();
    assert!(layout.restore(3, &location).is_err());
    assert_eq!(layout, original);

    let mut layout = without(Some(3));
    let before = layout.clone();
    let orphan = Location::new(Some(99), Placement::tab(None));
    assert!(layout.restore(3, &orphan).is_err());
    assert_eq!(layout, before);
}

#[test]
fn panel_state_tells_hidden_open_and_front() {
    let layout = docked();
    assert_eq!(layout.panel_state(&3), PanelState::Front);
    assert_eq!(layout.panel_state(&2), PanelState::Open);
    assert_eq!(layout.panel_state(&99), PanelState::Hidden);
}

#[test]
fn move_targets_offer_other_groups_that_accept_tabs() {
    let layout = docked();
    let all = |_: &u32| Permissions::default();
    let mut targets: Vec<u32> = layout.move_targets(&3, all).into_iter().copied().collect();
    targets.sort_unstable();
    assert_eq!(targets, [1, 4, 5, 6, 7, 8, 11, 12]);

    let no_tabs = |p: &u32| if *p == 1 { Permissions::PROTECTED } else { Permissions::default() };
    assert!(!layout.move_targets(&3, no_tabs).contains(&&1));
}

#[test]
fn step_group_never_passes_a_protected_group() {
    let column = Column::new(vec![g(&[2], 0), g(&[9], 0), g(&[4], 0)], 180.0);
    let layout = with_column(column);
    let all = |_: &u32| Permissions::default();
    assert!(layout.can_step_group(&2, false, all));
    assert!(!layout.can_step_group(&2, true, all));
    let permissions = |p: &u32| if *p == 9 { Permissions::PROTECTED } else { Permissions::default() };
    for (panel, up) in [(2, false), (4, true), (9, true), (9, false)] {
        assert!(!layout.can_step_group(&panel, up, permissions), "{panel} {up}");
    }
    let immovable = |p: &u32| if *p == 2 { Permissions::new(false, true, true, true, true) } else { Permissions::default() };
    assert!(!layout.can_step_group(&2, false, immovable));
    assert!(!layout.can_step_group(&9, true, immovable));
}

#[test]
fn a_saved_stack_of_groups_loads_as_a_vertical_split_by_their_heights() {
    let saved = json!({
        "groups": [{"panels": [4], "active": 0, "height": 100.0}, {"panels": [5, 6], "active": 1, "height": 300.0}],
        "rect": [1.0, 2.0, 300.0, 400.0],
    });
    let window: Floating<u32> = serde_json::from_value(saved).unwrap();
    let expected = Node::split(SplitAxis::Vertical, SplitSize::Ratio(0.25), tabs(&[4], 0), tabs(&[5, 6], 1));
    assert_eq!((window.root, window.minimized), (expected, false));
    let all_minimized = json!({"groups": [{"panels": [4], "active": 0, "minimized": true}], "rect": [1.0, 2.0, 300.0, 400.0]});
    let window: Floating<u32> = serde_json::from_value(all_minimized).unwrap();
    assert_eq!((window.root, window.minimized), (tabs(&[4], 0), true));
}

#[test]
fn a_single_plain_floating_window_writes_the_v1_shape_and_others_round_trip() {
    let rect = [1.0, 2.0, 300.0, 200.0];
    let plain = Floating::single(vec![4u32, 5], 1, rect);
    assert_eq!(serde_json::to_value(&plain).unwrap(), json!({"id": plain.id, "panels": [4, 5], "active": 1, "rect": rect}));
    let mut minimized = Floating::single(vec![4u32], 0, rect);
    minimized.minimized = true;
    let stacked = Floating::stack(vec![g(&[4], 0), g(&[5], 0)], rect).unwrap();
    let mut native = Floating::single(vec![4u32], 0, rect);
    native.native = Some(rect);
    for window in [minimized, stacked, native] {
        let value = serde_json::to_value(&window).unwrap();
        assert!(value.get("root").is_some() && value.get("panels").is_none(), "{value}");
        assert_eq!(value["id"], json!(window.id), "{value}");
        assert_eq!(serde_json::from_value::<Floating<u32>>(value).unwrap(), window);
    }
    assert!(serde_json::from_value::<Floating<u32>>(json!({"rect": rect})).is_err());
}

#[test]
fn windows_keep_their_ids_through_a_save() {
    let rect = [1.0, 2.0, 300.0, 200.0];
    let layout = Layout::new(
        None,
        vec![Floating::single(vec![4u32], 0, rect).with_id(7), Floating::stack(vec![g(&[5], 0), g(&[6], 0)], rect).unwrap().with_id(8)],
    );
    let saved = serde_json::to_string(&layout).unwrap();
    let loaded: Layout<u32> = serde_json::from_str(&saved).unwrap();
    assert_eq!(loaded, layout);
    assert_eq!(loaded.floating.iter().map(|w| w.id).collect::<Vec<_>>(), [7, 8]);
}

#[test]
fn a_layout_saved_by_the_legacy_code_loads_and_writes_back_unchanged() {
    let saved = r#"{
        "root": {"Split": {"axis": "Horizontal", "size": {"FixedSecond": 300.0},
            "first": {"Tabs": {"panels": [1, 2], "active": 1}},
            "second": {"Dock": {"side": "End", "columns": [
                {"groups": [{"panels": [3], "active": 0, "minimized": false, "height": null}], "width": 180.0, "iconic": false, "iconic_width": null}
            ]}}}},
        "floating": [{"id": 5, "panels": [4], "active": 0, "rect": [1.0, 2.0, 300.0, 200.0]}]
    }"#;
    let layout: Layout<u32> = serde_json::from_str(saved).unwrap();
    layout.validate().unwrap();
    let expected = Layout::new(
        Some(hsplit(SplitSize::FixedSecond(300.0), tabs(&[1, 2], 1), Node::dock(Side::End, vec![Column::new(vec![g(&[3], 0)], 180.0)]))),
        vec![Floating::single(vec![4], 0, [1.0, 2.0, 300.0, 200.0]).with_id(5)],
    );
    assert_eq!(layout, expected);
    assert_eq!(serde_json::to_value(&layout).unwrap(), serde_json::from_str::<serde_json::Value>(saved).unwrap());
}
