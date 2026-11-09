//! The resize grips and dividers of a `DockArea` report their resizes as actions: a column's inner edge, an iconic
//! column's edge, the dividers of an accordion, of a column's groups and of a floating window's tree. Groups in a
//! column stay inside it and apart. A Window menu row reopens a closed panel.

use craft_behavior::Axis;
use craft_behavior::dock::{Action, Column, DockEdge, Floating, Group, Layout, Node, PanelState, Permissions, SplitSize, StackEntry, Workspace};
use craft_theme::{Contrast, ControlSize, Mode};
use craft_ui::buttons::Button;
use craft_ui::contract::overlay::Trigger;
use craft_ui::contract::{Bind, BodyCx, Role, Show as _, ShowContainer as _};
use craft_ui::docking::{DockConfig, DockPanels, DockSizes, PanelSpec, WindowMenuRows};
use craft_ui::menu::Menu;
use craft_ui::patterns::dock::DockArea;
use craft_ui::style::Variant;
use craft_ui::testing::pointer::{moved, press, release};
use craft_ui::testing::style::{tokens, with_cx};
use craft_ui::testing::{Fixture, Target, trial};
use egui::{Pos2, Rect, Ui, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

const FIRST: usize = 1;
const SECOND: usize = 2;
const CLOSED: usize = 4;
const CANVAS: usize = 3;
const COLUMN_WIDTH: f32 = 200.0;
const AREA: Vec2 = Vec2::new(800.0, 600.0);

struct Plain;

impl DockPanels<usize> for Plain {
    fn spec(&self, panel: &usize) -> PanelSpec {
        PanelSpec::new(format!("Panel {panel}"))
    }

    fn permissions(&self, panel: &usize) -> Permissions {
        if *panel == CANVAS { Permissions::PROTECTED } else { Permissions::ALL }
    }

    fn body(&mut self, _: &usize, _: &mut Ui, _: &mut BodyCx<'_>) {}
}

type Actions = Rc<RefCell<Vec<Action<usize>>>>;

/// A fixture of `layout` in an area of `AREA` that applies the area's actions and keeps them.
fn bed(layout: Layout<usize>) -> (Fixture, Actions) {
    let actions: Actions = Rc::default();
    let kept = Rc::clone(&actions);
    let mut workspace = Workspace::new(layout);
    let mut fixture = Fixture::new(move |ui| {
        let mut panels = Plain;
        kept.borrow_mut().extend(DockArea::new("Dock area", Bind::bound(&mut workspace), &mut panels, DockConfig::default()).show(ui).actions);
    });
    fixture.room(AREA).settle();
    (fixture, actions)
}

fn docked(column: Column<usize>) -> Layout<usize> {
    let root = Node::split(Axis::X, SplitSize::FixedFirst(COLUMN_WIDTH), Node::dock(DockEdge::Start, vec![column]), Node::tabs(vec![CANVAS], 0));
    Layout::new(Some(root), Vec::new())
}

fn drag(fixture: &mut Fixture, from: Pos2, to: Pos2) {
    fixture.frame(vec![moved(from)]);
    fixture.frame(vec![press(from)]);
    for step in 1..=4 {
        fixture.frame(vec![moved(from + (to - from) * (step as f32 / 4.0))]);
    }
    fixture.frame(vec![moved(to)]);
    fixture.frame(vec![release(to)]);
}

/// The dividers and grips the area shows, with the narrow ones (vertical bars) or the wide ones (horizontal bars).
fn splitters(fixture: &Fixture, vertical: bool) -> Vec<Target> {
    let mut found: Vec<Target> =
        fixture.by_role(Role::SPLITTER).into_iter().filter(|bar| (bar.rect.height() > bar.rect.width()) == vertical).collect();
    found.sort_by(|a, b| a.rect.min.x.total_cmp(&b.rect.min.x).then(a.rect.min.y.total_cmp(&b.rect.min.y)));
    found
}

fn sizes() -> DockSizes {
    with_cx(&egui::Context::default(), &tokens(Mode::Dark, Contrast::Standard), Variant::SECONDARY, ControlSize::Md, DockSizes::style)
}

#[trial]
fn dragging_a_columns_inner_edge_reports_its_width_and_the_dock_follows() {
    let (mut fixture, actions) = bed(docked(Column::new(vec![Group::new(vec![FIRST], 0)]).with_width(COLUMN_WIDTH)));
    let grip = splitters(&fixture, true).into_iter().find(|bar| bar.rect.center().x <= COLUMN_WIDTH).expect("the column shows its inner edge grip");
    let from = grip.rect.center();
    drag(&mut fixture, from, from + Vec2::new(40.0, 0.0));
    let width = actions.borrow().iter().rev().find_map(|action| match action {
        Action::ResizeColumn { panel, width, .. } if *panel == FIRST => Some(*width),
        _ => None,
    });
    let width = width.expect("the drag reports ResizeColumn");
    assert!(width > COLUMN_WIDTH + 30.0, "the column is wider: {width}");
    let grip = splitters(&fixture, true).into_iter().find(|bar| bar.rect.center().x > COLUMN_WIDTH).expect("the grip moved with the edge");
    assert!(grip.rect.center().x > from.x + 30.0, "the dock follows the new width: {:?}", grip.rect);
}

#[trial]
fn dragging_an_iconic_columns_edge_reports_its_width() {
    let mut column = Column::new(vec![Group::new(vec![FIRST], 0)]).with_width(COLUMN_WIDTH);
    column.iconic = true;
    let (mut fixture, actions) = bed(docked(column));
    let rail = sizes().iconic_least;
    let grip = splitters(&fixture, true).into_iter().find(|bar| bar.rect.center().x <= rail).expect("the iconic column shows its edge grip");
    let from = grip.rect.center();
    drag(&mut fixture, from, from + Vec2::new(30.0, 0.0));
    let width = actions.borrow().iter().rev().find_map(|action| match action {
        Action::ResizeIconic { panel, width, .. } if *panel == FIRST => *width,
        _ => None,
    });
    assert!(width.expect("the drag reports ResizeIconic") > rail + 20.0, "the column is wider than the rail");
}

#[trial]
fn dragging_the_divider_between_accordion_sections_reports_the_section_height() {
    let entries = vec![StackEntry::new(FIRST), StackEntry::new(SECOND)];
    let root = Node::split(Axis::X, SplitSize::FixedFirst(COLUMN_WIDTH), Node::stack(entries), Node::tabs(vec![CANVAS], 0));
    let (mut fixture, actions) = bed(Layout::new(Some(root), Vec::new()));
    let divider = splitters(&fixture, false).into_iter().next().expect("the open sections show a divider");
    let from = divider.rect.center();
    drag(&mut fixture, from, from + Vec2::new(0.0, 60.0));
    assert!(
        actions.borrow().iter().any(|action| matches!(action, Action::ResizeStack { panel, height: Some(_), .. } if *panel == FIRST)),
        "the drag reports ResizeStack for the section above: {:?}",
        actions.borrow()
    );
}

#[trial]
fn dragging_the_lower_half_of_a_columns_group_divider_reports_the_group_height() {
    let column = Column::new(vec![Group::new(vec![FIRST], 0), Group::new(vec![SECOND], 0)]).with_width(COLUMN_WIDTH);
    let (mut fixture, actions) = bed(docked(column));
    let divider = splitters(&fixture, false).into_iter().find(|bar| bar.rect.center().x < COLUMN_WIDTH).expect("the stacked groups show a divider");
    // The lower half of the band lies on the lower group's tab row, a root of its own; the divider stacks above it.
    let from = Pos2::new(divider.rect.center().x, divider.rect.max.y - 1.0);
    drag(&mut fixture, from, from + Vec2::new(0.0, 60.0));
    assert!(
        actions.borrow().iter().any(|action| matches!(action, Action::ResizeGroup { panel, height: Some(_), .. } if *panel == FIRST)),
        "the drag reports ResizeGroup for the group above: {:?}",
        actions.borrow()
    );
}

#[trial]
fn dragging_a_divider_inside_a_floating_window_reports_the_window_split() {
    let window = Node::split(Axis::X, SplitSize::Ratio(0.5), Node::tabs(vec![FIRST], 0), Node::tabs(vec![SECOND], 0));
    let rect = Rect::from_min_size(Pos2::new(300.0, 100.0), Vec2::new(400.0, 300.0));
    let (mut fixture, actions) = bed(Layout::new(Some(Node::tabs(vec![CANVAS], 0)), vec![Floating::new(window, rect)]));
    let divider = splitters(&fixture, true).into_iter().find(|bar| rect.contains(bar.rect.center())).expect("the window's split shows a divider");
    let from = divider.rect.center();
    drag(&mut fixture, from, from + Vec2::new(50.0, 0.0));
    assert!(
        actions.borrow().iter().any(|action| matches!(action, Action::ResizeWindowSplit { panel, .. } if *panel == FIRST || *panel == SECOND)),
        "the drag reports ResizeWindowSplit: {:?}",
        actions.borrow()
    );
}

#[trial]
fn groups_stacked_in_a_column_stay_inside_it_and_apart() {
    let column = Column::new(vec![Group::new(vec![FIRST], 0), Group::new(vec![SECOND], 0)]).with_width(COLUMN_WIDTH);
    let (fixture, _) = bed(docked(column));
    let mut tabs: Vec<Target> = fixture.by_role(Role::TAB).into_iter().filter(|tab| tab.rect.center().x < COLUMN_WIDTH).collect();
    tabs.sort_by(|a, b| a.rect.min.y.total_cmp(&b.rect.min.y));
    let [upper, lower] = tabs[..] else { panic!("one tab per group: {tabs:?}") };
    assert!(upper.rect.max.y <= lower.rect.min.y, "the groups do not overlap: {:?} {:?}", upper.rect, lower.rect);
    assert!(upper.rect.max.x <= COLUMN_WIDTH && lower.rect.max.x <= COLUMN_WIDTH, "and stay inside the column");
}

#[trial]
fn a_window_menu_row_of_a_closed_panel_reopens_it() {
    let layout = Layout::new(Some(Node::tabs(vec![FIRST, CANVAS], 0)), Vec::new());
    assert_eq!(layout.panel_state(&CLOSED), PanelState::Hidden);
    let picked: Rc<RefCell<Option<Action<usize>>>> = Rc::default();
    let seen = Rc::clone(&picked);
    let mut fixture = Fixture::new(move |ui| {
        let opener = Button::new("Window").show(ui);
        Menu::new("Window").opened_by(&opener, Trigger::Press).show(ui, |ui, _| {
            let order = [FIRST, CLOSED];
            if let Some(action) = WindowMenuRows::new(&layout, &Plain, &order).show(ui).action {
                *seen.borrow_mut() = Some(action);
            }
        });
    });
    fixture.settle();
    let opener = fixture.by_label("Window").expect("the opener shows");
    fixture.click(&opener).settle();
    // Every row is a check row: a panel's row is a check box item, checked while the panel is in the layout.
    let row = fixture
        .by_role(Role::MENU_ITEM_CHECK_BOX)
        .into_iter()
        .find(|row| fixture.tree().name_of(row.node) == Some("Panel 4"))
        .expect("the closed panel has a row");
    fixture.click(&row);
    assert_eq!(*picked.borrow(), Some(Action::open(CLOSED, None)));
}
