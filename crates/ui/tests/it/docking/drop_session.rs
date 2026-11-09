//! Drops through the drag session: a dock's `DockDropTarget` takes a tab or a group grip drag, shows the mark
//! and the released drop applies to the layout.

use craft_behavior::Axis;
use craft_behavior::dock::{Layout, Node, Permissions, SplitSize};
use craft_ui::contract::SizeLimits;
use craft_ui::contract::{Common, Component, Cx, MeasureCx, Pointer as _, Role, Show as _, ShowContainer as _};
use craft_ui::docking::tree::{self, FrameRules};
use craft_ui::docking::{DockDropTarget, DockDropped, DockGroup, DockSizes, GroupDrag, PanelDrag, PanelKey, PanelTitle};
use craft_ui::runtime::drag;
use craft_ui::style::{ComponentStyle, StyleCx};
use craft_ui::testing::pointer::{moved, press, release};
use craft_ui::testing::{Fixture, trial};
use egui::{Id, Pos2, Rect, Response, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

/// The docking sizes with a one-point divider, a 40-point least pane and a 24-point strip.
fn sizes() -> DockSizes {
    super::sizes(1.0, 40.0, 24.0)
}
const AREA: Vec2 = Vec2::new(400.0, 300.0);

#[derive(Clone, Debug, Default, PartialEq, ComponentStyle)]
#[non_exhaustive]
pub struct HostStyle {
    #[style(metrics)]
    metrics: craft_behavior::Metrics,
}

/// The dock area: it registers the dock's drop target over the whole area, as a `DockArea` does.
#[derive(Component)]
struct Host<'a> {
    #[common]
    common: Common,
    target: &'a DockDropTarget<'a, String>,
    seen: &'a RefCell<Vec<DockDropped<String>>>,
}

impl Component for Host<'_> {
    const ROLE: Role = Role::GROUP;
    type Style = HostStyle;
    type Output = Response;

    fn style(_: &StyleCx<'_>, _: HostStyleKey) -> HostStyle {
        HostStyle::default()
    }

    fn measure(&mut self, _: &MeasureCx<'_, Self>) -> Vec2 {
        AREA
    }

    fn update(&mut self, cx: &mut Cx<'_, Self>) {
        if let Some(dropped) = cx.drop_target::<GroupDrag<String>, _>(self.target) {
            self.seen.borrow_mut().push(dropped);
        } else if let Some(dropped) = cx.drop_target::<PanelDrag, _>(self.target) {
            self.seen.borrow_mut().push(dropped);
        }
    }

    fn finish(self, cx: Cx<'_, Self>) -> Response {
        cx.response.clone()
    }
}

fn dock_id() -> Id {
    Id::NULL.with("session dock")
}

fn panels(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

/// `[Layers, Channels]` beside `[Properties]`.
fn two_groups() -> Layout<String> {
    let root = Node::split(Axis::X, SplitSize::Ratio(0.33), Node::tabs(panels(&["Layers", "Channels"]), 0), Node::tabs(panels(&["Properties"]), 0));
    Layout::new(Some(root), Vec::new())
}

/// The panels of each group, in reading order.
fn groups(layout: &Layout<String>) -> Vec<Vec<String>> {
    let limits = |_: &String| SizeLimits::default();
    let rules = FrameRules::new(&limits);
    let Some(root) = layout.root.as_ref() else { return Vec::new() };
    let area = Rect::from_min_size(Pos2::ZERO, AREA);
    tree::frames(root, area, &sizes(), &rules, false)
        .groups
        .iter()
        .map(|group| match group.node {
            Node::Tabs { panels, .. } => panels.clone(),
            _ => Vec::new(),
        })
        .collect()
}

/// The anchor, panels and rect of each group of the layout in an area at `origin`.
fn placed_groups(layout: &Layout<String>, origin: Pos2) -> Vec<(String, Vec<String>, Rect)> {
    let limits = |_: &String| SizeLimits::default();
    let rules = FrameRules::new(&limits);
    let Some(root) = layout.root.as_ref() else { return Vec::new() };
    let frames = tree::frames(root, Rect::from_min_size(origin, AREA), &sizes(), &rules, false);
    frames
        .groups
        .iter()
        .filter_map(|group| match (group.anchor(), group.node) {
            (Some(anchor), Node::Tabs { panels, .. }) => Some((anchor.clone(), panels.clone(), group.rect)),
            _ => None,
        })
        .collect()
}

type Bed = (Fixture, Rc<RefCell<Layout<String>>>, Rc<RefCell<Vec<DockDropped<String>>>>);

/// A dock of the layout with a host that applies the drops it takes.
fn bed(layout: Layout<String>) -> Bed {
    let layout = Rc::new(RefCell::new(layout));
    let taken = Rc::new(RefCell::new(Vec::new()));
    let (shared_layout, shared_taken) = (Rc::clone(&layout), Rc::clone(&taken));
    let fixture = Fixture::new(move |ui| {
        let shown = shared_layout.borrow().clone();
        let permissions = |_: &String| Permissions::default();
        let ctx = ui.ctx().clone();
        let target = DockDropTarget::new(&ctx, dock_id(), &shown, &permissions);
        let origin = ui.max_rect().min;
        Host { common: Common::new(), target: &target, seen: &shared_taken }.show(ui);
        for (anchor, panels, rect) in placed_groups(&shown, origin) {
            let titles: Vec<PanelTitle> = panels.iter().map(|panel| PanelTitle::new(PanelKey::of(panel), panel.clone())).collect();
            let active = titles.first().map_or(PanelKey::new(0), |title| title.key);
            DockGroup::new(anchor, &titles, active).dock(dock_id()).show_at(ui, rect, |_, _| ());
        }
        for dropped in shared_taken.borrow_mut().drain(..) {
            shared_layout.borrow_mut().apply_with_permissions(dropped.action, permissions).expect("the model takes what the dock accepted");
        }
    });
    (fixture, layout, taken)
}

/// Presses the middle of `from` and moves it to `to` in steps, leaving the button down.
fn hold_and_move(fixture: &mut Fixture, from: Pos2, to: Pos2) {
    fixture.frame(vec![moved(from)]);
    fixture.frame(vec![press(from)]);
    for step in 1..=4 {
        fixture.frame(vec![moved(from + (to - from) * (step as f32 / 4.0))]);
    }
    fixture.idle(1);
}

/// The grip of the left group: the second button of its header, after the collapse arrow.
fn left_grip(fixture: &Fixture) -> craft_ui::testing::Target {
    let mut buttons: Vec<_> = fixture.by_role(Role::BUTTON).into_iter().filter(|button| button.rect.min.x < 100.0).collect();
    buttons.sort_by(|a, b| a.rect.min.x.total_cmp(&b.rect.min.x));
    buttons[1]
}

fn tabs_by_x(fixture: &Fixture) -> Vec<craft_ui::testing::Target> {
    let mut tabs = fixture.by_role(Role::TAB);
    tabs.sort_by(|a, b| a.rect.min.x.total_cmp(&b.rect.min.x));
    tabs
}

#[trial]
fn a_panel_dropped_between_two_tabs_lands_between_them() {
    let (mut fixture, layout, _) = bed(two_groups());
    fixture.settle();
    let tabs = tabs_by_x(&fixture);
    let (first, properties) = (&tabs[0], &tabs[2]);
    let between = Pos2::new(first.rect.right() - 2.0, first.center().y);
    hold_and_move(&mut fixture, properties.center(), between);
    assert!(drag::mark(fixture.ctx()).is_some(), "the strip takes the drop between two tabs");
    fixture.frame(vec![release(between)]);
    fixture.idle(2);
    assert_eq!(groups(&layout.borrow())[0], panels(&["Layers", "Properties", "Channels"]));
}

#[trial]
fn a_group_grip_drag_shows_the_drop_mark_over_a_group_side_and_docks_on_release() {
    let (mut fixture, layout, taken) = bed(two_groups());
    fixture.settle();
    let grip = left_grip(&fixture);
    hold_and_move(&mut fixture, grip.center(), Pos2::new(395.0, 150.0));
    assert!(drag::active::<GroupDrag<String>>(fixture.ctx()).is_some(), "the grip started a group drag of the dock's panel type");
    assert!(drag::mark(fixture.ctx()).is_some(), "the dock's target accepts the drag over a group side");
    fixture.frame(vec![release(Pos2::new(395.0, 150.0))]);
    fixture.idle(2);
    assert!(taken.borrow().is_empty(), "the drop was applied in the pass it was taken");
    assert_eq!(groups(&layout.borrow()).len(), 2, "the group moved beside the right group");
    assert_eq!(groups(&layout.borrow())[1], panels(&["Layers", "Channels"]));
}
