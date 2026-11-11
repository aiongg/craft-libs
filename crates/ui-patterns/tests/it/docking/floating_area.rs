//! Floating windows of a `DockArea`: a drop over a window lands on it.

use craft_behavior::dock::{Action, Floating, Layout, Node, Permissions, Workspace};
use craft_ui::contract::{Bind, BodyCx, Role};
use craft_ui::docking::{DockConfig, DockPanels, PanelSpec};
use craft_ui::patterns::dock::DockArea;
use craft_ui::runtime::drag::{self, DropMark};
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

/// A dock area over a bound workspace; the area applies its actions.
struct Bed {
    fixture: Fixture,
    workspace: Rc<RefCell<Workspace<usize>>>,
    actions: Rc<RefCell<Vec<Action<usize>>>>,
}

fn bed(layout: Layout<usize>, rights: fn(&usize) -> Permissions) -> Bed {
    let workspace = Rc::new(RefCell::new(Workspace::new(layout)));
    let actions: Rc<RefCell<Vec<Action<usize>>>> = Rc::default();
    let (held, seen_actions) = (Rc::clone(&workspace), Rc::clone(&actions));
    let mut fixture = Fixture::new(move |ui| {
        let mut workspace = held.borrow().clone();
        let mut panels = Panels { rights };
        let out = DockArea::new("Floating dock", Bind::bound(&mut workspace), &mut panels, DockConfig::default()).show(ui);
        *held.borrow_mut() = workspace;
        seen_actions.borrow_mut().extend(out.actions);
    });
    fixture.settle();
    Bed { fixture, workspace, actions }
}

fn with_window(root: Option<Node<usize>>) -> Layout<usize> {
    Layout::new(root, vec![Floating::single(vec![WINDOW], 0, WINDOW_AT)])
}

fn press_and_move(fixture: &mut Fixture, from: Pos2, by: Vec2) {
    fixture.frame(vec![moved(from)]);
    fixture.frame(vec![press(from)]);
    for step in 1..=4 {
        fixture.frame(vec![moved(from + by * (step as f32 / 4.0))]);
    }
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
