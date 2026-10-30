//! Toolbar tests: vertical positions, the selected look, the shortcut hint, overflow by width,
//! the scrolling fallback, reading order in both directions, Tab order and the AccessKit nodes.

use egui::accesskit::{Orientation as AkOrientation, Role, Toggled};
use egui::{Context, Key, KeyboardShortcut, Modifiers, Rect};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::toolbar::stories;
use super::{Toolbar, ToolbarItem};
use crate::canvas::Orientation;
use crate::direction::{self, Direction};
use crate::icons::Icon;
use crate::testing::a11y;
use crate::theme::{Base, Tokens, TokensOf as _, install};

#[derive(Default)]
struct Host {
    selected: Option<(usize, usize)>,
    grid: bool,
    clicked: Option<(usize, usize)>,
    rects: Vec<Rect>,
    area: [f32; 4],
}

fn harness(dir: Direction, size: [f32; 2]) -> HarnessBuilder<Host> {
    Harness::<Host>::builder().with_size(size).with_context_setup(move |ctx: &Context| {
        install(ctx, &Base::suite_default());
        direction::apply(ctx, dir);
    })
}

fn app(orientation: Orientation, overflow: bool) -> impl FnMut(&mut egui::Ui, &mut Host) {
    move |ui, host| {
        let area = ui.available_rect_before_wrap();
        host.area = [area.left(), area.top(), area.right(), area.bottom()];
        // The clicked item stays selected, as a host would do.
        let mut grid = host.grid;
        let selected = host.selected;
        let w = KeyboardShortcut::new(Modifiers::NONE, Key::W);
        let toolbar = Toolbar::new("tools", orientation)
            .label("Tools")
            .selected(selected)
            .overflow(overflow)
            .group([ToolbarItem::new(Icon::Pointer, "Forward Warp").shortcut(&w), ToolbarItem::new(Icon::Droplet, "Reconstruct")])
            .group([ToolbarItem::new(Icon::Move, "Pucker"), ToolbarItem::new(Icon::Target, "Bloat")])
            .group([ToolbarItem::new(Icon::Grid, "Show Grid").toggle(&mut grid)]);
        let out = toolbar.show(ui);
        host.grid = grid;
        host.rects = out.items.iter().map(|r| r.rect).collect();
        if let Some(clicked) = out.clicked {
            host.clicked = Some(clicked);
            if clicked.0 < 2 {
                host.selected = Some(clicked);
            }
        }
    }
}

#[allow(clippy::expect_used)]
fn tokens() -> craft_theme::ToolbarTokens {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let mut t = None;
    ctx.run_ui(egui::RawInput::default(), |ui| t = Some(Tokens::of(ui).component.toolbar)).drop_without_applying_deltas();
    t.expect("tokens")
}

fn run(dir: Direction, size: [f32; 2], orientation: Orientation, overflow: bool) -> Harness<'static, Host> {
    let mut h = harness(dir, size).build_ui_state(app(orientation, overflow), Host::default());
    h.run();
    h
}

#[test]
fn a_vertical_strip_stacks_buttons_with_the_token_sizes() {
    let t = tokens();
    let h = run(Direction::Ltr, [200.0, 400.0], Orientation::Vertical, false);
    let r = &h.state().rects;
    assert_eq!(r.len(), 5);
    assert!(r.iter().all(|r| r.width() == t.button_large && r.height() == t.button_large));
    let area = Rect::from_min_max(egui::pos2(h.state().area[0], h.state().area[1]), egui::pos2(h.state().area[2], h.state().area[3]));
    assert_eq!(r[0].left() - area.left(), t.pad[0]);
    assert_eq!(r[0].top() - area.top(), t.pad[1]);
    assert_eq!(r[1].top() - r[0].bottom(), t.gap);
    assert_eq!(r[2].top() - r[1].bottom(), t.gap + t.group_gap);
    assert!(r[0].right() + t.pad[0] <= area.left() + t.width);
    assert_eq!(r[0].center().x, r[4].center().x);
}

#[test]
fn the_selected_item_is_toggled_and_the_others_are_not() {
    let mut h = run(Direction::Ltr, [200.0, 400.0], Orientation::Vertical, false);
    h.get_by_label("Pucker").click();
    h.run();
    assert_eq!(h.state().clicked, Some((1, 0)));
    assert_eq!(h.get_by_label("Pucker").accesskit_node().toggled(), Some(Toggled::True));
    assert_eq!(h.get_by_label("Bloat").accesskit_node().toggled(), Some(Toggled::False));
}

#[test]
fn a_toggle_item_flips_its_own_state() {
    let mut h = run(Direction::Ltr, [400.0, 60.0], Orientation::Horizontal, false);
    h.get_by_label("Show Grid").click();
    h.run();
    assert!(h.state().grid);
    assert_eq!(h.get_by_label("Show Grid").accesskit_node().toggled(), Some(Toggled::True));
}

#[test]
fn a_shortcut_is_the_buttons_description() {
    let h = run(Direction::Ltr, [200.0, 400.0], Orientation::Vertical, false);
    assert_eq!(h.get_by_label("Forward Warp").accesskit_node().description().as_deref(), Some("W"));
}

#[test]
fn the_toolbar_node_has_its_orientation_and_name() {
    let h = run(Direction::Ltr, [200.0, 400.0], Orientation::Vertical, false);
    let node = h.get_by_role(Role::Toolbar).accesskit_node();
    assert_eq!((node.label().as_deref(), node.orientation()), (Some("Tools"), Some(AkOrientation::Vertical)));
    let h = run(Direction::Ltr, [400.0, 60.0], Orientation::Horizontal, false);
    assert_eq!(h.get_by_role(Role::Toolbar).accesskit_node().orientation(), Some(AkOrientation::Horizontal));
    a11y::assert_labelled(&h);
}

#[test]
fn a_wide_row_has_no_overflow_button_and_shows_every_item() {
    let h = run(Direction::Ltr, [600.0, 60.0], Orientation::Horizontal, true);
    let (t, r) = (tokens(), &h.state().rects);
    assert_eq!(r.len(), 5);
    assert!(r.iter().all(|r| r.width() == t.button && r.height() == t.button));
    assert_eq!(r[1].left() - r[0].right(), t.gap);
    assert_eq!(r[2].left() - r[1].right(), t.button - 2.0);
    assert!(h.query_by_label("More").is_none());
}

#[test]
fn a_narrow_row_moves_the_groups_that_do_not_fit_into_a_menu_that_runs_them() {
    let mut h = run(Direction::Ltr, [190.0, 200.0], Orientation::Horizontal, true);
    let shown = h.state().rects.len();
    assert!((1..5).contains(&shown), "{shown}");
    assert!(h.query_by_label("Show Grid").is_none());
    let more = h.get_by_label("More");
    assert_eq!(more.accesskit_node().has_popup(), Some(egui::accesskit::HasPopup::Menu));
    assert_eq!(more.accesskit_node().data().is_expanded(), Some(false));
    more.click();
    h.run();
    assert_eq!(h.get_by_label("More").accesskit_node().data().is_expanded(), Some(true));
    h.get_by_label("Show Grid").click();
    h.run();
    assert_eq!(h.state().clicked, Some((2, 0)));
    assert!(h.state().grid, "the menu row flips the toggle");
}

#[test]
fn the_overflow_menu_checks_the_selected_item() {
    let mut h = harness(Direction::Ltr, [190.0, 200.0])
        .build_ui_state(app(Orientation::Horizontal, true), Host { selected: Some((1, 1)), ..Host::default() });
    h.run();
    h.get_by_label("More").click();
    h.run();
    assert_eq!(h.get_by_label("Bloat").accesskit_node().toggled(), Some(Toggled::True));
    assert_eq!(h.get_by_label("Pucker").accesskit_node().toggled(), Some(Toggled::False));
}

#[test]
fn a_row_too_narrow_for_the_overflow_button_scrolls_and_keeps_every_item() {
    let t = tokens();
    let h = run(Direction::Ltr, [t.overflow - 4.0, 60.0], Orientation::Horizontal, true);
    assert_eq!(h.state().rects.len(), 5);
    assert!(h.query_by_label("More").is_none());
}

#[test]
fn a_row_without_overflow_scrolls_when_it_is_too_narrow() {
    let h = run(Direction::Ltr, [120.0, 60.0], Orientation::Horizontal, false);
    assert_eq!(h.state().rects.len(), 5);
    assert!(h.query_by_label("More").is_none());
}

#[test]
fn a_horizontal_row_mirrors_in_a_right_to_left_ui() {
    let ltr = run(Direction::Ltr, [600.0, 60.0], Orientation::Horizontal, true);
    let rtl = run(Direction::Rtl, [600.0, 60.0], Orientation::Horizontal, true);
    let (l, r) = (&ltr.state().rects, &rtl.state().rects);
    assert!(l[0].left() < l[1].left() && l[1].left() < l[4].left());
    assert!(r[0].left() > r[1].left() && r[1].left() > r[4].left());
    assert!((l[0].left() - ltr.state().area[0]).abs() < 1.0 && (r[0].right() - rtl.state().area[2]).abs() < 1.0);
}

#[test]
fn a_scrolling_row_opens_at_the_reading_start_in_a_right_to_left_ui() {
    let h = run(Direction::Rtl, [120.0, 60.0], Orientation::Horizontal, false);
    let area = h.state().area;
    let first = h.state().rects[0];
    assert!(first.right() <= area[2] + 1.0 && first.left() >= area[0], "first item {first:?} outside {area:?}");
}

#[test]
fn tab_visits_the_buttons_in_reading_order_and_the_overflow_button_last() {
    let mut h = run(Direction::Ltr, [190.0, 200.0], Orientation::Horizontal, true);
    let shown = h.state().rects.len();
    let order: Vec<String> = a11y::tab_order(&mut h).into_iter().map(|s| s.label).collect();
    let all = ["Forward Warp", "Reconstruct", "Pucker", "Bloat", "Show Grid"];
    let mut expected: Vec<&str> = all[..shown].to_vec();
    expected.push("More");
    assert_eq!(order, expected);
}

#[test]
fn every_story_names_every_control() {
    for dir in [Direction::Ltr, Direction::Rtl] {
        for story in stories() {
            let mut args = (story.args)();
            let show = story.show;
            let mut h = harness(dir, [400.0, 520.0]).build_ui_state(move |ui: &mut egui::Ui, _: &mut Host| show(ui, &mut args), Host::default());
            h.run();
            a11y::assert_labelled(&h);
        }
    }
}
