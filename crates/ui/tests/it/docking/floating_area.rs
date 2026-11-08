//! Floating windows of a `DockArea`: their title row follows the panels' permissions, Float docks them, a move is one
//! workspace edit, a resize holds, and a drop over a window lands on it.

use craft_behavior::dock::{Action, Floating, Layout, Node, Permissions, Workspace};
use craft_behavior::{EditEvent, GesturePhase};
use craft_ui::contract::{Bind, BodyCx, Role};
use craft_ui::docking::{DockConfig, DockPanels, PanelSpec};
use craft_ui::patterns::dock::DockArea;
use craft_ui::runtime::drag::{self, DropMark};
use craft_ui::testing::memory::global;
use craft_ui::testing::pointer::{moved, press, release};
use craft_ui::testing::{Fixture, trial};
use egui::{Pos2, Rect, Ui, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

const DOCKED: usize = 1;
const WINDOW: usize = 2;
const SECOND: usize = 3;
/// Inside the fixture's room, so the area's bounds clamp neither the window nor the moves and resizes made of it.
const WINDOW_AT: Rect = Rect { min: Pos2::new(130.0, 90.0), max: Pos2::new(330.0, 210.0) };
/// A point on the window's title row, clear of the action parts at its end.
const TITLE_AT: Vec2 = Vec2::new(24.0, 10.0);
const MOVE: Vec2 = Vec2::new(-60.0, 40.0);
const GROW: Vec2 = Vec2::new(0.0, 80.0);
const SLACK: f32 = 1.5;

fn title(panel: usize) -> String {
    format!("Panel {panel}")
}

struct Panels {
    rights: fn(&usize) -> Permissions,
}

impl DockPanels<usize> for Panels {
    fn spec(&self, panel: &usize) -> PanelSpec {
        PanelSpec::new(title(*panel))
    }

    fn permissions(&self, panel: &usize) -> Permissions {
        (self.rights)(panel)
    }

    fn body(&mut self, _: &usize, _: &mut Ui, _: &mut BodyCx<'_>) {}
}

/// A dock area over a bound workspace whose host applies the actions the area reports.
struct Bed {
    fixture: Fixture,
    workspace: Rc<RefCell<Workspace<usize>>>,
    edits: Rc<RefCell<Vec<EditEvent<Workspace<usize>>>>>,
    actions: Rc<RefCell<Vec<Action<usize>>>>,
}

fn bed(layout: Layout<usize>, rights: fn(&usize) -> Permissions) -> Bed {
    let workspace = Rc::new(RefCell::new(Workspace::new(layout)));
    let (edits, actions): (Rc<RefCell<Vec<_>>>, Rc<RefCell<Vec<_>>>) = (Rc::default(), Rc::default());
    let (held, seen_edits, seen_actions) = (Rc::clone(&workspace), Rc::clone(&edits), Rc::clone(&actions));
    let mut fixture = Fixture::new(move |ui| {
        let mut workspace = held.borrow().clone();
        let mut panels = Panels { rights };
        let out = DockArea::new(global("floating dock", "trial"), Bind::bound(&mut workspace), &mut panels, DockConfig::default()).show(ui);
        for action in &out.actions {
            let _ = workspace.apply(action.clone(), false, rights);
        }
        *held.borrow_mut() = workspace;
        seen_edits.borrow_mut().extend(out.edit);
        seen_actions.borrow_mut().extend(out.actions);
    });
    fixture.settle();
    Bed { fixture, workspace, edits, actions }
}

fn with_window(root: Option<Node<usize>>) -> Layout<usize> {
    Layout::new(root, vec![Floating::single(vec![WINDOW], 0, WINDOW_AT)])
}

fn window_rect(bed: &Bed) -> Rect {
    bed.workspace.borrow().layout.floating.first().expect("the window floats").rect
}

/// The rect the window shows at: the largest group named by its panel.
fn shown_rect(bed: &Bed) -> Rect {
    let tree = bed.fixture.tree();
    let groups = bed.fixture.by_role(Role::GROUP);
    groups
        .into_iter()
        .filter(|group| tree.name_of(group.node) == Some(title(WINDOW).as_str()))
        .map(|group| group.rect)
        .max_by(|a, b| a.area().total_cmp(&b.area()))
        .expect("the window shows")
}

fn press_and_move(fixture: &mut Fixture, from: Pos2, by: Vec2) {
    fixture.frame(vec![moved(from)]);
    fixture.frame(vec![press(from)]);
    for step in 1..=4 {
        fixture.frame(vec![moved(from + by * (step as f32 / 4.0))]);
    }
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < SLACK
}

#[trial]
fn a_window_whose_panel_cannot_close_shows_no_close_and_float_docks_it() {
    let rights = |panel: &usize| if *panel == WINDOW { Permissions::ALL.closable(false) } else { Permissions::ALL };
    let mut bed = bed(with_window(Some(Node::tabs(vec![DOCKED], 0))), rights);
    assert!(bed.fixture.by_label("Close").is_none(), "a panel that cannot close has no Close on its window");
    let float = bed.fixture.by_label("Float").expect("the window offers Float");
    bed.fixture.click(&float);
    bed.fixture.settle();
    let layout = bed.workspace.borrow().layout.clone();
    assert!(layout.floating.is_empty(), "Float docked the window: {:?}", bed.actions.borrow());
    assert!(layout.panels().contains(&&WINDOW), "the panel stays in the layout");
}

#[trial]
fn one_drag_of_a_floating_window_is_one_committed_edit() {
    let mut bed = bed(with_window(None), |_| Permissions::ALL);
    press_and_move(&mut bed.fixture, WINDOW_AT.min + TITLE_AT, MOVE);
    bed.fixture.frame(vec![release(WINDOW_AT.min + TITLE_AT + MOVE)]);
    bed.fixture.settle();
    let edits = bed.edits.borrow();
    let phase = |wanted: GesturePhase| edits.iter().filter(|edit| edit.phase() == Some(wanted)).count();
    assert_eq!((phase(GesturePhase::Begin), phase(GesturePhase::Commit)), (1, 1), "one gesture: {edits:?}");
    assert!(edits.iter().all(|edit| edit.id() == edits[0].id()), "every step belongs to it");
    assert!(!bed.actions.borrow().iter().any(|action| matches!(action, Action::MoveFloating { .. })), "the move is no action");
    let rect = window_rect(&bed);
    assert!((rect.min - (WINDOW_AT.min + MOVE)).abs().max_elem() < SLACK, "the workspace holds the moved rect: {rect:?}");
}

#[trial]
fn a_resized_window_shows_at_the_rect_it_reported() {
    let mut bed = bed(with_window(None), |_| Permissions::ALL);
    let corner = WINDOW_AT.max - Vec2::splat(1.0);
    press_and_move(&mut bed.fixture, corner, GROW);
    bed.fixture.frame(vec![release(corner + GROW)]);
    bed.fixture.settle();
    let (kept, shown) = (window_rect(&bed), shown_rect(&bed));
    assert!(kept.height() > WINDOW_AT.height(), "the resize grew the window: {kept:?}");
    assert!(near(shown.height(), kept.height()) && near(shown.top(), kept.top()), "it shows at {kept:?}, not {shown:?}");
}

#[trial]
fn a_tab_dragged_over_a_floating_window_marks_and_joins_the_window() {
    let mut bed = bed(with_window(Some(Node::tabs(vec![DOCKED, SECOND], 0))), |_| Permissions::ALL);
    let tabs = bed.fixture.by_role(Role::TAB);
    let mut docked: Vec<_> = tabs.iter().filter(|tab| !WINDOW_AT.contains(tab.center())).copied().collect();
    docked.sort_by(|a, b| a.rect.min.x.total_cmp(&b.rect.min.x));
    let second = *docked.last().expect("the second docked tab shows");
    let window_tab = tabs.iter().find(|tab| WINDOW_AT.contains(tab.center())).copied().expect("the window's tab shows");
    let over = Pos2::new(window_tab.rect.right() + 20.0, window_tab.center().y);
    press_and_move(&mut bed.fixture, second.center(), over - second.center());
    bed.fixture.frame(Vec::new());
    let mark = drag::mark(bed.fixture.ctx()).expect("the window takes the drop");
    let points = match mark {
        DropMark::Line { from, to, .. } => vec![from, to],
        DropMark::Into(rect) | DropMark::Split(rect) => vec![rect.center()],
        _ => Vec::new(),
    };
    assert!(!points.is_empty() && points.iter().all(|point| WINDOW_AT.expand(SLACK).contains(*point)), "the mark is the window's: {mark:?}");
    bed.fixture.frame(vec![release(over)]);
    bed.fixture.settle();
    let layout = bed.workspace.borrow().layout.clone();
    let window = layout.floating.first().expect("the window stays");
    assert!(window.panels().any(|panel| *panel == SECOND), "the tab joined the window: {:?}", bed.actions.borrow());
}
