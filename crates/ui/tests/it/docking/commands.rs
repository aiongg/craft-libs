//! Dock commands: a panel command, run through the dock model, leaves the layout the command names; arrangements
//! and workspace commands change the windows and the set; the switcher reports the pick.

use super::sizes;
use craft_behavior::dock::{
    Column, DockEdge, DocumentGroup, DocumentWindows, Group, Layout, NamedWorkspace, Node, PanelPlace, Permissions, SplitSize, Workspace,
    WorkspaceCommand, WorkspaceSet,
};
use craft_behavior::{Axis, DockSide};
use craft_ui::docking::{
    ArrangeCommand, ArrangeSpace, Arranged, DockCommand, DockScene, DockTo, DragUnit, FloatSpace, WorkspaceAsk, WorkspaceCommands, actions, arrange,
    move_actions, run_workspace, select_workspace,
};
use craft_ui::patterns::dock::WorkspaceSwitcher;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, Pos2, Rect, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

const TOP: u32 = 1;
const TOP_TAB: u32 = 2;
const LOWER: u32 = 3;
const CANVAS: u32 = 9;
const COLUMN_WIDTH: f32 = 200.0;
const AREA: Rect = Rect { min: Pos2::ZERO, max: Pos2::new(1200.0, 800.0) };
const WINDOW_SIZE: Vec2 = Vec2::new(400.0, 300.0);
const CASCADE_STEP: f32 = 28.0;
const FADE_FRAMES: usize = 10;

/// A start dock column of two groups, `[1, 2]` above `[3]`, beside a canvas.
fn workspace() -> Workspace<u32> {
    let column = Column::new(vec![Group::new(vec![TOP, TOP_TAB], 0), Group::new(vec![LOWER], 0)], COLUMN_WIDTH);
    let dock = Node::dock(DockEdge::Start, vec![column]);
    let root = Node::split(Axis::X, SplitSize::FixedFirst(COLUMN_WIDTH), dock, Node::tabs(vec![CANVAS], 0));
    Workspace::new(Layout::new(Some(root), Vec::new()))
}

fn space() -> FloatSpace<u32> {
    FloatSpace::new(AREA, &sizes(4.0, 40.0, 24.0))
}

/// Runs `command` on `panel` of `workspace` and applies the actions.
fn run(workspace: &mut Workspace<u32>, command: DockCommand, panel: u32, locked: bool) -> Result<(), craft_behavior::dock::DockError> {
    let space = space();
    let permissions = |_: &u32| Permissions::ALL;
    let scene = DockScene::new(&workspace.layout, &space, locked, &permissions);
    let planned = actions(&scene, command, &panel)?;
    workspace.apply_all(planned, locked, permissions).map(drop)
}

#[trial]
fn float_panel_floats_one_tab_and_float_group_floats_the_whole_group() {
    let mut one = workspace();
    run(&mut one, DockCommand::FloatPanel, TOP, false).expect("the panel floats");
    assert!(matches!(one.layout.place(&TOP), Some(PanelPlace::Floating { .. })));
    assert!(matches!(one.layout.place(&TOP_TAB), Some(PanelPlace::Dock { .. })), "its tab stays docked");

    let mut group = workspace();
    run(&mut group, DockCommand::FloatGroup, TOP, false).expect("the group floats");
    let (first, second) = (group.layout.place(&TOP), group.layout.place(&TOP_TAB));
    assert!(matches!(first, Some(PanelPlace::Floating { .. })));
    assert_eq!(first, second, "both tabs are in the one window");
}

#[trial]
fn step_group_down_swaps_the_group_with_its_neighbour() {
    let mut workspace = workspace();
    run(&mut workspace, DockCommand::StepGroupDown, TOP, false).expect("the group steps");
    let at = |panel| match workspace.layout.place(&panel) {
        Some(PanelPlace::Dock { group, .. }) => group,
        other => panic!("{panel} is docked: {other:?}"),
    };
    assert!(at(LOWER) < at(TOP), "the lower group is now above");
    assert!(run(&mut workspace, DockCommand::StepGroupDown, TOP, false).is_err(), "the last group has no neighbour below");
}

#[trial]
fn close_commands_close_the_panel_its_others_or_its_group() {
    let mut panel = workspace();
    run(&mut panel, DockCommand::ClosePanel, TOP, false).unwrap();
    assert!(!panel.layout.contains(&TOP) && panel.layout.contains(&TOP_TAB));

    let mut others = workspace();
    run(&mut others, DockCommand::CloseOthers, TOP, false).unwrap();
    assert!(others.layout.contains(&TOP) && !others.layout.contains(&TOP_TAB));

    let mut group = workspace();
    run(&mut group, DockCommand::CloseGroup, TOP, false).unwrap();
    assert!(!group.layout.contains(&TOP) && !group.layout.contains(&TOP_TAB) && group.layout.contains(&LOWER));
}

#[trial]
fn a_locked_workspace_refuses_a_move_and_a_close() {
    let mut workspace = workspace();
    assert!(run(&mut workspace, DockCommand::ClosePanel, TOP, true).is_err());
    assert!(run(&mut workspace, DockCommand::FloatPanel, TOP, true).is_err());
    assert!(workspace.layout.contains(&TOP), "the layout is unchanged");
}

#[trial]
fn collapse_to_icons_and_minimize_toggle_with_the_current_state() {
    let mut workspace = workspace();
    let iconic = |workspace: &Workspace<u32>| {
        let place = workspace.layout.place(&TOP).expect("docked");
        workspace.layout.column(&place).map(|column| column.iconic)
    };
    run(&mut workspace, DockCommand::CollapseToIcons, TOP, false).unwrap();
    assert_eq!(iconic(&workspace), Some(true));
    run(&mut workspace, DockCommand::CollapseToIcons, TOP, false).unwrap();
    assert_eq!(iconic(&workspace), Some(false));

    run(&mut workspace, DockCommand::ToggleMinimized, TOP, false).unwrap();
    let place = workspace.layout.place(&TOP).unwrap();
    assert!(workspace.layout.group(&place).is_some_and(|group| group.minimized));
}

#[trial]
fn move_actions_dock_a_panel_as_a_tab_beside_a_group_or_in_a_new_dock() {
    let space = space();
    let permissions = |_: &u32| Permissions::ALL;

    let mut tabbed = workspace();
    let scene = DockScene::new(&tabbed.layout, &space, false, &permissions);
    let planned = move_actions(&scene, &DragUnit::Panel(TOP), DockTo::Tabs { anchor: LOWER, before: None }).unwrap();
    tabbed.apply_all(planned, false, permissions).unwrap();
    assert_eq!(tabbed.layout.place(&TOP), tabbed.layout.place(&LOWER), "the panel joined the lower group");

    let mut new_dock = workspace();
    let scene = DockScene::new(&new_dock.layout, &space, false, &permissions);
    let planned = move_actions(&scene, &DragUnit::Group(LOWER), DockTo::Dock(DockEdge::End)).unwrap();
    new_dock.apply_all(planned, false, permissions).unwrap();
    assert!(new_dock.layout.dock_path(DockEdge::End).is_some(), "the group went to a dock of its own on the end edge");

    let mut beside = workspace();
    let scene = DockScene::new(&beside.layout, &space, false, &permissions);
    let planned = move_actions(&scene, &DragUnit::Panel(LOWER), DockTo::Beside { anchor: TOP, side: DockSide::End }).unwrap();
    beside.apply_all(planned, false, permissions).unwrap();
    match (beside.layout.place(&TOP), beside.layout.place(&LOWER)) {
        (Some(PanelPlace::Dock { column: a, .. }), Some(PanelPlace::Dock { column: b, .. })) => assert_ne!(a, b, "a new column beside the first"),
        other => panic!("both panels are docked: {other:?}"),
    }
}

fn documents(count: u32) -> DocumentWindows<u32> {
    let mut windows = DocumentWindows::default();
    windows.docked = vec![DocumentGroup::new(0, (0..count).collect())];
    windows
}

fn arrange_space(active: u32) -> ArrangeSpace<u32> {
    ArrangeSpace::new(AREA, WINDOW_SIZE, CASCADE_STEP, Some(active))
}

#[trial]
fn tile_all_vertically_floats_every_document_in_its_own_column_of_the_area() {
    let mut windows = documents(3);
    assert_eq!(arrange(&mut windows, ArrangeCommand::TileAllVertically, &arrange_space(0)), Arranged::Windows);
    assert!(windows.docked.is_empty());
    let rects: Vec<Rect> = windows.floating.iter().map(|(_, rect)| *rect).collect();
    assert_eq!(rects.len(), 3);
    for pair in rects.windows(2) {
        assert!(pair[0].right() <= pair[1].left() + 0.5, "side by side: {pair:?}");
    }
    assert!(rects.iter().all(|rect| AREA.contains_rect(*rect)));
}

#[trial]
fn n_up_arrangements_split_the_documents_into_that_many_groups_and_consolidate_returns_one() {
    let mut windows = documents(6);
    arrange(&mut windows, ArrangeCommand::FourUp, &arrange_space(0));
    assert_eq!(windows.docked.len(), 4);
    assert_eq!(windows.docked.iter().map(|group| group.documents.len()).sum::<usize>(), 6, "no document is lost");
    arrange(&mut windows, ArrangeCommand::ConsolidateAllToTabs, &arrange_space(0));
    assert_eq!(windows.docked.len(), 1);
}

#[trial]
fn float_in_window_floats_the_active_document_and_match_commands_ask_the_views() {
    let mut windows = documents(2);
    arrange(&mut windows, ArrangeCommand::FloatInWindow, &arrange_space(1));
    assert_eq!(windows.floating.len(), 1);
    assert_eq!(windows.floating[0].0.documents, vec![1]);
    let matched = arrange(&mut windows, ArrangeCommand::MatchAll, &arrange_space(1));
    assert!(matches!(matched, Arranged::Views { zoom: true, location: true, rotation: true, .. }));
}

#[trial]
fn lock_and_reset_act_on_the_set_and_select_keeps_each_workspaces_edits() {
    let painting = NamedWorkspace::new("painting", "Painting", workspace());
    let essentials = NamedWorkspace::new("essentials", "Essentials", Workspace::default());
    let mut set = WorkspaceSet::new(vec![essentials, painting]);
    let mut live = Workspace::<u32>::default();

    assert_eq!(run_workspace(&mut set, &mut live, WorkspaceCommands::Lock), None);
    assert!(set.locked);
    assert_eq!(run_workspace(&mut set, &mut live, WorkspaceCommands::New), Some(WorkspaceAsk::Name));
    assert_eq!(run_workspace(&mut set, &mut live, WorkspaceCommands::Delete), Some(WorkspaceAsk::Which));

    select_workspace(&mut set, &mut live, "painting").unwrap();
    assert_eq!(live, workspace(), "the target's layout is live");
    live.apply(craft_behavior::dock::Action::close(TOP), false, |_| Permissions::ALL).unwrap();
    select_workspace(&mut set, &mut live, "essentials").unwrap();
    select_workspace(&mut set, &mut live, "painting").unwrap();
    assert!(!live.layout.contains(&TOP), "the edit was kept under its workspace");
    run_workspace(&mut set, &mut live, WorkspaceCommands::Reset);
    assert_eq!(live, workspace(), "Reset returns the stored layout");
    assert!(select_workspace(&mut set, &mut live, "missing").is_err());
}

#[trial]
fn the_switcher_reports_the_workspace_that_was_picked() {
    let set = WorkspaceSet::new(vec![
        NamedWorkspace::new("essentials", "Essentials", Workspace::<u32>::default()),
        NamedWorkspace::new("painting", "Painting", Workspace::default()),
    ]);
    let reported: Rc<RefCell<Vec<(Option<String>, Option<WorkspaceCommand>)>>> = Rc::default();
    let seen = Rc::clone(&reported);
    let mut fixture = Fixture::new(move |ui| {
        let out = WorkspaceSwitcher::new(&set).show(ui);
        seen.borrow_mut().push((out.switched, out.command));
    });
    fixture.settle();
    fixture.key(Key::Tab, Modifiers::NONE);
    fixture.idle(2);
    fixture.key(Key::ArrowDown, Modifiers::ALT);
    fixture.idle(2);
    fixture.key(Key::ArrowDown, Modifiers::NONE);
    fixture.idle(FADE_FRAMES);
    fixture.key(Key::Enter, Modifiers::NONE);
    fixture.idle(FADE_FRAMES);
    assert!(reported.borrow().iter().any(|(switched, _)| switched.as_deref() == Some("painting")), "the pick is reported once as the workspace id");
}
