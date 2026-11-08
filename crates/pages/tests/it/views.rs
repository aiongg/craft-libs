//! The page widgets of craft-pages: view state and layout, synced views, grids, thumbnails, the list and the
//! preview.

use craft_behavior::{Selection, kurbo::Size};
use craft_pages::layout::{Fit, LayoutOptions, ZOOM_RANGE};
use craft_pages::ui::fixtures::{DemoPages, PAPER};
use craft_pages::ui::{
    Lead, PageList, PageListEvent, PageListRow, PagePreview, PageSource, PageThumbnails, PageView, PageViewState, SyncedViews, SyncedViewsState,
    ThumbnailEvent, baseline_lines, moved_page,
};

use craft_ui::contract::ReorderMove;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, vec2};
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
        PagePreview::new(&pages, &shown, &mut now).show(ui);
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
