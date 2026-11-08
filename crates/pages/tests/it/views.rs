//! The page widgets of craft-pages: view state and layout, synced views, grids, thumbnails, the list and the
//! preview.

use craft_behavior::{Selection, kurbo, kurbo::Size};
use craft_pages::layout::{Fit, LayoutOptions, ZOOM_RANGE};
use craft_pages::ui::fixtures::{DemoPages, PAPER};
use craft_pages::ui::{
    Lead, PageList, PageListEvent, PageListRow, PagePreview, PageSource, PageThumbnails, PageView, PageViewState, SyncedViews, SyncedViewsState,
    ThumbnailEvent, baseline_lines, moved_page,
};

use craft_pages::ui::reading::{ReadingState, ReadingView};
use craft_ui::contract::{ReorderMove, Role, Show as _};
use craft_ui::primitives::textures::{Revision, ThumbRender, ThumbRequest, ThumbSource};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, Pos2, Rect, pos2, vec2};
use std::cell::RefCell;
use std::rc::Rc;

/// The most baseline lines a page draws.
const MOST_BASELINES: usize = craft_pages::ui::grid::MAX_BASELINES;

#[trial]
fn baseline_lines_start_inside_the_page() {
    assert_eq!(baseline_lines(-30.0, 12.0, 50.0), [6.0, 18.0, 30.0, 42.0]);
    assert_eq!(baseline_lines(0.0, 0.2, 3.5), [0.0, 1.0, 2.0, 3.0], "an increment under 1 pt counts as 1");
    assert_eq!(baseline_lines(0.0, 1.0, 1.0e9).len(), MOST_BASELINES);
    assert!(baseline_lines(f64::NAN, 12.0, 50.0).is_empty());
}

#[trial]
fn baseline_lines_end_before_the_bottom_edge() {
    assert_eq!(baseline_lines(0.0, 10.0, 30.0), [0.0, 10.0, 20.0], "the bottom edge is not a line");
}

/// The default state with `change` applied.
fn state_with(change: impl FnOnce(&mut PageViewState)) -> PageViewState {
    let mut state = PageViewState::default();
    change(&mut state);
    state
}

#[trial]
fn zoom_steps_start_from_the_shown_zoom_and_end_a_fit() {
    let mut state = state_with(|s| {
        s.fit = Fit::Width;
        s.zoom = 0.9;
    });
    state.zoom_step(true);
    assert_eq!(state.fit, Fit::None);
    assert!(state.zoom > 0.9, "in goes up from the shown zoom, got {}", state.zoom);
    let up = state.zoom;
    state.zoom_step(false);
    assert!(state.zoom < up);
    state.zoom = *ZOOM_RANGE.end();
    state.zoom_step(true);
    assert_eq!(state.zoom, *ZOOM_RANGE.end(), "the top of the range stays");
}

#[trial]
fn going_to_a_page_makes_it_current_at_once() {
    let mut state = PageViewState::default();
    state.go_to(4);
    assert_eq!(state.current, 4);
    assert_eq!(state.go_to, Some((4, 0.0)));
}

/// A page view over `count` Letter pages in a room of 400 by 300, with the state the test starts from.
struct ViewRig {
    fixture: Fixture,
    state: Rc<RefCell<PageViewState>>,
    shown: Rc<RefCell<Vec<usize>>>,
}

fn view_rig(count: usize, start: PageViewState) -> ViewRig {
    let state = Rc::new(RefCell::new(start));
    let shown: Rc<RefCell<Vec<usize>>> = Rc::default();
    let (kept, seen) = (Rc::clone(&state), Rc::clone(&shown));
    let pages = DemoPages::new(count);
    let mut fixture = Fixture::new(move |ui| {
        let mut now = kept.borrow().clone();
        let out = PageView::new("Pages", &pages, &mut now, PAPER).options(LayoutOptions::ACROBAT).show(ui, |_, _| ());
        *seen.borrow_mut() = out.pages.iter().map(|placed| placed.page).collect();
        *kept.borrow_mut() = now;
    });
    fixture.room(vec2(400.0, 300.0));
    fixture.settle();
    ViewRig { fixture, state, shown }
}

#[trial]
fn fit_width_keeps_the_zoom_equal_to_what_shows() {
    let rig = view_rig(
        3,
        state_with(|s| {
            s.fit = Fit::Width;
            s.zoom = 7.0;
        }),
    );
    let zoom = rig.state.borrow().zoom;
    assert!(zoom < 7.0 && zoom > 0.0, "the fit wrote its zoom back, got {zoom}");
}

#[trial]
fn only_the_row_of_the_current_page_shows_when_not_continuous() {
    let rig = view_rig(
        5,
        state_with(|s| {
            s.continuous = false;
            s.current = 2;
            s.fit = Fit::Page;
        }),
    );
    assert_eq!(*rig.shown.borrow(), [2]);
}

#[trial]
fn the_current_page_follows_the_middle_line() {
    let mut rig = view_rig(6, state_with(|s| s.fit = Fit::Width));
    rig.state.borrow_mut().go_to(3);
    rig.fixture.idle(3);
    let state = rig.state.borrow();
    assert_eq!(state.current, 3, "the page brought to the top is the current one");
    assert!(state.offset.1 > 0.0, "the view scrolled to it, offset {:?}", state.offset);
}

/// The pair's pages are Letter-sized: two halves of one document.
#[trial]
fn the_follower_takes_the_leaders_zoom_and_layout() {
    let states = Rc::new(RefCell::new((SyncedViewsState::default(), PageViewState::default(), PageViewState::default())));
    let kept = Rc::clone(&states);
    let (old_pages, new_pages) = (DemoPages::new(3), DemoPages::new(3));
    let mut fixture = Fixture::new(move |ui| {
        let mut now = kept.borrow().clone();
        let (pair, left, right) = (&mut now.0, &mut now.1, &mut now.2);
        SyncedViews::new("Compare", pair).halves(ui, (left, right), |ui, rect, state, half| match half {
            Lead::Left => PageView::new("Old", &old_pages, state, PAPER).show_at(ui, rect, |_, _| ()),
            _ => PageView::new("New", &new_pages, state, PAPER).show_at(ui, rect, |_, _| ()),
        });
        *kept.borrow_mut() = now;
    });
    fixture.room(vec2(600.0, 300.0));
    fixture.settle();
    states.borrow_mut().2.fit = Fit::None;
    states.borrow_mut().2.zoom = 2.0;
    states.borrow_mut().2.continuous = false;
    fixture.idle(3);
    let state = states.borrow();
    assert_eq!(state.0.lead, Lead::Right, "the main view leads at first");
    assert_eq!(state.1.zoom, state.2.zoom, "the follower shows the leader's zoom");
    assert_eq!(state.1.continuous, state.2.continuous, "and its layout");
}

#[trial]
fn a_page_moves_to_where_a_reorder_puts_it() {
    // Pages 1 and 2 go to position 3 of the pages that remain: 0 3 4 | 1 2 | 5.
    let mv = ReorderMove::new(1, vec![1, 2], 3, false);
    let after: Vec<usize> = (0..6).map(|page| moved_page(&mv, page, 6)).collect();
    assert_eq!(after, [0, 3, 4, 1, 2, 5]);
    assert_eq!(moved_page(&mv, 9, 6), 9, "a page past the end stays");
}

#[trial]
fn moving_a_page_to_the_front_pushes_the_rest_back() {
    let mv = ReorderMove::new(3, vec![3], 0, false);
    let after: Vec<usize> = (0..5).map(|page| moved_page(&mv, page, 5)).collect();
    assert_eq!(after, [1, 2, 3, 0, 4]);
}

/// Thumbnails of `count` pages with the events of every pass collected.
struct StripRig {
    fixture: Fixture,
    picked: Rc<RefCell<Selection<usize>>>,
    events: Rc<RefCell<Vec<ThumbnailEvent>>>,
}

fn strip_rig(count: usize, picked: Selection<usize>) -> StripRig {
    let picked = Rc::new(RefCell::new(picked));
    let events: Rc<RefCell<Vec<ThumbnailEvent>>> = Rc::default();
    let (kept, seen) = (Rc::clone(&picked), Rc::clone(&events));
    let pages = DemoPages::new(count);
    let mut fixture = Fixture::new(move |ui| {
        let mut now = kept.borrow().clone();
        let out = PageThumbnails::new("Pages", &pages, &mut now, 0).show(ui);
        seen.borrow_mut().extend(out.events);
        *kept.borrow_mut() = now;
    });
    fixture.room(vec2(400.0, 500.0));
    fixture.settle();
    StripRig { fixture, picked, events }
}

#[trial]
fn clicking_a_thumbnail_goes_to_its_page() {
    let mut rig = strip_rig(4, Selection::single(0));
    let tile = rig.fixture.by_label("Page 3").expect("the third page's tile is in the tree");
    rig.fixture.click(&tile);
    assert!(rig.events.borrow().contains(&ThumbnailEvent::Go(2)), "events {:?}", rig.events.borrow());
}

#[trial]
fn delete_with_nothing_picked_removes_the_current_page_but_never_every_page() {
    let mut rig = strip_rig(1, Selection::new(Vec::new(), None, None));
    rig.fixture.key(Key::Delete, Modifiers::NONE);
    assert!(!rig.events.borrow().iter().any(|event| matches!(event, ThumbnailEvent::Delete(_))), "a document keeps one page");
    assert!(rig.picked.borrow().items.is_empty());
}

/// A list of three boards, the second locked.
struct ListRig {
    fixture: Fixture,
    events: Rc<RefCell<Vec<PageListEvent>>>,
}

fn list_rig() -> ListRig {
    let events: Rc<RefCell<Vec<PageListEvent>>> = Rc::default();
    let seen = Rc::clone(&events);
    let picked = Rc::new(RefCell::new(Selection::single(0_usize)));
    let rows = vec![PageListRow::new("Board 1"), PageListRow::new("Board 2").locked(true), PageListRow::new("Board 3").hidden(true)];
    let mut fixture = Fixture::new(move |ui| {
        let mut now = picked.borrow().clone();
        let out = PageList::new("Boards", &rows, &mut now, 0).show(ui);
        seen.borrow_mut().extend(out.events);
        *picked.borrow_mut() = now;
    });
    fixture.room(vec2(320.0, 300.0));
    fixture.settle();
    ListRig { fixture, events }
}

#[trial]
fn a_click_on_a_row_makes_it_the_active_one() {
    let mut rig = list_rig();
    let row = rig.fixture.by_label("Board 3").expect("the row is in the tree");
    rig.fixture.click(&row);
    assert!(rig.events.borrow().contains(&PageListEvent::Go(2)), "events {:?}", rig.events.borrow());
}

#[trial]
fn a_name_typed_in_place_is_trimmed_and_never_empty() {
    let mut rig = list_rig();
    let row = rig.fixture.by_label("Board 1").expect("the row is in the tree");
    rig.fixture.click(&row);
    rig.fixture.key(Key::F2, Modifiers::NONE);
    rig.fixture.type_text("  Cover  ");
    rig.fixture.key(Key::Enter, Modifiers::NONE);
    assert!(
        rig.events.borrow().iter().any(|event| matches!(event, PageListEvent::Rename { row: 0, text, .. } if text == "Cover")),
        "events {:?}",
        rig.events.borrow()
    );
}

/// The preview of five pages, started at page `start`.
struct PreviewRig {
    fixture: Fixture,
    at: Rc<RefCell<usize>>,
}

fn preview_rig(start: usize) -> PreviewRig {
    let at = Rc::new(RefCell::new(start));
    let kept = Rc::clone(&at);
    let pages = DemoPages::new(5);
    let shown = [0, 1, 2, 3, 4];
    let mut fixture = Fixture::new(move |ui| {
        let mut now = *kept.borrow();
        PagePreview::new(&pages, &shown, &mut now, PAPER).show(ui);
        *kept.borrow_mut() = now;
    });
    fixture.room(vec2(500.0, 500.0));
    fixture.settle();
    PreviewRig { fixture, at }
}

#[trial]
fn the_preview_steps_with_its_buttons_and_stops_at_the_ends() {
    let mut rig = preview_rig(0);
    let next = rig.fixture.by_label("Next page").expect("the next button is in the tree");
    rig.fixture.click(&next);
    assert_eq!(*rig.at.borrow(), 1);
    let previous = rig.fixture.by_label("Previous page").expect("the previous button is in the tree");
    rig.fixture.click(&previous);
    rig.fixture.click(&previous);
    assert_eq!(*rig.at.borrow(), 0, "the first page has no page before it");
    let mut last = preview_rig(4);
    let next = last.fixture.by_label("Next page").expect("the next button is in the tree");
    last.fixture.click(&next);
    assert_eq!(*last.at.borrow(), 4, "the last page has no page after it");
}

/// The page size the demo source reports, for the reader of the trials above.
#[trial]
fn the_demo_pages_are_letter_sized() {
    assert_eq!(DemoPages::new(2).page_size(1), Size::new(612.0, 792.0));
}

/// Three columns of the demo pages with a margin each side, in a room of known geometry.
struct ReadingRig {
    fixture: Fixture,
    state: Rc<RefCell<ReadingState>>,
    /// The width of one column.
    column: f32,
}

/// Columns shown side by side in a [`ReadingRig`].
const READING_COLUMNS: usize = 3;
/// The width left for the two side margins and the gaps, screen points.
const READING_LEFTOVER: f32 = 300.0;
/// The pages in a reading rig.
const READING_PAGES: usize = 9;

fn reading_rig(start: usize) -> ReadingRig {
    let column = DemoPages::new(1).page_size(0).width as f32 * ReadingView::screen_scale(1.0);
    let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(READING_COLUMNS as f32 * column + READING_LEFTOVER, 700.0));
    let state = Rc::new(RefCell::new(ReadingState::default()));
    state.borrow_mut().column = start;
    let kept = Rc::clone(&state);
    let pages = DemoPages::new(READING_PAGES);
    let mut fixture = Fixture::new(move |ui| {
        let mut now = kept.borrow().clone();
        ReadingView::new("Document", &pages, &mut now, PAPER).show_at(ui, rect);
        *kept.borrow_mut() = now;
    });
    fixture.room(rect.size());
    fixture.settle();
    ReadingRig { fixture, state, column }
}

impl ReadingRig {
    /// The centres of the page-turn buttons the view exposes, left to right: the left margin, the gaps between the
    /// columns, the right margin.
    fn turns(&self) -> Vec<Pos2> {
        let mut centres: Vec<Pos2> = self.fixture.by_role(Role::BUTTON).iter().map(|target| target.center()).collect();
        centres.sort_by(|a, b| a.x.total_cmp(&b.x));
        centres
    }

    /// The middle of the middle column, between the two gaps around it.
    fn middle_column(&self) -> Pos2 {
        let turns = self.turns();
        pos2((turns[1].x + turns[2].x) / 2.0, turns[1].y)
    }

    fn at(&self) -> usize {
        self.state.borrow().column
    }
}

#[trial]
fn a_click_inside_a_column_leaves_the_reading_view_where_it_is() {
    let mut rig = reading_rig(2);
    let inside = rig.middle_column();
    rig.fixture.click_at(inside);
    assert_eq!(rig.at(), 2, "the click is the app's, so the text can be selected");
}

#[trial]
fn a_click_in_a_gap_or_a_side_margin_turns_toward_that_side() {
    let mut rig = reading_rig(2);
    let turns = rig.turns();
    assert_eq!(turns.len(), READING_COLUMNS + 1, "a part over each gap and each margin");
    rig.fixture.click_at(turns[0]);
    assert_eq!(rig.at(), 1, "the left margin turns back");
    rig.fixture.click_at(turns[2]);
    assert_eq!(rig.at(), 2, "a gap right of the middle turns forward");
    rig.fixture.click_at(turns[1]);
    assert_eq!(rig.at(), 1, "a gap left of the middle turns back");
    rig.fixture.click_at(turns[3]);
    assert_eq!(rig.at(), 2, "the right margin turns forward");
}

#[trial]
fn a_drag_inside_a_column_does_not_turn_but_a_swipe_from_a_margin_does() {
    let mut rig = reading_rig(2);
    let turns = rig.turns();
    let along = vec2(-rig.column / 2.0, 0.0);
    let inside = rig.middle_column();
    rig.fixture.drag_at(inside, along);
    assert_eq!(rig.at(), 2, "a drag in a column is a text selection");
    rig.fixture.drag_at(turns[3], along);
    assert_eq!(rig.at(), 3, "a swipe to the left turns forward");
    rig.fixture.drag_at(turns[0], -along);
    assert_eq!(rig.at(), 2, "a swipe to the right turns back");
}

#[trial]
fn space_turns_the_reading_view_forward() {
    let mut rig = reading_rig(0);
    rig.fixture.key(Key::Tab, Modifiers::NONE);
    rig.fixture.key(Key::Space, Modifiers::NONE);
    assert_eq!(rig.at(), 1);
}

/// Pages whose pictures never arrive: only the paper under them is painted.
struct BlankPages(DemoPages);

impl ThumbSource<usize> for BlankPages {
    fn revision(&self, key: &usize, request: &ThumbRequest) -> Revision {
        self.0.revision(key, request)
    }

    fn render(&self, _: &usize, _: &ThumbRequest) -> ThumbRender {
        ThumbRender::Blank
    }
}

impl PageSource for BlankPages {
    fn page_count(&self) -> usize {
        self.0.page_count()
    }

    fn page_size(&self, page: usize) -> Size {
        self.0.page_size(page)
    }
}

/// Whether a painted shape is as large as `size`, within a point.
fn paints_shape_of(fixture: &Fixture, size: egui::Vec2) -> bool {
    fixture.shape_bounds().iter().any(|bounds| (bounds.size() - size).abs().max_elem() < 1.0)
}

#[trial]
fn the_reading_view_paints_the_paper_under_a_page_without_a_picture() {
    let scale = ReadingView::screen_scale(1.0);
    let page = vec2(612.0, 792.0) * scale;
    let room = Rect::from_min_size(pos2(0.0, 0.0), page + vec2(300.0, 200.0));
    let pages = BlankPages(DemoPages::new(2));
    let mut state = ReadingState::default();
    let mut fixture = Fixture::new(move |ui| {
        ReadingView::new("Document", &pages, &mut state, PAPER).show_at(ui, room);
    });
    fixture.room(room.size());
    fixture.settle();
    assert!(paints_shape_of(&fixture, page), "a shape the size of the page is the paper");
}

/// The share of the room's height a shape must reach to be the page and not a control.
const LARGE_SHARE: f32 = 0.5;

#[trial]
fn the_preview_paints_the_paper_under_a_page_without_a_picture() {
    let pages = BlankPages(DemoPages::new(2));
    let shown = [0, 1];
    let mut at = 0;
    let mut fixture = Fixture::new(move |ui| {
        PagePreview::new(&pages, &shown, &mut at, PAPER).show(ui);
    });
    fixture.room(vec2(500.0, 500.0));
    fixture.settle();
    let page = DemoPages::new(1).page_size(0);
    let aspect = (page.width / page.height) as f32;
    let room = fixture.shown().height() * LARGE_SHARE;
    assert!(
        fixture.shape_bounds().iter().any(|bounds| bounds.height() > room && (bounds.width() / bounds.height() - aspect).abs() < 0.01),
        "a page-shaped fill is the paper: {:?}",
        fixture.shape_bounds()
    );
}

/// A page view over `count` demo pages with the state it starts from, and the rects of the pages it laid out.
fn layout_rig(count: usize, start: PageViewState) -> (Fixture, Rc<RefCell<PageViewState>>, Rc<RefCell<Vec<kurbo::Rect>>>) {
    let state = Rc::new(RefCell::new(start));
    let rects: Rc<RefCell<Vec<kurbo::Rect>>> = Rc::default();
    let (kept, seen) = (Rc::clone(&state), Rc::clone(&rects));
    let pages = DemoPages::new(count);
    let mut fixture = Fixture::new(move |ui| {
        let mut now = kept.borrow().clone();
        let out = PageView::new("Pages", &pages, &mut now, PAPER).options(LayoutOptions::ACROBAT).show(ui, |_, _| ());
        seen.borrow_mut().clone_from(&out.layout.rects);
        *kept.borrow_mut() = now;
    });
    fixture.room(vec2(900.0, 600.0));
    fixture.settle();
    (fixture, state, rects)
}

#[trial]
fn a_changed_zoom_keeps_the_page_at_the_top_in_place() {
    let (mut fixture, state, rects) = layout_rig(
        5,
        state_with(|s| {
            s.fit = Fit::None;
            s.zoom = 1.0;
        }),
    );
    let third = rects.borrow()[2];
    state.borrow_mut().offset.1 = third.y0 + 0.4 * third.height();
    fixture.idle(2);
    state.borrow_mut().zoom = 2.0;
    fixture.idle(3);
    let third = rects.borrow()[2];
    let offset = state.borrow().offset.1;
    assert!((offset - (third.y0 + 0.4 * third.height())).abs() < 1.0, "offset {offset}, page {third:?}");
}

/// A wheel step of `lines` over the middle of the room.
fn wheel(fixture: &mut Fixture, lines: f32) {
    let middle = pos2(450.0, 300.0);
    fixture.frame(vec![egui::Event::PointerMoved(middle)]);
    fixture.frame(vec![egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: vec2(0.0, lines),
        modifiers: Modifiers::NONE,
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
    }]);
}

#[trial]
fn wheel_gathered_past_the_bottom_is_forgotten_by_a_pass_without_wheel_input() {
    let (mut fixture, state, _) = layout_rig(
        4,
        state_with(|s| {
            s.continuous = false;
            s.fit = Fit::None;
            s.zoom = 2.0;
        }),
    );
    state.borrow_mut().offset.1 = 1e9;
    fixture.idle(2);
    wheel(&mut fixture, -0.5);
    fixture.idle(1);
    wheel(&mut fixture, -0.5);
    fixture.idle(1);
    assert_eq!(state.borrow().current, 0, "each half notch alone is not a turn");
    wheel(&mut fixture, -3.0);
    assert_eq!(state.borrow().current, 1, "a whole notch at once is");
}

#[trial]
fn the_follower_takes_the_leaders_rotation() {
    let states = Rc::new(RefCell::new((SyncedViewsState::default(), PageViewState::default(), PageViewState::default())));
    let kept = Rc::clone(&states);
    let pages = DemoPages::new(3);
    let mut fixture = Fixture::new(move |ui| {
        let mut now = kept.borrow().clone();
        let (pair, left, right) = (&mut now.0, &mut now.1, &mut now.2);
        SyncedViews::new("Compare", pair)
            .halves(ui, (left, right), |ui, rect, state, _| PageView::new("Pages", &pages, state, PAPER).show_at(ui, rect, |_, _| ()));
        *kept.borrow_mut() = now;
    });
    fixture.room(vec2(600.0, 300.0));
    fixture.settle();
    states.borrow_mut().2.rotation = 90;
    fixture.idle(3);
    assert_eq!(states.borrow().1.rotation, 90, "the follower turns with the leader");
}
