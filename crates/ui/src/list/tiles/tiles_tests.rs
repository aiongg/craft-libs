//! `TileGrid` behaviour through egui_kittest (task B6).

use std::cell::RefCell;
use std::collections::BTreeSet;

use crate::behavior::{Axis, DropPosition, Move, Reorder, Selection};
use crate::canvas::textures::{Revision, ThumbRender, ThumbRequest, ThumbSource, ThumbnailOptions, Thumbnails};
use crate::controls::{ControlSize, Rating};
use crate::direction::{self, Direction};
use crate::edit::EditEvent;
use crate::testing::{HarnessExt as _, events};
use crate::theme::{Base, current, install};
use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect, Ui, pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, Node};

use super::fake::FakeTiles;
use super::groups::pinned_rect;
use super::layout::{ArrangeSpacing, arrange};
use super::paint::badge_rects;
use super::{BadgeVisibility, RatingKeys, SpaceAction, TileContent, TileEvent, TileGrid, TileGroup, TileLayout, TileStyle};

/// Cmd on a Mac and Ctrl elsewhere, as a platform reports it: `ctrl` with the abstract `command`.
const CMD: Modifiers = Modifiers { alt: false, ctrl: true, shift: false, mac_cmd: false, command: true };

/// Cards of this side with a caption: six to a row in the 640 point harness.
const CARD: TileLayout = TileLayout::Cards { side: 100.0, aspect: 1.0, caption: true };

/// Records which tiles the grid asked a picture for; every picture is blank.
#[derive(Default)]
struct Pictures {
    asked: RefCell<BTreeSet<usize>>,
}

impl ThumbSource<usize> for Pictures {
    fn revision(&self, key: &usize, _: &ThumbRequest) -> Revision {
        self.asked.borrow_mut().insert(*key);
        Revision::of(key)
    }

    fn render(&self, _: &usize, _: &ThumbRequest) -> ThumbRender {
        ThumbRender::Blank
    }
}

struct Host {
    tiles: FakeTiles,
    pictures: Pictures,
    selection: Selection<usize>,
    events: Vec<TileEvent<usize>>,
    layout: TileLayout,
    groups: Vec<TileGroup>,
    badges: BadgeVisibility,
    check_mode: bool,
    space: SpaceAction,
    rating_keys: RatingKeys,
    hover_scrub: bool,
    reorder: bool,
    direction: Direction,
    renamable: bool,
    start_rename: Option<usize>,
    /// The last pass's `TileGridOutput::drop_target`.
    drop_target: Option<Move<usize>>,
}

impl Host {
    fn new(count: usize) -> Self {
        Self {
            tiles: FakeTiles::new(count),
            pictures: Pictures::default(),
            selection: Selection::new(),
            events: Vec::new(),
            layout: CARD,
            groups: Vec::new(),
            badges: BadgeVisibility::Auto,
            check_mode: false,
            space: SpaceAction::Preview,
            rating_keys: RatingKeys::CommandDigits,
            hover_scrub: false,
            reorder: false,
            direction: Direction::Ltr,
            renamable: false,
            start_rename: None,
            drop_target: None,
        }
    }

    fn take(&mut self) -> Vec<TileEvent<usize>> {
        std::mem::take(&mut self.events)
    }
}

fn show(ui: &mut Ui, host: &mut Host) {
    let thumbnails = Thumbnails::<usize>::in_context(ui.ctx(), "pictures", ThumbnailOptions::default());
    let Host {
        tiles,
        pictures,
        selection,
        events,
        layout,
        groups,
        badges,
        check_mode,
        space,
        rating_keys,
        hover_scrub,
        reorder,
        renamable,
        start_rename,
        drop_target,
        ..
    } = host;
    let mut grid = TileGrid::new("grid", &*tiles)
        .layout(*layout)
        .groups(groups)
        .selection(selection)
        .thumbnails(&thumbnails, &*pictures)
        .badges(*badges)
        .check_mode(*check_mode)
        .space(*space)
        .rating_keys(*rating_keys)
        .hover_scrub(*hover_scrub)
        .label("Photos");
    if *reorder {
        grid = grid.reorder(Reorder::new(Axis::Horizontal));
    }
    grid = grid.renamable(*renamable);
    if let Some(key) = start_rename.take() {
        grid = grid.start_rename(key);
    }
    let output = grid.show(ui);
    events.extend(output.events);
    *drop_target = output.drop_target;
}

fn harness(host: Host) -> Harness<'static, Host> {
    let direction = host.direction;
    let mut harness = Harness::<Host>::builder()
        .with_size([640.0, 480.0])
        .with_step_dt(0.05)
        .with_context_setup(move |ctx| {
            install(ctx, &Base::suite_default());
            direction::apply(ctx, direction);
        })
        .build_ui_state(show, host);
    harness.run();
    harness
}

fn tile<'h>(harness: &'h Harness<'_, Host>, at: usize) -> Node<'h> {
    let label = format!("Tile {at}");
    harness.get_by(move |n| {
        n.role() == Role::ListBoxOption
            && n.label().is_some_and(|l| l.starts_with(&label) && l[label.len()..].chars().next().is_none_or(|c| c == ','))
    })
}

fn centre(harness: &Harness<'_, Host>, at: usize) -> Pos2 {
    tile(harness, at).rect().center()
}

fn picture(harness: &Harness<'_, Host>, at: usize) -> Rect {
    let cell = tile(harness, at).rect();
    Rect::from_min_size(cell.min, vec2(100.0, 100.0))
}

fn click(harness: &mut Harness<'_, Host>, at: usize) {
    let pos = centre(harness, at);
    harness.click_at(pos);
    harness.run_steps(8);
}

fn click_with(harness: &mut Harness<'_, Host>, at: usize, modifiers: Modifiers) {
    let pos = centre(harness, at);
    harness.click_at_with(pos, PointerButton::Primary, modifiers);
    harness.run_steps(8);
}

fn key(harness: &mut Harness<'_, Host>, modifiers: Modifiers, key: Key) {
    harness.key_press_modifiers(modifiers, key);
    harness.run();
}

fn selected(harness: &Harness<'_, Host>) -> Vec<usize> {
    let mut items = harness.state().selection.items.clone();
    items.sort_unstable();
    items
}

fn play(harness: &mut Harness<'_, Host>, frames: Vec<events::Frame>) {
    for frame in frames {
        for event in frame {
            harness.event(event);
        }
        harness.step();
    }
}

fn style(harness: &Harness<'_, Host>) -> TileStyle {
    TileStyle::style(&current(&harness.ctx))
}

// 50 000 fake tiles: `content` is asked only for rows in view ± one viewport; thumbnails VISIBLE and NEAR accordingly.
#[test]
fn only_tiles_in_view_and_one_viewport_around_are_built() {
    let harness = harness(Host::new(50_000));
    let built = harness.state().tiles.asked.borrow().clone();
    let pictures = harness.state().pictures.asked.borrow().clone();
    // A 480 point view shows about three rows of six; one viewport either side adds about seven more.
    assert!(built.len() >= 18, "the rows in view are built: {}", built.len());
    assert!(built.len() <= 6 * 12, "no more than the view and one viewport below it: {}", built.len());
    assert_eq!(built.last(), pictures.last(), "pictures are asked for the tiles built, no further");
    let drawn = harness.query_all_by_role(Role::ListBoxOption).count();
    assert!(drawn <= 6 * 5, "only the tiles in view have nodes: {drawn}");
    assert_eq!(harness.get_by_role(Role::ListBox).accesskit_node().size_of_set(), Some(50_000));
}

// Click replaces, Shift ranges from the anchor, Cmd toggles, background click clears.
#[test]
fn click_shift_and_command_pick_as_in_a_list() {
    let mut harness = harness(Host::new(5));
    click(&mut harness, 1);
    assert_eq!(selected(&harness), [1]);
    click_with(&mut harness, 3, Modifiers::SHIFT);
    assert_eq!(selected(&harness), [1, 2, 3], "a range from the anchor");
    click_with(&mut harness, 0, CMD);
    assert_eq!(selected(&harness), [0, 1, 2, 3], "Cmd adds");
    click_with(&mut harness, 2, CMD);
    assert_eq!(selected(&harness), [0, 1, 3], "Cmd removes");
    click(&mut harness, 4);
    assert_eq!(selected(&harness), [4], "a plain click replaces");
    harness.state_mut().take();
    click(&mut harness, 4);
    assert_eq!(harness.state_mut().take(), [TileEvent::Clicked(4)], "a click on the only selected tile is reported and changes nothing");
    let below = pos2(centre(&harness, 0).x, 400.0);
    harness.click_at(below);
    harness.run_steps(8);
    assert!(harness.state().selection.is_empty(), "the background clears");
}

// Down keeps the column through rows of unequal length.
#[test]
fn arrows_move_the_lead_over_ragged_rows_keeping_the_column() {
    let mut host = Host::new(24);
    host.layout = TileLayout::Justified { row_height: 90.0 };
    host.tiles.aspect = Box::new(|i| if i < 12 { 0.6 } else { 2.0 });
    let mut harness = harness(host);
    // The rows as drawn: tiles grouped by their top.
    let mut drawn: Vec<(usize, f32)> =
        harness.query_all_by_role(Role::ListBoxOption).filter_map(|n| Some((n.accesskit_node().position_in_set()?, n.rect().top()))).collect();
    drawn.sort_by_key(|(at, _)| *at);
    let mut rows: Vec<Vec<usize>> = Vec::new();
    let mut last_top = f32::MIN;
    for (at, top) in drawn {
        if (top - last_top).abs() > 1.0 {
            rows.push(Vec::new());
            last_top = top;
        }
        rows.last_mut().unwrap().push(at);
    }
    assert!(rows.len() >= 3 && rows[0].len() != rows[1].len(), "rows of unequal length: {rows:?}");
    click(&mut harness, 0);
    key(&mut harness, Modifiers::NONE, Key::ArrowDown);
    assert_eq!(selected(&harness), [rows[1][0]]);
    for _ in 0..3 {
        key(&mut harness, Modifiers::NONE, Key::ArrowRight);
    }
    let column = harness.state().selection.lead.map(|l| rows[1].iter().position(|t| *t == l)).unwrap_or(None);
    let row_two = rows[1].len();
    key(&mut harness, Modifiers::NONE, Key::ArrowDown);
    let lead = harness.state().selection.lead.unwrap();
    match column {
        Some(c) => assert_eq!(rows[2].iter().position(|t| *t == lead), Some(c.min(rows[2].len() - 1)), "column kept: {rows:?}"),
        None => assert!(rows[2].contains(&lead) || row_two <= 3, "the lead moved on from row two"),
    }
    key(&mut harness, Modifiers::NONE, Key::ArrowUp);
    assert!(rows[1].contains(&harness.state().selection.lead.unwrap()), "Up returns to the row above");
}

// Space reports Preview; with check mode or SpaceAction::Toggle it toggles; Ctrl+Space always toggles.
#[test]
fn space_previews_and_toggles_in_check_mode() {
    let mut harness_a = harness(Host::new(8));
    click(&mut harness_a, 2);
    harness_a.state_mut().take();
    key(&mut harness_a, Modifiers::NONE, Key::Space);
    assert_eq!(harness_a.state_mut().take(), [TileEvent::Preview(2)]);
    click_with(&mut harness_a, 4, CMD);
    key(&mut harness_a, Modifiers::CTRL, Key::Space);
    assert_eq!(selected(&harness_a), [2], "Ctrl+Space toggles the lead off");

    let mut host = Host::new(8);
    host.space = SpaceAction::Toggle;
    let mut harness_b = harness(host);
    click(&mut harness_b, 2);
    click_with(&mut harness_b, 4, CMD);
    key(&mut harness_b, Modifiers::NONE, Key::Space);
    assert_eq!(selected(&harness_b), [2], "Space toggles with SpaceAction::Toggle");

    let mut host = Host::new(8);
    host.check_mode = true;
    let mut harness_c = harness(host);
    click(&mut harness_c, 2);
    key(&mut harness_c, Modifiers::NONE, Key::ArrowRight);
    key(&mut harness_c, Modifiers::NONE, Key::Space);
    assert_eq!(selected(&harness_c), [2, 3], "in check mode an arrow moves the lead and Space checks it");
    assert!(!harness_c.state().events.iter().any(|e| matches!(e, TileEvent::Preview(_))));
}

// CommandDigits: Cmd+3 rates, 3 does nothing; Digits: 3 rates, 0 clears.
#[test]
fn rating_keys_follow_the_chosen_set() {
    let mut host = Host::new(6);
    host.tiles.rated = Some(0);
    let mut harness_a = harness(host);
    click(&mut harness_a, 1);
    harness_a.state_mut().take();
    key(&mut harness_a, Modifiers::NONE, Key::Num3);
    assert!(harness_a.state_mut().take().is_empty(), "a bare digit does nothing");
    key(&mut harness_a, CMD, Key::Num3);
    assert_eq!(harness_a.state_mut().take(), [TileEvent::Rate { keys: vec![1], rating: 3 }]);
    key(&mut harness_a, CMD, Key::Num0);
    assert_eq!(harness_a.state_mut().take(), [TileEvent::Rate { keys: vec![1], rating: 0 }], "Cmd+0 clears, it does not zoom");

    let mut host = Host::new(6);
    host.tiles.rated = Some(0);
    host.rating_keys = RatingKeys::Digits;
    let mut harness_b = harness(host);
    click(&mut harness_b, 1);
    harness_b.state_mut().take();
    key(&mut harness_b, Modifiers::NONE, Key::Num3);
    assert_eq!(harness_b.state_mut().take(), [TileEvent::Rate { keys: vec![1], rating: 3 }]);
    key(&mut harness_b, Modifiers::NONE, Key::Num0);
    assert_eq!(harness_b.state_mut().take(), [TileEvent::Rate { keys: vec![1], rating: 0 }]);
}

// A star click on a selected tile rates the whole selection; on another tile, that tile only.
#[test]
fn a_star_click_rates_the_whole_selection_when_the_tile_is_in_it() {
    let mut host = Host::new(6);
    host.tiles.rated = Some(2);
    host.badges = BadgeVisibility::Always;
    let mut harness = harness(host);
    click(&mut harness, 1);
    click_with(&mut harness, 2, Modifiers::SHIFT);
    harness.state_mut().take();
    let tokens = current(&harness.ctx);
    let (tile_style, step) = (TileStyle::style(&tokens), Rating::style(&tokens, ControlSize::Sm).badge_step);
    let star = |harness: &Harness<'_, Host>, at: usize, n: f32| {
        let p = picture(harness, at);
        pos2(p.left() + tile_style.inset + 2.0 + step * (n - 0.5), p.bottom() - tile_style.bar / 2.0)
    };
    let fourth = star(&harness, 2, 4.0);
    harness.click_at(fourth);
    harness.run_steps(8);
    assert_eq!(harness.state_mut().take(), [TileEvent::Rate { keys: vec![1, 2], rating: 4 }], "the selection is rated");
    let other = star(&harness, 4, 5.0);
    harness.click_at(other);
    harness.run_steps(8);
    assert_eq!(harness.state_mut().take(), [TileEvent::Rate { keys: vec![4], rating: 5 }], "an unselected tile alone");
    assert_eq!(selected(&harness), [1, 2], "a star click does not change the selection");
    let same = star(&harness, 4, 2.0);
    harness.click_at(same);
    harness.run_steps(8);
    assert_eq!(harness.state_mut().take(), [TileEvent::Rate { keys: vec![4], rating: 0 }], "the star of the current value clears");
}

// Dragging an unselected tile replaces the selection; with Cmd it joins; Drag is reported once.
#[test]
fn a_drag_carries_the_selection_and_joins_the_grabbed_tile() {
    let mut harness = harness(Host::new(8));
    click(&mut harness, 0);
    harness.state_mut().take();
    let (from, to) = (centre(&harness, 3), centre(&harness, 3) + vec2(0.0, 200.0));
    harness.drag(from, to, Modifiers::NONE);
    harness.run_steps(4);
    let events = harness.state_mut().take();
    let drags: Vec<_> = events.iter().filter_map(|e| if let TileEvent::Drag(d) = e { Some(d.items.clone()) } else { None }).collect();
    assert_eq!(drags, [vec![3]], "one Drag, carrying the grabbed tile");
    assert_eq!(selected(&harness), [3], "an unselected tile replaces the selection");

    let (from, to) = (centre(&harness, 5), centre(&harness, 5) + vec2(0.0, 200.0));
    harness.drag(from, to, CMD);
    harness.run_steps(4);
    let events = harness.state_mut().take();
    let drags: Vec<_> = events.iter().filter_map(|e| if let TileEvent::Drag(d) = e { Some(d.items.clone()) } else { None }).collect();
    assert_eq!(drags.len(), 1);
    assert_eq!(selected(&harness), [3, 5], "with Cmd it joins");
    let mut carried = drags[0].clone();
    carried.sort_unstable();
    assert_eq!(carried, [3, 5]);
}

// Begin, then Commit with one Move on release; Esc gives Cancel and no Commit.
#[test]
fn reorder_commits_one_move_and_esc_cancels() {
    let mut host = Host::new(8);
    host.reorder = true;
    let mut harness = harness(host);
    click(&mut harness, 0);
    harness.state_mut().take();
    let from = centre(&harness, 0);
    let to = centre(&harness, 3) + vec2(30.0, 0.0);
    let mut frames = events::drag(from, to, Modifiers::NONE);
    let release = frames.pop().unwrap();
    play(&mut harness, frames);
    let target = Move { items: vec![0], target: 3, position: DropPosition::After, copy: false };
    assert_eq!(harness.state().drop_target, Some(target.clone()), "the output names the gap while dragging");
    play(&mut harness, vec![release]);
    harness.run_steps(4);
    assert_eq!(harness.state().drop_target, None);
    let reorders: Vec<_> =
        harness.state_mut().take().into_iter().filter_map(|e| if let TileEvent::Reorder(r) = e { Some(r) } else { None }).collect();
    assert!(matches!(reorders.first(), Some(EditEvent::Begin { .. })), "{reorders:?}");
    let commits: Vec<_> = reorders.iter().filter_map(|e| if let EditEvent::Commit { value, .. } = e { Some(value.clone()) } else { None }).collect();
    assert_eq!(commits, [target]);

    // Esc halfway through a second drag.
    let from = centre(&harness, 1);
    let to = centre(&harness, 4) + vec2(30.0, 0.0);
    let mut frames = events::drag(from, to, Modifiers::NONE);
    let release = frames.pop().unwrap();
    frames.extend(events::key_press(Key::Escape, Modifiers::NONE));
    frames.push(release);
    play(&mut harness, frames);
    harness.run_steps(4);
    let reorders: Vec<_> =
        harness.state_mut().take().into_iter().filter_map(|e| if let TileEvent::Reorder(r) = e { Some(r) } else { None }).collect();
    assert!(reorders.iter().any(|e| matches!(e, EditEvent::Cancel { .. })), "{reorders:?}");
    assert!(!reorders.iter().any(|e| matches!(e, EditEvent::Commit { .. })), "{reorders:?}");
}

// The selection follows the rectangle live; Esc restores the selection from before.
#[test]
fn marquee_selects_live_and_escape_restores() {
    let mut harness = harness(Host::new(5));
    click(&mut harness, 0);
    let right = tile(&harness, 4).rect();
    let start = pos2(right.right() + 30.0, right.center().y);
    let across = centre(&harness, 2);
    let mut frames = events::drag(start, across, Modifiers::NONE);
    let release = frames.pop().unwrap();
    play(&mut harness, frames);
    assert_eq!(selected(&harness), [2, 3, 4], "the tiles the rectangle touches, live");
    play(&mut harness, events::key_press(Key::Escape, Modifiers::NONE));
    assert_eq!(selected(&harness), [0], "Esc puts the selection back");
    play(&mut harness, vec![release]);
    assert_eq!(selected(&harness), [0]);
}

// Pointer at a quarter of the picture: 0.25; in RTL: 0.75.
#[test]
fn hover_scrub_reports_the_fraction_from_the_reading_start() {
    for (direction, expected) in [(Direction::Ltr, 0.25), (Direction::Rtl, 0.75)] {
        let mut host = Host::new(4);
        host.hover_scrub = true;
        host.direction = direction;
        let mut harness = harness(host);
        let p = picture(&harness, 0);
        harness.state_mut().take();
        harness.event(Event::PointerMoved(pos2(p.left() + 25.0, p.center().y)));
        harness.run_steps(3);
        let fractions: Vec<f32> = harness
            .state_mut()
            .take()
            .into_iter()
            .filter_map(|e| if let TileEvent::Scrub { key: 0, fraction } = e { Some(fraction) } else { None })
            .collect();
        assert!(!fractions.is_empty() && fractions.iter().all(|f| (f - expected).abs() < 0.01), "{direction:?}: {fractions:?}");
    }
}

// LightCraft's `shift_click_sets_the_range_to_the_clicked_state`.
#[test]
fn check_mode_shift_click_sets_the_range_to_the_clicked_state() {
    let mut host = Host::new(8);
    host.check_mode = true;
    let mut harness = harness(host);
    click(&mut harness, 1);
    assert_eq!(selected(&harness), [1], "a click checks");
    click_with(&mut harness, 4, Modifiers::SHIFT);
    assert_eq!(selected(&harness), [1, 2, 3, 4], "Shift checks the range to the new state of the clicked tile");
    click(&mut harness, 2);
    assert_eq!(selected(&harness), [1, 3, 4], "a click unchecks");
    click_with(&mut harness, 4, Modifiers::SHIFT);
    assert_eq!(selected(&harness), [1], "the clicked tile turns off, so the range 2..=4 does");
}

// A disabled tile is skipped by Shift ranges, Cmd/Ctrl+A and arrows, and can't be checked.
#[test]
fn a_disabled_tile_is_skipped_by_ranges_and_keys() {
    let mut host = Host::new(6);
    host.tiles.disabled = vec![3];
    let mut harness = harness(host);
    click(&mut harness, 1);
    click_with(&mut harness, 5, Modifiers::SHIFT);
    assert_eq!(selected(&harness), [1, 2, 4, 5], "a range skips it");
    click(&mut harness, 3);
    assert_eq!(selected(&harness), [1, 2, 4, 5], "a click on it changes nothing");
    key(&mut harness, CMD, Key::A);
    assert_eq!(selected(&harness), [0, 1, 2, 4, 5], "Select All skips it");
    click(&mut harness, 2);
    key(&mut harness, Modifiers::NONE, Key::ArrowRight);
    assert_eq!(harness.state().selection.lead, Some(4), "the arrow passes over it");

    let mut host = Host::new(6);
    host.tiles.disabled = vec![3];
    host.check_mode = true;
    let mut checking = self::harness(host);
    click(&mut checking, 3);
    assert!(checking.state().selection.is_empty(), "it can't be checked");
}

// LightCraft's `a_badge_never_covers_the_checkbox` (`badge_rects`).
#[test]
fn a_badge_never_covers_the_checkbox() {
    let host = harness(Host::new(1));
    let style = style(&host);
    let picture = Rect::from_min_size(pos2(0.0, 0.0), vec2(120.0, 90.0));
    let checkbox = Rect::from_min_size(pos2(style.inset + 2.0, style.inset + 2.0), vec2(style.checkbox, style.checkbox));
    let rects = badge_rects(picture, &[40.0, 30.0], &style, Some(checkbox), true, true);
    assert_eq!(rects.len(), 2);
    for rect in &rects {
        assert!(!rect.intersects(checkbox), "{rect:?} covers {checkbox:?}");
        assert!(picture.contains_rect(*rect));
    }
    assert!(rects[0].top() >= checkbox.bottom(), "the badge moves below the box");
    assert!(rects[1].top() >= rects[0].bottom(), "the next stacks inward");
    // The far corner is unaffected.
    let end = badge_rects(picture, &[40.0], &style, Some(checkbox), false, true);
    assert_eq!(end[0].top(), picture.top() + style.inset);
    assert_eq!(end[0].right(), picture.right() - style.inset);
}

// Scrolling: the pinned header is the top group's; as the next header arrives it pushes it up.
#[test]
fn pinned_group_header_is_pushed_up_by_the_next() {
    let spacing = ArrangeSpacing { gap: 6.0, inset: 4.0, group_header: 40.0, caption: 0.0 };
    let groups: Vec<TileGroup> = [(0, 12), (12, 12), (24, 12)]
        .into_iter()
        .map(|(start, count)| TileGroup { start, count, title: String::new(), detail: String::new() })
        .collect();
    let a = arrange(&[1.0; 36], &groups, 640.0, CARD, spacing);
    let at_top = |top: f32| a.headers.partition_point(|h| h.top() <= top).checked_sub(1);
    let view = |top: f32| Rect::from_min_size(pos2(0.0, top), vec2(640.0, 300.0));

    assert_eq!(pinned_rect(&a.headers, view(0.0), at_top(0.0)), None, "the first header is in view");
    let inside = a.headers[0].bottom() + 20.0;
    let (group, rect) = pinned_rect(&a.headers, view(inside), at_top(inside)).unwrap();
    assert_eq!((group, rect.top()), (0, inside), "pinned at the top while its rows cross it");
    let near = a.headers[1].top() - 10.0;
    let (group, rect) = pinned_rect(&a.headers, view(near), at_top(near)).unwrap();
    assert_eq!(group, 0);
    assert_eq!(rect.bottom(), a.headers[1].top(), "the next header pushes it up");
    let past = a.headers[1].top() + 5.0;
    let (group, rect) = pinned_rect(&a.headers, view(past), at_top(past)).unwrap();
    assert_eq!((group, rect.top()), (1, past), "then the next group's takes over");
}

// Each built tile is a ListBoxOption with position_in_set and size_of_set of the full count.
#[test]
fn tiles_tell_their_position_in_the_whole_set() {
    let harness = harness(Host::new(200));
    assert_eq!(harness.get_by_role(Role::ListBox).accesskit_node().size_of_set(), Some(200));
    let tiles: Vec<_> = harness.query_all_by_role(Role::ListBoxOption).collect();
    assert!(!tiles.is_empty());
    for node in tiles {
        let label = node.accesskit_node().label().unwrap();
        let n: usize = label.trim_start_matches("Tile ").parse().unwrap();
        assert_eq!(node.accesskit_node().position_in_set(), Some(n), "{label}");
        assert_eq!(node.accesskit_node().size_of_set(), Some(200), "{label}");
    }
}

// Group headers select the group's tiles and report it; Shift or Cmd adds to the selection.
#[test]
fn a_group_header_click_selects_the_group() {
    let mut host = Host::new(12);
    host.groups = [(0, 4, "Monday"), (4, 8, "Tuesday")]
        .into_iter()
        .map(|(start, count, title)| TileGroup { start, count, title: title.to_owned(), detail: String::new() })
        .collect();
    let mut harness = harness(host);
    click(&mut harness, 10);
    harness.state_mut().take();
    let header = harness.get_by_label("Monday").rect().center();
    harness.click_at(header);
    harness.run_steps(8);
    assert_eq!(selected(&harness), [0, 1, 2, 3]);
    let events = harness.state_mut().take();
    assert!(events.iter().any(|e| matches!(e, TileEvent::GroupPicked { group: 0, .. })), "{events:?}");

    let header = harness.get_by_label("Tuesday").rect().center();
    harness.click_at_with(header, PointerButton::Primary, CMD);
    harness.run_steps(8);
    assert_eq!(selected(&harness), (0..12).collect::<Vec<_>>(), "Cmd adds the group");
}

// Cmd/Ctrl + / - / 0 and a pinch ask the host for a size step.
#[test]
fn zoom_keys_and_pinch_report_a_zoom_step() {
    use super::ZoomStep;
    let mut host = Host::new(12);
    // With `CommandDigits` Cmd+0 clears a rating (Bridge's rule); Reset needs plain digits.
    host.rating_keys = RatingKeys::Digits;
    let mut harness = harness(host);
    click(&mut harness, 0);
    harness.state_mut().take();
    let zooms = |harness: &mut Harness<'_, Host>| -> Vec<ZoomStep> {
        harness.state_mut().take().into_iter().filter_map(|e| if let TileEvent::Zoom(step) = e { Some(step) } else { None }).collect()
    };
    key(&mut harness, CMD, Key::Equals);
    assert_eq!(zooms(&mut harness), [ZoomStep::In]);
    key(&mut harness, CMD, Key::Minus);
    assert_eq!(zooms(&mut harness), [ZoomStep::Out]);
    key(&mut harness, CMD, Key::Num0);
    assert_eq!(zooms(&mut harness), [ZoomStep::Reset]);
    harness.event(Event::Zoom(1.5));
    harness.run_steps(2);
    assert_eq!(zooms(&mut harness), [ZoomStep::In], "a pinch out");
    harness.event(Event::Zoom(0.6));
    harness.run_steps(2);
    assert_eq!(zooms(&mut harness), [ZoomStep::Out], "a pinch in");
}

fn renames(events: Vec<TileEvent<usize>>) -> Vec<TileEvent<usize>> {
    events.into_iter().filter(|e| matches!(e, TileEvent::Renamed { .. } | TileEvent::RenameEnded { .. })).collect()
}

// F2 opens the lead's rename field in the caption; Enter commits the typed text, Esc cancels.
#[test]
fn f2_renames_the_lead_and_enter_commits_while_escape_cancels() {
    use crate::list::RenameEnd;
    let mut host = Host::new(8);
    host.renamable = true;
    let mut harness = harness(host);
    key(&mut harness, Modifiers::NONE, Key::F2);
    assert!(renames(harness.state_mut().take()).is_empty(), "no lead, no rename");
    click(&mut harness, 2);
    harness.state_mut().take();
    key(&mut harness, Modifiers::NONE, Key::F2);
    harness.run_steps(2);
    harness.event(Event::Text("Sunrise".to_owned()));
    harness.run_steps(2);
    key(&mut harness, Modifiers::NONE, Key::Enter);
    harness.run_steps(2);
    assert_eq!(
        renames(harness.state_mut().take()),
        [TileEvent::Renamed { key: 2, text: "Sunrise".to_owned() }, TileEvent::RenameEnded { key: 2, end: RenameEnd::Enter }]
    );

    key(&mut harness, Modifiers::NONE, Key::F2);
    harness.run_steps(2);
    harness.event(Event::Text("Dusk".to_owned()));
    harness.run_steps(2);
    key(&mut harness, Modifiers::NONE, Key::Escape);
    harness.run_steps(2);
    assert_eq!(renames(harness.state_mut().take()), [TileEvent::RenameEnded { key: 2, end: RenameEnd::Escape }]);
}

// F2 is the host's when the grid does not rename.
#[test]
fn f2_does_nothing_without_a_rename_hook() {
    let mut harness = harness(Host::new(8));
    click(&mut harness, 2);
    harness.state_mut().take();
    key(&mut harness, Modifiers::NONE, Key::F2);
    harness.run_steps(2);
    assert!(harness.query_by_role(Role::TextInput).is_none());
}

// A second click on the caption of the only selected tile opens the rename once the double-click
// interval passes; a click on the picture does not.
#[test]
fn a_slow_click_on_the_caption_renames() {
    let mut host = Host::new(8);
    host.renamable = true;
    let mut harness = harness(host);
    click(&mut harness, 3);
    harness.state_mut().take();
    let caption = picture(&harness, 3).center_bottom() + vec2(0.0, 6.0);
    harness.click_at(caption);
    harness.run_steps(2);
    assert!(harness.query_by_role(Role::TextInput).is_none(), "not before the double-click interval");
    harness.run_steps(12);
    assert!(harness.query_by_role(Role::TextInput).is_some(), "the field opens");

    let mut host = Host::new(8);
    host.renamable = true;
    let mut harness = self::harness(host);
    click(&mut harness, 3);
    let on_picture = picture(&harness, 3).center();
    harness.click_at(on_picture);
    harness.run_steps(16);
    assert!(harness.query_by_role(Role::TextInput).is_none(), "a click on the picture does not rename");
}

// The host's context menu opens a rename with `start_rename`; another open rename commits first.
#[test]
fn start_rename_opens_the_field_of_that_tile() {
    let mut host = Host::new(8);
    host.renamable = true;
    let mut harness = harness(host);
    harness.state_mut().start_rename = Some(4);
    harness.run_steps(2);
    assert!(harness.query_by_role(Role::TextInput).is_some());
}

// The scroll bar is named in the language of the Ui (ScrollBarLabels).
#[test]
fn the_scroll_bar_has_a_name() {
    let harness = harness(Host::new(200));
    assert!(harness.query_by_role(Role::ScrollBar).is_some_and(|n| n.accesskit_node().label().as_deref() == Some("Scroll bar")));
}

// A rating's accessible name uses the count word: "1 star", "3 stars".
#[test]
fn the_accessible_name_counts_stars_with_the_plural_word() {
    use super::TileLabels;
    use super::paint::accessible_name;
    use crate::localized::Labels as _;
    let ctx = egui::Context::default();
    let labels = TileLabels::english();
    let mut content = TileContent { name: "Beach".to_owned(), ..TileContent::default() };
    content.rating = Some(1);
    assert_eq!(accessible_name(&ctx, &content, &labels, None), "Beach, 1 star");
    content.rating = Some(3);
    assert_eq!(accessible_name(&ctx, &content, &labels, None), "Beach, 3 stars");
    content.stack = Some(4);
    content.status = super::TileStatus::Offline("moved".to_owned());
    assert_eq!(accessible_name(&ctx, &content, &labels, None), "Beach, 3 stars, Offline: moved, ×4");
}

// The thumbnail slider derives its id in the ui it is shown in: two with the same salt in two
// scopes do not share an id (they would share the slider's drag state).
#[test]
fn two_thumbnail_sliders_with_one_salt_in_two_scopes_have_two_ids() {
    use super::{ThumbnailSlider, TileZoom};
    let mut ids = Vec::new();
    let mut harness = Harness::builder().with_size([400.0, 200.0]).build_ui(|ui| {
        for scope in ["left", "right"] {
            ui.push_id(scope, |ui| {
                let _ = ThumbnailSlider::new("zoom", 1.0, TileZoom::Ladder).show(ui);
            });
        }
    });
    harness.run();
    let sliders: Vec<_> = harness.query_all_by_role(Role::Slider).collect();
    for node in &sliders {
        ids.push(node.accesskit_node().id());
    }
    assert_eq!(ids.len(), 2);
    assert_ne!(ids[0], ids[1]);
}

// The strip parts (craft-pages' thumbnails): `current` draws the current ring on a tile that is not
// selected, marks its option `aria_current`, and a new current tile scrolls into view.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn current_tile_is_ringed_and_revealed() {
    todo!()
}

// `CaptionPlace::Start`: the name sits beside the picture at the reading start (the right in RTL),
// a hidden tile's name has the slash and its picture the veil, and a mark's tip joins the accessible name.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn start_captions_show_names_marks_and_hidden_tiles() {
    todo!()
}

// `group_disclosure`: a chevron click reports GroupOpen without picking the group; a closed
// group's tiles are skipped by Down and absent from AccessKit; `group_menus` reports
// GroupContextMenu on a secondary click on a header.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn closed_groups_hide_their_tiles_and_headers_report_menus() {
    todo!()
}

// `new_item_row`: a click, and Enter after Tab from the grid, report NewItem.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn new_item_row_reports_new_item() {
    todo!()
}

// `file_drops`: a hovered file over the gap after tile 2 reports `file_gap == Some(3)`; none without
// a hovered file.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn file_drops_report_the_gap() {
    todo!()
}
