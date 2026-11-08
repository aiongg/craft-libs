//! The divider every docking container shares: dragging, keys, this pass's frames and the layout write-back.

use craft_behavior::Axis;
use craft_behavior::dock::{Column, Group, Layout, Node, SplitSize};
use craft_ui::contract::ShowContainer as _;
use craft_ui::contract::{Bind, Role};
use craft_ui::docking::frame::maximize;
use craft_ui::docking::tree::{self, FrameRules};
use craft_ui::docking::{DockColumn, HidePanels, PanelLimits, PanelSpec, SplitArea};
use craft_ui::runtime::env::RootEnv;
use craft_ui::runtime::frame::take_output;
use craft_ui::testing::input::key_press;
use craft_ui::testing::memory::global;
use craft_ui::testing::pointer;
use craft_ui::testing::{Fixture, Frames, trial};
use egui::{Key, Modifiers, Pos2, Rect, Ui};
use std::cell::RefCell;
use std::rc::Rc;

fn spec(panel: &u8) -> PanelSpec {
    PanelSpec::new(format!("Panel {panel}"))
}

type Rects = Rc<RefCell<Vec<Rect>>>;

/// A column of two open groups whose resizes the host applies each pass, and the groups' rects of the last pass.
fn column() -> (Fixture, Rects) {
    let column = Rc::new(RefCell::new(Column::new(vec![Group::new(vec![1], 0), Group::new(vec![2], 0)], 240.0)));
    let rects: Rects = Rc::default();
    let seen = Rc::clone(&rects);
    let fixture = Fixture::new(move |ui| {
        seen.borrow_mut().clear();
        let shown = column.borrow().clone();
        let out =
            DockColumn::new("Panels", global("dock", "split test"), &shown, &spec).show_groups(ui, |_, rect, _, _| seen.borrow_mut().push(rect));
        for (anchor, height) in out.resized {
            if let Some(group) = column.borrow_mut().groups.iter_mut().find(|group| group.panels.first() == Some(&anchor)) {
                group.height = Some(height);
            }
        }
    });
    (fixture, rects)
}

fn divider(fixture: &Fixture) -> craft_ui::testing::Target {
    fixture.by_role(Role::SPLITTER).into_iter().next().expect("a divider between the two open groups")
}

#[trial]
fn a_column_divider_dragged_past_a_minimum_and_back_stays_under_the_pointer() {
    let (mut fixture, rects) = column();
    fixture.settle();
    let grip = divider(&fixture);
    let at = grip.center();
    fixture.press(&grip);
    for y in [at.y + 2000.0, at.y + 2000.0, at.y + 15.0] {
        fixture.frame(vec![pointer::moved(Pos2::new(at.x, y))]);
    }
    let bottom = rects.borrow()[0].bottom();
    assert!(
        (bottom - (at.y + 15.0)).abs() < 1.0,
        "the divider is at the pointer after the drag came back from past the minimum: {bottom} for {}",
        at.y + 15.0
    );
}

#[trial]
fn the_arrow_keys_step_a_focused_column_divider() {
    let (mut fixture, rects) = column();
    fixture.settle();
    let grip = divider(&fixture);
    fixture.click(&grip);
    let before = rects.borrow()[0].bottom();
    fixture.key(Key::ArrowDown, Modifiers::NONE);
    assert!(rects.borrow()[0].bottom() > before, "Down moved the divider down");
}

fn two_columns() -> Layout<usize> {
    let root = Node::split(Axis::X, SplitSize::Ratio(0.33), Node::tabs(vec![0, 1], 0), Node::tabs(vec![10], 0));
    Layout::new(Some(root), Vec::new())
}

#[trial]
fn the_groups_are_at_the_dragged_divider_in_the_same_pass() {
    let layout = Rc::new(RefCell::new(two_columns()));
    let rects: Rects = Rc::default();
    let seen = Rc::clone(&rects);
    let mut fixture = Fixture::new(move |ui| {
        let limits = |_: &usize| PanelLimits::default();
        let mut layout = layout.borrow_mut();
        SplitArea::new("Panels", Bind::bound(&mut *layout), FrameRules::new(&limits)).show(ui, |_, body| {
            *seen.borrow_mut() = body.group_frames::<usize>().into_iter().map(|(_, rect)| rect).collect();
        });
    });
    fixture.settle();
    let grip = fixture.by_role(Role::SPLITTER).into_iter().next().expect("a handle between the two groups");
    let at = grip.center();
    fixture.press(&grip);
    for x in [at.x + 10.0, at.x + 40.0] {
        fixture.frame(vec![pointer::moved(Pos2::new(x, at.y))]);
        let rects = rects.borrow();
        assert!(
            rects[0].right() <= x + grip.rect.width() && x - grip.rect.width() <= rects[1].left(),
            "the pointer is between the groups of the same pass: {rects:?} at {x}"
        );
    }
}

#[trial]
fn tab_in_a_window_whose_only_keys_belong_to_a_dock_area_toggles_nothing() {
    let mut layout = two_columns();
    let mut show = |ui: &mut Ui| {
        let limits = |_: &usize| PanelLimits::default();
        SplitArea::new("Panels", Bind::bound(&mut layout), FrameRules::new(&limits)).show(ui, |_, _| ());
    };
    let mut frames = Frames::new();
    let mut root = RootEnv::default();
    root.tab_toggles_chrome = true;
    frames.root(root);
    frames.frame(Vec::new(), &mut show);
    take_output(&frames.ctx);
    frames.frame(vec![key_press(Key::Tab, Modifiers::NONE)], &mut show);
    assert!(take_output(&frames.ctx).chrome_toggles.is_empty(), "a divider is no document, so Tab with nothing focused toggles no panels");
}

#[trial]
fn a_resize_under_a_pruned_view_keeps_the_hidden_panels() {
    let rest = Node::split(Axis::X, SplitSize::Ratio(0.5), Node::tabs(vec![1], 0), Node::tabs(vec![2], 0));
    let mut layout = Layout::new(Some(Node::split(Axis::X, SplitSize::Ratio(0.5), Node::tabs(vec![0], 0), rest)), Vec::new());
    let keep = |panel: &usize| *panel != 2;
    let none_protected = |_: &usize| false;
    let original = layout.clone();
    let view = maximize(&original, None, HidePanels::Except(&keep), &none_protected);
    let mut edited = view.layout.clone().into_owned();
    if let Some(root) = edited.root.as_mut() {
        tree::apply_sizes(root, &[(Vec::new(), SplitSize::Ratio(0.8))]);
    }
    view.write_back(&edited, &mut layout);
    assert!(layout.panels().contains(&&2), "the hidden panel stays in the layout");
    assert!(
        matches!(layout.root, Some(Node::Split { size: SplitSize::Ratio(ratio), .. }) if (ratio - 0.8).abs() < 1e-6),
        "the resize reached the layout"
    );
}
