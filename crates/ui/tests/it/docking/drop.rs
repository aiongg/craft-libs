//! Drop resolution: a spot under the pointer becomes the model action that moves the dragged panel or group.

use craft_behavior::dock::{Action, Column, DockEdge, Floating, Group, Layout, Node, Permissions, Placement, SplitSize, Target};
use craft_behavior::{Axis, DockSide};
use craft_theme::{Contrast, ControlSize, Mode};
use craft_ui::docking::{DockDrag, DockSizes, DockSpot, DockTargets, DragUnit, FloatSpace, float_drop, resolve_drop};
use craft_ui::style::Variant;
use craft_ui::testing::memory::global;
use craft_ui::testing::style::{tokens, with_cx};
use craft_ui::testing::{Frames, trial};
use egui::{Context, Id, Pos2, Rect, Vec2};

const TOP_GROUP: u32 = 1;
const TOP_SECOND_TAB: u32 = 2;
const LOWER_GROUP: u32 = 3;
const CANVAS: u32 = 9;
const WINDOW: u32 = 7;
const COLUMN_WIDTH: f32 = 200.0;

/// A dock column of two groups, `[1, 2]` above `[3]`, beside a canvas.
fn layout() -> Layout<u32> {
    let column = Column::new(vec![Group::new(vec![TOP_GROUP, TOP_SECOND_TAB], 0), Group::new(vec![LOWER_GROUP], 0)], COLUMN_WIDTH);
    let dock = Node::dock(DockEdge::Start, vec![column]);
    let window = Floating::new(Node::tabs(vec![WINDOW], 0), Rect::from_min_size(at(500.0, 0.0), Vec2::new(200.0, 150.0)));
    Layout::new(Some(Node::split(Axis::X, SplitSize::FixedFirst(COLUMN_WIDTH), dock, Node::tabs(vec![CANVAS], 0))), vec![window])
}

fn dock_id() -> Id {
    global("drop registry", "dock")
}

fn at(x: f32, y: f32) -> Pos2 {
    Pos2::new(x, y)
}

/// The spots a dock column registers: the top group's strip (its tabs are 100 wide) and body sides, and the lower
/// group's strip.
fn register(ctx: &Context) {
    let dock = dock_id();
    let rect = |min: Pos2, size: Vec2| Rect::from_min_size(min, size);
    DockTargets::register(ctx, dock, DockSpot::group(TOP_GROUP, DockSide::Center), rect(at(0.0, 24.0), Vec2::new(COLUMN_WIDTH, 100.0)));
    DockTargets::register(ctx, dock, DockSpot::group(TOP_GROUP, DockSide::Bottom), rect(at(0.0, 124.0), Vec2::new(COLUMN_WIDTH, 24.0)));
    DockTargets::register(ctx, dock, DockSpot::group(TOP_GROUP, DockSide::Start), rect(at(0.0, 24.0), Vec2::new(24.0, 100.0)));
    DockTargets::register(ctx, dock, DockSpot::tab(TOP_GROUP, 1), rect(at(100.0, 0.0), Vec2::new(100.0, 24.0)));
    DockTargets::register(ctx, dock, DockSpot::tab(TOP_GROUP, 2), rect(at(0.0, 0.0), Vec2::new(24.0, 24.0)));
    DockTargets::register(ctx, dock, DockSpot::group(LOWER_GROUP, DockSide::Center), rect(at(0.0, 200.0), Vec2::new(COLUMN_WIDTH, 100.0)));
    DockTargets::register(ctx, dock, DockSpot::<u32>::NewDock(DockSide::End), rect(at(380.0, 0.0), Vec2::new(20.0, 300.0)));
}

/// Resolves a drop of `unit` at `pointer` in the pass after the spots were registered.
fn drop_of(unit: DragUnit<u32>, pointer: Pos2, permissions: impl Fn(&u32) -> Permissions) -> Option<Action<u32>> {
    struct Carried(DragUnit<u32>);
    impl DockDrag<u32> for Carried {
        fn unit(&self, _layout: &Layout<u32>) -> Option<DragUnit<u32>> {
            Some(self.0.clone())
        }
    }
    let mut frames = Frames::new();
    frames.frame(Vec::new(), |ui| register(ui.ctx()));
    let mut resolved = None;
    frames.frame(Vec::new(), |ui| {
        register(ui.ctx());
        resolved = resolve_drop(ui.ctx(), dock_id(), &layout(), pointer, &Carried(unit.clone()), &permissions);
    });
    resolved
}

fn open(_: &u32) -> Permissions {
    Permissions::default()
}

#[trial]
fn a_panel_dropped_on_a_strip_joins_the_tabs_before_the_tab_there() {
    let action = drop_of(DragUnit::Panel(LOWER_GROUP), at(150.0, 10.0), open);
    assert_eq!(action, Some(Action::move_beside(LOWER_GROUP, TOP_GROUP, Placement::tab(Some(TOP_SECOND_TAB)))));
}

#[trial]
fn a_tab_dropped_after_its_own_neighbour_reorders_its_group() {
    let action = drop_of(DragUnit::Panel(TOP_GROUP), at(10.0, 10.0), open);
    assert_eq!(
        action,
        Some(Action::move_beside(TOP_GROUP, TOP_SECOND_TAB, Placement::tab(None))),
        "the destination is a panel that stays, not the dragged one"
    );
}

#[trial]
fn a_group_side_docks_beside_in_the_vocabulary_of_its_container() {
    let below = drop_of(DragUnit::Panel(TOP_SECOND_TAB), at(100.0, 136.0), open);
    assert_eq!(below, Some(Action::move_beside(TOP_SECOND_TAB, TOP_GROUP, Placement::group(true, None))), "a dock column stacks groups");
    let start = drop_of(DragUnit::Panel(LOWER_GROUP), at(10.0, 60.0), open);
    assert_eq!(start, Some(Action::move_beside(LOWER_GROUP, TOP_GROUP, Placement::column(false))), "a column edge makes a column");
}

#[trial]
fn the_middle_of_a_group_joins_it_but_never_its_own_group() {
    let joined = drop_of(DragUnit::Group(LOWER_GROUP), at(100.0, 60.0), open);
    assert_eq!(joined, Some(Action::move_group(LOWER_GROUP, Target::at(TOP_GROUP, Placement::Split(DockSide::Center)))));
    assert_eq!(drop_of(DragUnit::Panel(TOP_GROUP), at(100.0, 60.0), open), None, "a panel gains nothing joining its own group");
    assert_eq!(drop_of(DragUnit::Group(LOWER_GROUP), at(100.0, 250.0), open), None, "a group cannot dock on itself");
}

#[trial]
fn permissions_refuse_a_drop_the_destination_or_the_unit_does_not_allow() {
    let no_tabs = |panel: &u32| if *panel == TOP_GROUP { Permissions::ALL.accepts_tabs(false) } else { Permissions::default() };
    assert_eq!(drop_of(DragUnit::Panel(LOWER_GROUP), at(100.0, 60.0), no_tabs), None, "the group takes no tabs");
    assert!(drop_of(DragUnit::Panel(TOP_SECOND_TAB), at(100.0, 136.0), no_tabs).is_some(), "it still takes a new group beside it");
    let pinned = |panel: &u32| if *panel == LOWER_GROUP { Permissions::ALL.movable(false) } else { Permissions::default() };
    assert_eq!(drop_of(DragUnit::Panel(LOWER_GROUP), at(100.0, 60.0), pinned), None, "a panel that cannot move is not dropped");
}

#[trial]
fn the_new_dock_band_gives_the_unit_a_dock_of_its_own() {
    let action = drop_of(DragUnit::Group(TOP_GROUP), at(390.0, 100.0), open);
    assert_eq!(action, Some(Action::move_group(TOP_GROUP, Target::new_dock(DockEdge::End))));
}

#[trial]
fn a_float_drop_puts_the_window_under_the_pointer_inside_the_area() {
    let frames = Frames::new();
    let sizes = with_cx(&frames.ctx, &tokens(Mode::Dark, Contrast::Standard), Variant::SECONDARY, ControlSize::Md, DockSizes::style);
    let area = Rect::from_min_size(at(0.0, 0.0), Vec2::new(800.0, 600.0));
    let space = FloatSpace::<u32>::new(area, &sizes);
    let dropped = float_drop(&DragUnit::Panel(LOWER_GROUP), &space, at(790.0, 590.0));
    let Action::Float { rect, .. } = dropped.action else { panic!("a panel floats alone") };
    let window = rect;
    assert!(area.contains_rect(window), "the window is pulled back inside the area: {window:?}");
}

#[trial]
fn a_floating_window_docks_as_one_unit_on_the_spot_under_it() {
    let action = drop_of(DragUnit::Window(WINDOW), at(100.0, 60.0), open);
    assert_eq!(action, Some(Action::move_window(WINDOW, Target::at(TOP_GROUP, Placement::Split(DockSide::Center)))));
}

#[trial]
fn a_drop_the_model_refuses_resolves_to_nothing() {
    let member_pinned = |panel: &u32| if *panel == TOP_SECOND_TAB { Permissions::ALL.movable(false) } else { Permissions::default() };
    assert_eq!(
        drop_of(DragUnit::Group(TOP_GROUP), at(100.0, 250.0), member_pinned),
        None,
        "the lead may move, but the group holds a panel that cannot, and the model refuses the move"
    );
    assert!(drop_of(DragUnit::Group(TOP_GROUP), at(100.0, 250.0), open).is_some(), "the same drop with nothing pinned is taken");
}
