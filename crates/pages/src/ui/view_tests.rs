//! The page view in a harness over a mock page source.
#![allow(clippy::expect_used, reason = "tests")]

use std::cell::RefCell;

use craft_ui::canvas::ZoomLadder;
use craft_ui::canvas::textures::{FakeThumbSource, Revision, ThumbRender, ThumbRequest, ThumbSource};
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, MouseWheelUnit, vec2};
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, HarnessBuilder};
use kurbo::Size;

use super::view::*;
use crate::layout::{Fit, LayoutOptions, PageDisplay, PagedLayout, SCREEN_POINTS_PER_POINT, ZOOM_RANGE};

const LETTER: Size = Size::new(612.0, 792.0);

struct Pages {
    count: usize,
    fake: FakeThumbSource<usize>,
    asked: RefCell<Vec<usize>>,
}

impl ThumbSource<usize> for Pages {
    fn revision(&self, key: &usize, request: &ThumbRequest) -> Revision {
        self.asked.borrow_mut().push(*key);
        self.fake.revision(key, request)
    }

    fn render(&self, key: &usize, request: &ThumbRequest) -> ThumbRender {
        self.fake.render(key, request)
    }
}

impl PageSource for Pages {
    fn page_count(&self) -> usize {
        self.count
    }

    fn page_size(&self, _page: usize) -> Size {
        LETTER
    }
}

struct App {
    pages: Pages,
    state: PageViewState,
    options: LayoutOptions,
    layout: PagedLayout,
    shown: Vec<PlacedPage>,
    visible: std::ops::Range<usize>,
    view_height: f64,
    changes: usize,
}

fn open(count: usize, options: LayoutOptions, setup: impl FnOnce(&mut PageViewState)) -> Harness<'static, App> {
    let pages = Pages { count, fake: FakeThumbSource::new().aspect(612, 792), asked: RefCell::new(Vec::new()) };
    let mut state = PageViewState::default();
    setup(&mut state);
    let app = App { pages, state, options, layout: PagedLayout::default(), shown: Vec::new(), visible: 0..0, view_height: 0.0, changes: 0 };
    let mut harness =
        HarnessBuilder::default().with_size(vec2(900.0, 600.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, app: &mut App| {
                let out = PageView::new("doc", &app.pages, &mut app.state).options(app.options).name("Doc").show(ui);
                app.changes += usize::from(out.current_changed);
                app.view_height = f64::from(out.canvas.rect.height());
                app.shown = out.pages;
                app.visible = out.visible;
                app.layout = out.layout;
            },
            app,
        );
    harness.run();
    harness
}

fn focus(harness: &mut Harness<'_, App>) {
    harness.get_by_role(Role::Document).focus();
    harness.run();
}

fn wheel(harness: &mut Harness<'_, App>, lines: f32) {
    harness.hover_at(egui::pos2(450.0, 300.0));
    harness.event(Event::MouseWheel {
        unit: MouseWheelUnit::Line,
        delta: vec2(0.0, lines),
        modifiers: Modifiers::NONE,
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
    });
    harness.run();
}

// Fit width in a 900 pt view over Letter pages writes the shown zoom into the state; Zoom In then steps from it.
#[test]
fn fit_width_keeps_the_zoom_equal_to_what_shows() {
    let mut harness = open(5, LayoutOptions::ACROBAT, |s| {
        s.fit = Fit::Width;
        s.zoom = 3.0;
    });
    let app = harness.state();
    let shown = app.shown.first().expect("first page shows");
    let scale = f64::from(shown.rect.width()) / LETTER.width;
    assert!((app.state.zoom * SCREEN_POINTS_PER_POINT - scale).abs() < 1e-3, "{} vs {scale}", app.state.zoom);
    assert!(app.state.zoom < 3.0, "the fit replaced the stale zoom");

    let before = app.state.zoom;
    harness.state_mut().state.zoom_step(true);
    harness.run();
    let app = harness.state();
    assert_eq!(app.state.fit, Fit::None);
    assert_eq!(app.state.zoom, ZoomLadder::step_in_range(before, 1, ZOOM_RANGE));
    assert!(app.state.zoom > before);
}

// Page Down moves one spread two-up with a cover; the wheel turns a page that fits; a zoomed-in page pans first and turns at its edge.
#[test]
fn paging_keys_and_the_wheel_turn_rows_when_not_continuous() {
    let two_up = LayoutOptions { display: PageDisplay::TwoUp, ..LayoutOptions::ACROBAT };
    let mut harness = open(7, two_up, |s| {
        s.display = PageDisplay::TwoUp;
        s.cover = true;
        s.continuous = false;
        s.fit = Fit::Page;
    });
    focus(&mut harness);
    harness.key_press(Key::PageDown);
    harness.run();
    assert_eq!(harness.state().state.current, 1, "the cover is a row of its own");
    harness.key_press(Key::PageDown);
    harness.run();
    assert_eq!(harness.state().state.current, 3);
    harness.key_press(Key::PageUp);
    harness.run();
    assert_eq!(harness.state().state.current, 1);

    let mut harness = open(4, LayoutOptions::ACROBAT, |s| {
        s.continuous = false;
        s.fit = Fit::Page;
    });
    wheel(&mut harness, -3.0);
    assert_eq!(harness.state().state.current, 1, "a page that fits turns with the wheel");

    let mut harness = open(4, LayoutOptions::ACROBAT, |s| {
        s.continuous = false;
        s.fit = Fit::None;
        s.zoom = 2.0;
    });
    wheel(&mut harness, -1.0);
    assert_eq!(harness.state().state.current, 0, "a zoomed-in page pans first");
    assert!(harness.state().state.offset.1 > 0.0);
    for _ in 0..60 {
        if harness.state().state.current == 1 {
            break;
        }
        wheel(&mut harness, -3.0);
    }
    let app = harness.state();
    assert_eq!(app.state.current, 1, "and turns at its edge");
    assert!(app.state.offset.1 <= app.layout.rects[1].y0, "the next page starts at its top");
}

// Scrolling half a page past page 1 makes page 2 current and sets current_changed once.
#[test]
fn current_page_follows_the_middle_line() {
    let mut harness = open(5, LayoutOptions::ACROBAT, |s| {
        s.fit = Fit::None;
        s.zoom = 1.0;
    });
    assert_eq!(harness.state().state.current, 0);
    let app = harness.state_mut();
    let middle = app.layout.rects[1].y0 + app.layout.rects[1].height() / 2.0;
    app.state.offset.1 = middle - app.view_height / 2.0;
    harness.run();
    let app = harness.state();
    assert_eq!(app.state.current, 1);
    assert_eq!(app.changes, 1);
}

// Of 200 pages only those on screen ask with visible true and one row either side with false.
#[test]
fn only_pages_near_the_view_ask_for_rasters() {
    let mut harness = open(200, LayoutOptions::ACROBAT, |s| {
        s.fit = Fit::None;
        s.zoom = 1.0;
    });
    let app = harness.state_mut();
    let middle = app.layout.rects[100].y0;
    app.state.offset.1 = middle - app.view_height / 2.0;
    app.pages.asked.borrow_mut().clear();
    harness.run();
    let app = harness.state();
    let mut asked = app.pages.asked.borrow().clone();
    asked.sort_unstable();
    asked.dedup();
    assert!(!app.visible.is_empty());
    let (first, last) = (app.visible.start - 1, app.visible.end);
    assert_eq!(asked, (first..=last).collect::<Vec<_>>(), "visible {:?}", app.visible);
}
