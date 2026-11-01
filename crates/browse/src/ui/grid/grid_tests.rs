//! `TileGrid` behaviour through egui_kittest (task B6).

use std::cell::RefCell;
use std::collections::BTreeSet;

use craft_ui::behavior::{Axis, DropPosition, Move, Reorder, Selection};
use craft_ui::canvas::textures::{Revision, ThumbRender, ThumbRequest, ThumbSource, ThumbnailOptions, Thumbnails};
use craft_ui::controls::{ControlSize, Rating};
use craft_ui::direction::{self, Direction};
use craft_ui::edit::EditEvent;
use craft_ui::testing::{HarnessExt as _, events};
use craft_ui::theme::{Base, current, install};
use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect, Ui, pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, Node};

use super::groups::pinned_rect;
use super::layout::{ArrangeSpacing, arrange};
use super::paint::badge_rects;
use super::{BadgeVisibility, RatingKeys, SpaceAction, TileContent, TileEvent, TileGrid, TileGroup, TileLayout, TileSource, TileStyle};

/// Cmd on a Mac and Ctrl elsewhere, as a platform reports it: `ctrl` with the abstract `command`.
const CMD: Modifiers = Modifiers { alt: false, ctrl: true, shift: false, mac_cmd: false, command: true };

/// Cards of this side with a caption: six to a row in the 640 point harness.
const CARD: TileLayout = TileLayout::Cards { side: 100.0, aspect: 1.0, caption: true };

/// The fake tiles: keys are positions, names `Tile {n}`.
struct Tiles {
    count: usize,
    keys: Vec<usize>,
    aspects: Vec<f32>,
    rated: Option<u8>,
    disabled: Vec<usize>,
    asked: RefCell<BTreeSet<usize>>,
}

impl Tiles {
    fn new(count: usize) -> Self {
        Self { count, keys: (0..count).collect(), aspects: Vec::new(), rated: None, disabled: Vec::new(), asked: RefCell::default() }
    }
}

impl craft_ui::behavior::DisplayOrder<usize> for Tiles {
    fn len(&self) -> usize {
        self.count
    }

    fn key(&self, at: usize) -> &usize {
        &self.keys[at]
    }
}

impl TileSource<usize> for Tiles {
    fn aspect(&self, at: usize) -> f32 {
        self.aspects.get(at).copied().unwrap_or(1.0)
    }

    fn content(&self, at: usize, content: &mut TileContent) {
        self.asked.borrow_mut().insert(at);
        content.name = format!("Tile {at}");
        content.rating = self.rated;
        content.enabled = !self.disabled.contains(&at);
    }
}

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
    tiles: Tiles,
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
}

impl Host {
    fn new(count: usize) -> Self {
        Self {
            tiles: Tiles::new(count),
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
        }
    }

    fn take(&mut self) -> Vec<TileEvent<usize>> {
        std::mem::take(&mut self.events)
    }
}

fn show(ui: &mut Ui, host: &mut Host) {
    let thumbnails = Thumbnails::<usize>::in_context(ui.ctx(), "pictures", ThumbnailOptions::default());
    let Host { tiles, pictures, selection, events, layout, groups, badges, check_mode, space, rating_keys, hover_scrub, reorder, .. } = host;
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
    events.extend(grid.show(ui).events);
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
    TileStyle::of(&current(&harness.ctx))
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
    host.tiles.aspects = (0..24).map(|i| if i < 12 { 0.6 } else { 2.0 }).collect();
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
    let (tile_style, step) = (TileStyle::of(&tokens), Rating::style(&tokens, ControlSize::Sm).badge_step);
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
    harness.drag(from, to, Modifiers::NONE);
    harness.run_steps(4);
    let reorders: Vec<_> =
        harness.state_mut().take().into_iter().filter_map(|e| if let TileEvent::Reorder(r) = e { Some(r) } else { None }).collect();
    assert!(matches!(reorders.first(), Some(EditEvent::Begin { .. })), "{reorders:?}");
    let commits: Vec<_> = reorders.iter().filter_map(|e| if let EditEvent::Commit { value, .. } = e { Some(value.clone()) } else { None }).collect();
    assert_eq!(commits, [Move { items: vec![0], target: 3, position: DropPosition::After, copy: false }]);

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
