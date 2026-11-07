//! The preset library's rules, the open-group state and the nesting rule, the panel menu and the file entries.

use craft_ui::contract::{OpenChange, ReorderMove};
use craft_ui::patterns::presets::{
    DefaultOpen, GroupPath, PresetChange, PresetCommands, PresetError, PresetEvent, PresetFileEvent, PresetFileFailure, PresetFileRequest,
    PresetFiles, PresetId, PresetKey, PresetLibrary, PresetMenuAction, PresetPanelMenu, PresetRow, PresetState, PresetView, children_end,
    nest_allowed,
};
use craft_ui::testing::trial;
use craft_ui::tile_grid::TileZoom;
use std::collections::HashSet;

type Key = PresetKey<PresetId>;

/// Root: Loose; A: Red, Green, folder Inner (Blue); B: Cyan; nesting 1.
fn sample() -> (PresetLibrary<u32>, [PresetId; 5]) {
    let mut lib = PresetLibrary::new().with_nesting(1);
    let root = GroupPath::root();
    let loose = lib.add(&root, "Loose", 0).unwrap();
    let a = lib.new_group(&root, "A").unwrap();
    let red = lib.add(&a, "Red", 1).unwrap();
    let green = lib.add(&a, "Green", 2).unwrap();
    let inner = lib.new_group(&a, "Inner").unwrap();
    let blue = lib.add(&inner, "Blue", 3).unwrap();
    let b = lib.new_group(&root, "B").unwrap();
    let cyan = lib.add(&b, "Cyan", 4).unwrap();
    (lib, [loose, red, green, blue, cyan])
}

/// The display position `target` has once `moved` and the rows inside them are taken out: the `to` a drop before it has.
fn before(lib: &PresetLibrary<u32>, moved: &[Key], target: &Key) -> usize {
    let rows = lib.rows();
    let mut gone = vec![false; rows.len()];
    for key in moved {
        let at = rows.iter().position(|row| row.key == *key).unwrap();
        gone[at..children_end(&rows, at)].fill(true);
    }
    rows.iter().zip(&gone).filter(|(_, gone)| !**gone).position(|(row, _)| row.key == *target).unwrap()
}

fn names(lib: &PresetLibrary<u32>, group: &GroupPath) -> Vec<String> {
    lib.group(group).unwrap().items.iter().map(|item| item.name.clone()).collect()
}

fn group_labels(lib: &PresetLibrary<u32>) -> Vec<String> {
    lib.rows().iter().filter(|row| row.is_group()).map(|row| row.label.clone()).collect()
}

fn moved_ids(changes: &[PresetChange]) -> Vec<PresetId> {
    changes.iter().filter_map(|change| if let PresetChange::Moved { id, .. } = change { Some(*id) } else { None }).collect()
}

fn reordered(changes: &[PresetChange]) -> Vec<GroupPath> {
    changes.iter().filter_map(|change| if let PresetChange::Reordered(path) = change { Some(path.clone()) } else { None }).collect()
}

/// A drop before a row moves presets next to it, across groups, keeping their display order, and the changes say what moved.
#[trial]
fn reordering_presets_moves_them_before_the_target_row() {
    let (mut lib, [_loose, red, green, _blue, cyan]) = sample();
    let a = GroupPath::new(["A"]);
    let b = GroupPath::new(["B"]);

    let keys = [PresetKey::Item(green)];
    let to = before(&lib, &keys, &PresetKey::Item(red));
    let changes = lib.reorder(&keys, to).unwrap();
    assert_eq!(reordered(&changes), [a.clone()]);
    assert_eq!(names(&lib, &a), ["Green", "Red"]);

    let keys = [PresetKey::Item(green), PresetKey::Item(red)];
    let to = before(&lib, &keys, &PresetKey::Item(cyan));
    let changes = lib.reorder(&keys, to).unwrap();
    assert_eq!(moved_ids(&changes), [green, red]);
    assert_eq!(reordered(&changes), [b.clone()]);
    assert_eq!(names(&lib, &b), ["Green", "Red", "Cyan"]);
}

/// A group dropped before another group becomes its sibling and takes its presets along; a drop past the end lands after
/// the last row, inside that row's group.
#[trial]
fn reordering_a_group_changes_its_parent_and_keeps_its_presets() {
    let (mut lib, [_loose, _red, _green, blue, _cyan]) = sample();
    let inner = PresetKey::group(["A", "Inner"]);
    let to = before(&lib, std::slice::from_ref(&inner), &PresetKey::group(["B"]));
    let changes = lib.reorder(std::slice::from_ref(&inner), to).unwrap();
    assert!(matches!(&changes[0], PresetChange::GroupMoved { from, to, .. } if from.names() == ["A", "Inner"] && to.names() == ["Inner"]));
    assert_eq!(lib.group_of(blue), Some(GroupPath::new(["Inner"])));
    assert_eq!(group_labels(&lib), ["A", "Inner", "B"]);

    let a = PresetKey::group(["A"]);
    let end = lib.rows().len();
    lib.reorder(std::slice::from_ref(&a), end).unwrap();
    assert!(lib.group(&GroupPath::new(["B", "A"])).is_some(), "after Cyan, which is in B");
}

/// A move that names its parent group lands inside it: before its child at `to`, else after its last child, which
/// the flat position alone cannot say for an empty group or the end of a group.
#[trial]
fn a_move_into_a_group_lands_in_that_group() {
    let (mut lib, [loose, red, _green, _blue, cyan]) = sample();
    let empty = lib.new_group(&GroupPath::root(), "Empty").unwrap();
    let b = GroupPath::new(["B"]);

    let end = lib.rows().len();
    let into_b = ReorderMove::new(PresetKey::Item(loose), vec![PresetKey::Item(loose)], end, false).in_group(Some(PresetKey::Group(b.clone())));
    lib.reorder_move(&into_b).unwrap();
    assert_eq!(lib.group_of(loose), Some(b.clone()));
    assert_eq!(lib.group(&b).unwrap().items.iter().map(|item| item.id).collect::<Vec<_>>(), [cyan, loose], "after the last child");

    let into_empty = ReorderMove::new(PresetKey::Item(red), vec![PresetKey::Item(red)], 0, false).in_group(Some(PresetKey::Group(empty.clone())));
    lib.reorder_move(&into_empty).unwrap();
    assert_eq!(lib.group_of(red), Some(empty), "the empty group takes it wherever `to` points");
}

/// A group moved past the library's nesting, or onto a sibling's name, is refused and the library stays as it was.
#[trial]
fn reordering_refuses_nesting_past_the_depth_and_a_taken_name() {
    let (mut lib, [_loose, red, _green, blue, _cyan]) = sample();
    lib.new_group(&GroupPath::new(["B"]), "Sub").unwrap();
    let snapshot = lib.clone();
    let group_b = [PresetKey::group(["B"])];
    let to = before(&lib, &group_b, &PresetKey::Item(blue));
    assert_eq!(lib.reorder(&group_b, to), Err(PresetError::TooDeep), "B carries a folder, so it would reach depth 4");
    assert_eq!(lib, snapshot);

    lib.new_group(&GroupPath::new(["A"]), "Sub").unwrap();
    let snapshot = lib.clone();
    let sub = [PresetKey::group(["B", "Sub"])];
    let to = before(&lib, &sub, &PresetKey::Item(red));
    assert_eq!(lib.reorder(&sub, to), Err(PresetError::NameTaken));
    assert_eq!(lib, snapshot);
}

/// A browser's Delete event changes the library and returns its change; a rename to a taken name changes nothing.
#[trial]
fn the_library_applies_browser_events() {
    let (mut lib, [_loose, red, green, _blue, cyan]) = sample();
    assert_eq!(lib.apply(&PresetEvent::Delete(PresetKey::Item(cyan))), [PresetChange::Deleted(cyan)]);
    let deleted = lib.apply(&PresetEvent::Delete(PresetKey::group(["A", "Inner"])));
    assert!(matches!(&deleted[..], [PresetChange::GroupDeleted { items, .. }] if items.len() == 1));
    assert_eq!(lib.rename(red, "green"), Err(PresetError::NameTaken));
    assert!(lib.get(green).is_some());
}

fn rows() -> Vec<PresetRow<&'static str>> {
    vec![
        PresetRow::group("a", "A", 0),
        PresetRow::item("a1", "A1", 1),
        PresetRow::group("a-sub", "Sub", 1),
        PresetRow::item("a2", "A2", 2),
        PresetRow::group("b", "B", 0),
        PresetRow::group("b-sub", "Sub", 1),
    ]
}

#[trial]
fn a_group_nests_only_within_the_depth_and_never_into_itself() {
    let rows = rows();
    assert_eq!(children_end(&rows, 0), 4);
    assert_eq!(children_end(&rows, 4), 6);
    assert_eq!(children_end(&rows, 1), 2, "an item has none");
    assert!(!nest_allowed(&rows, 4, 0, 0), "no nesting at 0");
    assert!(nest_allowed(&rows, 5, 0, 1), "a folder moves to another group");
    assert!(!nest_allowed(&rows, 4, 5, 9), "not into its own folder");
    assert!(!nest_allowed(&rows, 0, 4, 1), "A carries a folder, so it would reach depth 2");
    assert!(nest_allowed(&rows, 0, 4, 2));
    assert!(!nest_allowed(&rows, 1, 4, 9), "an item is not nested");
}

#[trial]
fn groups_are_open_by_default_mode_flipped_by_the_toggled_set() {
    let rows = vec![PresetRow::group("a", "A", 0), PresetRow::item("a1", "A1", 1), PresetRow::group("b", "B", 0), PresetRow::item("b1", "B1", 1)];
    let mut state = PresetState::new();
    let open = |s: &PresetState<&str>, mode| (s.is_open(&rows, 0, mode), s.is_open(&rows, 2, mode));
    assert_eq!(open(&state, DefaultOpen::All), (true, true));
    assert_eq!(open(&state, DefaultOpen::First), (true, false));
    assert_eq!(open(&state, DefaultOpen::None), (false, false));
    state.toggle(&rows, 2, DefaultOpen::First);
    state.toggle(&rows, 0, DefaultOpen::First);
    assert_eq!(open(&state, DefaultOpen::First), (false, true));
    state.toggle(&rows, 0, DefaultOpen::First);
    assert_eq!(state.toggled.len(), 1, "back to the default leaves no entry");

    let all = |open| OpenChange::new("a", open).all();
    state.apply_open(&rows, &all(false), DefaultOpen::First);
    assert!(!open(&state, DefaultOpen::First).0 && !open(&state, DefaultOpen::First).1);
    let toggled = state.toggled.clone();
    state.apply_open(&rows, &OpenChange::new("a1", false), DefaultOpen::All);
    assert_eq!(state.toggled, toggled, "an item has no open state");
}

#[trial]
fn filtering_opens_every_group_and_stale_keys_and_zoom_are_cleaned_up() {
    let rows = rows();
    let mut state = PresetState::<&str>::new();
    state.apply_open(&rows, &OpenChange::new("a", false), DefaultOpen::All);
    state.filter = "  ".into();
    assert!(!state.filtering() && !state.is_open(&rows, 0, DefaultOpen::All));
    state.filter = "b".into();
    assert!(state.is_open(&rows, 0, DefaultOpen::All));

    state.toggled.extend(["gone", "a1"]);
    state.retain_groups(&rows);
    assert_eq!(state.toggled, HashSet::from(["a"]));

    let zoom = TileZoom::bounded(0.5, 2.0, 0.1, 1.0);
    state.zoom = 9.0;
    assert_eq!(state.zoom_in(&zoom), 2.0);
    state.zoom = f64::NAN;
    assert_eq!(state.zoom_in(&zoom), 1.0);
}

#[trial]
fn the_panel_menu_lists_its_commands_and_runs_them() {
    let files = PresetFiles::<u32>::new("contours", "Contours", &["shc"])
        .replace(true)
        .can_export(|selected| selected == Some(&1))
        .export(|selected| selected.map(|_| craft_ui::patterns::presets::PresetExport::new("C.shc", vec![1])));
    let menu = PresetPanelMenu::new(PresetView::List).new_group(false).files(&files);
    assert_eq!(
        menu.commands(),
        [
            PresetCommands::GridView,
            PresetCommands::ListView,
            PresetCommands::NewGroup,
            PresetCommands::Load,
            PresetCommands::Replace,
            PresetCommands::Save
        ]
    );
    assert_eq!(menu.rows().len(), 7, "a separator after the view rows");
    assert_eq!(menu.run_command(PresetCommands::ListView), Some(PresetMenuAction::View(PresetView::List)));
    assert_eq!(menu.run_command(PresetCommands::NewGroup), None, "disabled");
    assert_eq!(menu.run_command(PresetCommands::Save), None, "nothing to export without a selection");
    assert!(matches!(menu.run_command(PresetCommands::Replace), Some(PresetMenuAction::File(PresetFileRequest::Open { replace: true, .. }))));

    let one = 1u32;
    let selected = PresetPanelMenu::new(PresetView::Grid).new_group(true).files(&files).selected(Some(&one));
    assert_eq!(selected.run_command(PresetCommands::NewGroup), Some(PresetMenuAction::NewGroup));
    assert!(matches!(selected.run_command(PresetCommands::Save), Some(PresetMenuAction::File(PresetFileRequest::Save { .. }))));
}

#[trial]
fn a_file_request_turns_the_hosts_answer_into_the_matching_event() {
    let files = PresetFiles::<u32>::new("contours", "Contours", &["shc"]);
    let open = files.request(PresetCommands::Load, None).unwrap();
    assert_eq!(files.request(PresetCommands::Replace, None), None, "Replace… is not offered");
    assert!(matches!(open.loaded("a.shc", vec![4, 5]), Some(PresetFileEvent::Loaded { bytes, replace: false, .. }) if bytes == [4, 5]));
    assert_eq!(open.saved("a.shc"), None);
    assert!(matches!(open.failed("a.shc", PresetFileFailure::NotFound), PresetFileEvent::Failed { writing: false, .. }));
}
