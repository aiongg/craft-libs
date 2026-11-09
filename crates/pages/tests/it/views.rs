//! The page widgets of craft-pages: view state and layout, synced views, grids, thumbnails, the list and the
//! preview.

use craft_behavior::{Selection, kurbo, kurbo::Size};
use craft_pages::layout::{Fit, LayoutOptions};
use craft_pages::ui::fixtures::{DemoPages, PAPER};
use craft_pages::ui::{
    PageList, PageListEvent, PageListRow, PagePreview, PageSource, PageThumbnails, PageView, PageViewState, ThumbnailEvent, baseline_lines,
    moved_page,
};

use craft_pages::ui::reading::{ReadingState, ReadingView};
use craft_ui::contract::{ReorderMove, Role, Show as _, SweepStep};
use craft_ui::primitives::textures::{LoadFailure, Revision, ThumbRender, ThumbRequest, ThumbSource};
use craft_ui::style::DataColor;
use craft_ui::testing::{Fixture, Frames, pointer, trial};
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

/// The default state with `change` applied.
fn state_with(change: impl FnOnce(&mut PageViewState)) -> PageViewState {
    let mut state = PageViewState::default();
    change(&mut state);
    state
}

/// A page view over `count` Letter pages in a room of 400 by 300, with the state the test starts from.
struct ViewRig {
    fixture: Fixture,
    state: Rc<RefCell<PageViewState>>,
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
    ViewRig { fixture, state }
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

#[trial]
fn a_page_moves_to_where_a_reorder_puts_it() {
    // Pages 1 and 2 go to position 3 of the pages that remain: 0 3 4 | 1 2 | 5.
    let mv = ReorderMove::new(1, vec![1, 2], 3, false);
    let after: Vec<usize> = (0..6).map(|page| moved_page(&mv, page, 6)).collect();
    assert_eq!(after, [0, 3, 4, 1, 2, 5]);
    assert_eq!(moved_page(&mv, 9, 6), 9, "a page past the end stays");
}

/// Thumbnails of `count` pages with the events of every pass collected.
struct StripRig {
    fixture: Fixture,
    events: Rc<RefCell<Vec<ThumbnailEvent>>>,
}

fn strip_rig(count: usize, picked: Selection<usize>) -> StripRig {
    let picked = Rc::new(RefCell::new(picked));
    let events: Rc<RefCell<Vec<ThumbnailEvent>>> = Rc::default();
    let (kept, seen) = (Rc::clone(&picked), Rc::clone(&events));
    let pages = DemoPages::new(count);
    let mut fixture = Fixture::new(move |ui| {
        let mut now = kept.borrow().clone();
        let out = PageThumbnails::new("Pages", &pages, &mut now, 0, PAPER).show(ui);
        seen.borrow_mut().extend(out.events);
        *kept.borrow_mut() = now;
    });
    fixture.room(vec2(400.0, 500.0));
    fixture.settle();
    StripRig { fixture, events }
}

#[trial]
fn delete_removes_the_picked_pages_but_never_all_of_them() {
    let mut rig = strip_rig(4, Selection::new(vec![1, 2], Some(2), Some(1)));
    rig.fixture.key(Key::Tab, Modifiers::NONE);
    rig.fixture.key(Key::Delete, Modifiers::NONE);
    assert!(rig.events.borrow().contains(&ThumbnailEvent::Delete(vec![1, 2])), "the picked pages go: {:?}", rig.events.borrow());
    let mut rig = strip_rig(2, Selection::new(vec![0, 1], Some(1), Some(0)));
    rig.fixture.key(Key::Tab, Modifiers::NONE);
    rig.fixture.key(Key::Delete, Modifiers::NONE);
    assert!(!rig.events.borrow().iter().any(|event| matches!(event, ThumbnailEvent::Delete(_))), "a document keeps one page");
}

/// A list of three boards, the second locked.
struct ListRig {
    fixture: Fixture,
    events: Rc<RefCell<Vec<PageListEvent>>>,
}

fn list_rig() -> ListRig {
    list_rig_picking(Selection::single(0_usize))
}

fn list_rig_picking(start: Selection<usize>) -> ListRig {
    let events: Rc<RefCell<Vec<PageListEvent>>> = Rc::default();
    let seen = Rc::clone(&events);
    let picked = Rc::new(RefCell::new(start));
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

/// An Alt sweep down the eyes reports Show Only once, for the pressed row, and nothing for the rows it crosses; Escape
/// reports the undo of it.
#[trial]
fn an_alt_eye_sweep_reports_show_only_then_its_undo() {
    let mut rig = list_rig();
    let from = rig.fixture.by_label("Visibility Board 1").expect("the eye").center();
    let to = rig.fixture.by_label("Visibility Board 3").expect("the eye").center();
    rig.fixture.hold(Modifiers::ALT);
    rig.fixture.frame(vec![pointer::moved(from)]);
    rig.fixture.frame(vec![pointer::press(from)]);
    rig.fixture.frame(vec![pointer::moved(from + vec2(0.0, 8.0))]);
    rig.fixture.frame(vec![pointer::moved(to)]);
    rig.fixture.idle(2);
    let shown: Vec<_> = rig.events.borrow_mut().drain(..).collect();
    assert!(matches!(shown.as_slice(), [PageListEvent::Toggle(SweepStep::ShowOnly(cell))] if cell.key == 0), "events {shown:?}");
    rig.fixture.key(Key::Escape, Modifiers::ALT);
    rig.fixture.idle(2);
    let undone: Vec<_> = rig.events.borrow_mut().drain(..).collect();
    assert!(matches!(undone.as_slice(), [PageListEvent::Toggle(SweepStep::UndoShowOnly(cell))] if cell.key == 0), "events {undone:?}");
}

#[trial]
fn delete_skips_the_locked_rows() {
    let mut rig = list_rig_picking(Selection::new(vec![0, 1], Some(1), Some(0)));
    rig.fixture.key(Key::Tab, Modifiers::NONE);
    rig.fixture.key(Key::Delete, Modifiers::NONE);
    assert!(rig.events.borrow().contains(&PageListEvent::Delete(vec![0])), "the unlocked picked row goes: {:?}", rig.events.borrow());
    let mut rig = list_rig_picking(Selection::single(1));
    rig.fixture.key(Key::Tab, Modifiers::NONE);
    rig.fixture.key(Key::Delete, Modifiers::NONE);
    assert!(!rig.events.borrow().iter().any(|event| matches!(event, PageListEvent::Delete(_))), "a locked row stays");
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

/// How far a painted fill may stray from the exact page shape: shapes lie on device pixels, so a side is off by at most a point.
const PAPER_SLACK: f32 = 1.0;

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
    let picture = fixture.by_label(&DemoPages::new(1).page_label(0)).expect("the page picture").rect;
    // The paper is the fill of the page's aspect that lies in the picture.
    let paper = |bounds: &Rect| {
        picture.expand(PAPER_SLACK).contains_rect(*bounds)
            && (f64::from(bounds.width()) - f64::from(bounds.height()) * page.width / page.height).abs() < f64::from(PAPER_SLACK)
    };
    assert!(fixture.shape_bounds().iter().any(paper), "a fill with the page's aspect inside the picture is the paper: {:?}", fixture.shape_bounds());
}

/// Pages whose pictures never arrive: each render is a job that waits for a signal the source keeps and never sends.
struct LoadingPages {
    pages: DemoPages,
    held: std::sync::Mutex<Vec<std::sync::mpsc::Sender<()>>>,
}

impl ThumbSource<usize> for LoadingPages {
    fn revision(&self, key: &usize, request: &ThumbRequest) -> Revision {
        self.pages.revision(key, request)
    }

    fn render(&self, _: &usize, _: &ThumbRequest) -> ThumbRender {
        let (hold, wait) = std::sync::mpsc::channel();
        self.held.lock().expect("the held signals").push(hold);
        ThumbRender::Later(Box::new(move |_| {
            let _ = wait.recv();
            Err(LoadFailure::Missing)
        }))
    }
}

impl PageSource for LoadingPages {
    fn page_count(&self) -> usize {
        self.pages.page_count()
    }

    fn page_size(&self, page: usize) -> Size {
        self.pages.page_size(page)
    }
}

/// Whether any filled rectangle among `shapes` has the colour `color`.
fn fills_with(shapes: &[egui::Shape], color: egui::Color32) -> bool {
    shapes.iter().any(|shape| match shape {
        egui::Shape::Rect(rect) => rect.fill == color,
        egui::Shape::Vec(inner) => fills_with(inner, color),
        _ => false,
    })
}

/// A paper colour no other fill of the strip uses.
const LOADING_PAPER: egui::Color32 = egui::Color32::from_rgb(0x12, 0xab, 0x34);

#[trial]
fn a_thumbnail_paints_the_paper_in_its_tile_while_its_picture_loads() {
    let pages = LoadingPages { pages: DemoPages::new(2), held: Default::default() };
    let mut picked = Selection::single(0);
    let mut frames = Frames::new();
    frames.screen(vec2(400.0, 500.0));
    let mut show = |ui: &mut egui::Ui| {
        let _ = PageThumbnails::new("Pages", &pages, &mut picked, 0, DataColor::new(LOADING_PAPER)).show(ui);
    };
    frames.idle(4, &mut show);
    let shapes = frames.frame_shapes(&mut show);
    assert!(fills_with(&shapes, LOADING_PAPER), "a tile whose picture is loading shows the paper");
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
