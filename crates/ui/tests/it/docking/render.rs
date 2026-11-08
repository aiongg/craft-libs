//! Docking render maths: divider travel with the shrink chain, fixed sizes, accordion sections and the hidden-panel
//! prune.

use craft_behavior::Axis;
use craft_behavior::dock::{Layout, Node, SplitSize};
use craft_ui::docking::frame::maximize;
use craft_ui::docking::stack::{StackSection, section_heights};
use craft_ui::docking::tree::{self, FrameRules};
use craft_ui::docking::{DockSizes, HidePanels, PanelLimits};
use craft_ui::testing::trial;
use egui::{Pos2, Rect, Vec2};

const WIDTH: f32 = 400.0;
const HEIGHT: f32 = 300.0;
const LEAST_WIDTH: f32 = 60.0;
const HEADER: f32 = 20.0;
const EPS: f32 = 0.01;

const A: usize = 0;
const B: usize = 1;
const C: usize = 2;

fn sizes() -> DockSizes {
    super::sizes(0.0, 0.0, HEADER)
}

fn area(width: f32) -> Rect {
    Rect::from_min_size(Pos2::ZERO, Vec2::new(width, HEIGHT))
}

fn limits(_: &usize) -> PanelLimits {
    PanelLimits::at_least(Vec2::new(LEAST_WIDTH, 0.0))
}

fn widths(root: &Node<usize>, rect: Rect, rules: &FrameRules<'_, usize>) -> Vec<f32> {
    tree::frames(root, rect, &sizes(), rules, false).groups.iter().map(|group| group.rect.width()).collect()
}

/// `A | (B | C)`: three groups side by side, the first divider ahead of a nested split.
fn three_columns() -> Node<usize> {
    let rest = Node::split(Axis::X, SplitSize::Ratio(0.5), Node::tabs(vec![B], 0), Node::tabs(vec![C], 0));
    Node::split(Axis::X, SplitSize::Ratio(0.5), Node::tabs(vec![A], 0), rest)
}

#[trial]
fn a_divider_shrinks_the_groups_beyond_it_in_turn_down_to_their_minimum() {
    let rect = area(WIDTH);
    let rules = FrameRules::new(&limits);
    let mut root = three_columns();
    let first_divider = tree::frames(&root, rect, &sizes(), &rules, false)
        .handles
        .into_iter()
        .find(|handle| handle.path.is_empty())
        .expect("the root split has a handle");
    let sizes = tree::drag_sizes(&root, rect, &sizes(), &rules, &first_divider.path, WIDTH);
    tree::apply_sizes(&mut root, &sizes);

    let after = widths(&root, rect, &rules);
    assert!(
        (after[1] - LEAST_WIDTH).abs() < EPS && (after[2] - LEAST_WIDTH).abs() < EPS,
        "both groups beyond the divider are at their minimum: {after:?}"
    );
    assert!((after.iter().sum::<f32>() - WIDTH).abs() < EPS, "the room they gave went to the group before the divider: {after:?}");
}

#[trial]
fn a_divider_cannot_travel_past_the_minimum_of_every_group_beyond_it() {
    let rect = area(WIDTH);
    let rules = FrameRules::new(&limits);
    let root = three_columns();
    let (lo, hi) = tree::drag_range(&root, rect, &sizes(), &rules, &[]);
    let room_beyond = 2.0 * (WIDTH / 4.0 - LEAST_WIDTH);
    assert!(lo <= 0.0, "the divider can stay");
    assert!((hi - room_beyond).abs() < EPS, "the divider travels as far as the groups beyond it can shrink: {hi}");
    let sizes = tree::drag_sizes(&root, rect, &sizes(), &rules, &[], 10.0 * WIDTH);
    let mut moved = root.clone();
    tree::apply_sizes(&mut moved, &sizes);
    let after = widths(&moved, rect, &rules);
    assert!(after.iter().all(|width| *width >= LEAST_WIDTH - EPS), "no group goes below its minimum: {after:?}");
}

#[trial]
fn a_fixed_split_keeps_its_size_when_the_area_resizes() {
    let rules = FrameRules::new(&limits);
    let fixed = 120.0;
    let root = Node::split(Axis::X, SplitSize::FixedFirst(fixed), Node::tabs(vec![A], 0), Node::tabs(vec![B], 0));
    let narrow = widths(&root, area(WIDTH), &rules);
    let wide = widths(&root, area(2.0 * WIDTH), &rules);
    assert!((narrow[0] - wide[0]).abs() < EPS, "the fixed side keeps its width: {narrow:?} then {wide:?}");
    assert!(wide[1] > narrow[1], "the other side takes the room");
}

#[trial]
fn a_dragged_ratio_split_stays_a_ratio_and_a_fixed_one_stays_fixed() {
    let rect = area(WIDTH);
    let rules = FrameRules::new(&limits);
    let mut ratio = Node::split(Axis::X, SplitSize::Ratio(0.5), Node::tabs(vec![A], 0), Node::tabs(vec![B], 0));
    let mut fixed = Node::split(Axis::X, SplitSize::FixedSecond(150.0), Node::tabs(vec![A], 0), Node::tabs(vec![B], 0));
    for root in [&mut ratio, &mut fixed] {
        let sizes = tree::drag_sizes(root, rect, &sizes(), &rules, &[], 20.0);
        tree::apply_sizes(root, &sizes);
    }
    assert!(matches!(ratio, Node::Split { size: SplitSize::Ratio(_), .. }), "a ratio split keeps its mode");
    assert!(matches!(fixed, Node::Split { size: SplitSize::FixedSecond(_), .. }), "a fixed split keeps its mode");
}

#[trial]
fn closed_sections_take_their_title_and_open_ones_share_the_rest() {
    let sections = [StackSection::new(true), StackSection::new(false), StackSection::new(true)];
    let total = 200.0;
    let heights = section_heights(&sections, total, HEADER);
    assert!((heights[1] - HEADER).abs() < EPS, "a closed section is its title row: {heights:?}");
    assert!((heights[0] - heights[2]).abs() < EPS, "sections without a preferred height share equally: {heights:?}");
    assert!((heights.iter().sum::<f32>() - total).abs() < EPS, "the sections fill the area: {heights:?}");
}

#[trial]
fn a_preferred_section_height_is_kept_and_the_last_sections_give_way_when_it_does_not_fit() {
    let preferred = 90.0;
    let sections = [StackSection::new(true).height(Some(preferred)), StackSection::new(true).height(Some(preferred))];
    let roomy = section_heights(&sections, 300.0, HEADER);
    assert!((roomy[0] - (preferred + HEADER)).abs() < EPS, "the preferred height is kept: {roomy:?}");
    let tight = section_heights(&sections, 2.0 * HEADER + preferred + 10.0, HEADER);
    assert!((tight[0] - (preferred + HEADER)).abs() < EPS && tight[1] < roomy[1], "the last section gives way first: {tight:?}");
}

#[trial]
fn hiding_a_panel_replaces_its_split_with_the_side_that_stays() {
    let layout = Layout::new(Some(three_columns()), Vec::new());
    let keep = |panel: &usize| *panel == A;
    let none_protected = |_: &usize| false;
    let view = maximize(&layout, None, HidePanels::Except(&keep), &none_protected);
    assert_eq!(view.hidden, vec![B, C], "the panels the view leaves out");
    assert!(matches!(view.layout.root, Some(Node::Tabs { .. })), "a split with one visible side becomes that side");
    let protected = |panel: &usize| *panel == B;
    let view = maximize(&layout, None, HidePanels::All, &protected);
    assert_eq!(view.layout.panels().into_iter().copied().collect::<Vec<_>>(), vec![B], "a protected panel always stays");
}

#[trial]
fn a_maximized_frame_is_shown_alone() {
    let layout = Layout::new(Some(three_columns()), Vec::new());
    let none_protected = |_: &usize| false;
    let view = maximize(&layout, Some(&C), HidePanels::None, &none_protected);
    assert_eq!(view.maximized, Some(C));
    assert_eq!(view.layout.panels().into_iter().copied().collect::<Vec<_>>(), vec![C], "only the maximized frame shows");
}
