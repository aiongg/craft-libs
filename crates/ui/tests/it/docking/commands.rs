//! Dock commands: a panel command, run through the dock model, leaves the layout the command names; arrangements
//! and workspace commands change the windows and the set; the switcher reports the pick and fires its command rows.

use super::sizes;
use craft_behavior::dock::{
    Column, DockEdge, DocumentGroup, DocumentWindows, Group, Layout, NamedWorkspace, Node, PanelPlace, Permissions, SplitSize, Workspace,
    WorkspaceSet,
};
use craft_behavior::{Axis, DockSide};
use craft_ui::docking::{
    ArrangeCommands, ArrangeSpace, Arranged, DockCommands, DockScene, DockTo, DragUnit, FloatSpace, WorkspaceAsk, WorkspaceCommands, actions,
    arrange, dock_states, move_actions, run_workspace, select_workspace,
};
use craft_ui::patterns::dock::WorkspaceSwitcher;
use craft_ui::runtime::commands::{CommandSet as _, CommandState};
use craft_ui::runtime::frame::take_output;
use craft_ui::testing::focus::AccessActionKind;
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
    let column = Column::new(vec![Group::new(vec![TOP, TOP_TAB], 0), Group::new(vec![LOWER], 0)]).with_width(COLUMN_WIDTH);
    let dock = Node::dock(DockEdge::Start, vec![column]);
    let root = Node::split(Axis::X, SplitSize::FixedFirst(COLUMN_WIDTH), dock, Node::tabs(vec![CANVAS], 0));
    Workspace::new(Layout::new(Some(root), Vec::new()))
}

fn space() -> FloatSpace<u32> {
    FloatSpace::new(AREA, &sizes(4.0, 40.0, 24.0))
}

/// Runs `command` on `panel` of `workspace` and applies the actions.
fn run(workspace: &mut Workspace<u32>, command: DockCommands, panel: u32, locked: bool) -> Result<(), craft_behavior::dock::DockError> {
    let space = space();
    let permissions = |_: &u32| Permissions::ALL;
    let scene = DockScene::new(&workspace.layout, &space, locked, &permissions);
    let planned = actions(&scene, command, &panel)?;
    workspace.apply_all(planned, locked, permissions).map(drop)
}

#[trial]
fn move_actions_dock_a_panel_as_a_tab_beside_a_group_or_in_a_new_dock() {
    let space = space();
    let permissions = |_: &u32| Permissions::ALL;

    let mut tabbed = workspace();
    let scene = DockScene::new(&tabbed.layout, &space, false, &permissions);
    let planned = move_actions(&scene, &DragUnit::Panel(TOP), DockTo::tabs(LOWER, None)).unwrap();
    tabbed.apply_all(planned, false, permissions).unwrap();
    assert_eq!(tabbed.layout.place(&TOP), tabbed.layout.place(&LOWER), "the panel joined the lower group");

    let mut new_dock = workspace();
    let scene = DockScene::new(&new_dock.layout, &space, false, &permissions);
    let planned = move_actions(&scene, &DragUnit::Group(LOWER), DockTo::Dock(DockEdge::End)).unwrap();
    new_dock.apply_all(planned, false, permissions).unwrap();
    assert!(new_dock.layout.dock_path(DockEdge::End).is_some(), "the group went to a dock of its own on the end edge");

    let mut beside = workspace();
    let scene = DockScene::new(&beside.layout, &space, false, &permissions);
    let planned = move_actions(&scene, &DragUnit::Panel(LOWER), DockTo::beside(TOP, DockSide::End)).unwrap();
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
    assert_eq!(arrange(&mut windows, ArrangeCommands::TileAllVertically, &arrange_space(0)), Arranged::Windows);
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
    arrange(&mut windows, ArrangeCommands::FourUp, &arrange_space(0));
    assert_eq!(windows.docked.len(), 4);
    assert_eq!(windows.docked.iter().map(|group| group.documents.len()).sum::<usize>(), 6, "no document is lost");
    arrange(&mut windows, ArrangeCommands::ConsolidateAllToTabs, &arrange_space(0));
    assert_eq!(windows.docked.len(), 1);
}

/// Runs the workspace commands fired since the last call, as a host that binds `WorkspaceCommands` does.
fn run_fired(fixture: &Fixture, set: &RefCell<WorkspaceSet<u32>>, live: &mut Workspace<u32>) {
    for fired in take_output(fixture.ctx()).fired {
        if let Some(command) = WorkspaceCommands::from_ref(&fired.command) {
            run_workspace(&mut set.borrow_mut(), live, command);
        }
    }
}

#[trial]
fn lock_picked_once_in_the_switcher_locks_the_set_of_a_host_that_binds_the_commands() {
    let set = Rc::new(RefCell::new(WorkspaceSet::new(vec![NamedWorkspace::new("essentials", "Essentials", Workspace::<u32>::default())])));
    let shown = Rc::clone(&set);
    let mut fixture = Fixture::new(move |ui| {
        let _ = WorkspaceSwitcher::new(&shown.borrow()).show(ui);
    });
    let mut live = Workspace::default();
    fixture.settle();
    let select = fixture.by_label("Choose a workspace").expect("the switcher has a node");
    fixture.click(&select);
    fixture.idle(FADE_FRAMES);
    let more = fixture.by_label("More options").expect("the list has a header menu");
    fixture.access(AccessActionKind::Click, &more);
    fixture.idle(FADE_FRAMES);
    let lock = fixture.by_label("Lock Workspace").expect("the header menu lists Lock Workspace");
    fixture.access(AccessActionKind::Click, &lock);
    fixture.idle(FADE_FRAMES);
    run_fired(&fixture, &set, &mut live);
    assert!(set.borrow().locked, "one pick of Lock runs the command once");
}
