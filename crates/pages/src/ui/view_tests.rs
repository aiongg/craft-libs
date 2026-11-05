//! The page view in a harness over a mock page source.
#![allow(clippy::expect_used, reason = "tests")]

use std::cell::RefCell;

use craft_ui::canvas::textures::{FakeThumbSource, Revision, ThumbRender, ThumbRequest, ThumbSource};
use craft_ui::canvas::{PasteboardResponse, ZoomLadder};
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
    /// Every request the source was asked to render (not served from the cache).
    renders: RefCell<Vec<ThumbRequest>>,
}

impl ThumbSource<usize> for Pages {
    fn revision(&self, key: &usize, request: &ThumbRequest) -> Revision {
        self.asked.borrow_mut().push(*key);
        self.fake.revision(key, request)
    }

    fn render(&self, key: &usize, request: &ThumbRequest) -> ThumbRender {
        self.renders.borrow_mut().push(*request);
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
    canvas_top: f32,
    changes: usize,
    pasteboard: PasteboardResponse,
}

fn open(count: usize, options: LayoutOptions, setup: impl FnOnce(&mut PageViewState)) -> Harness<'static, App> {
    let pages = Pages { count, fake: FakeThumbSource::new().aspect(612, 792), asked: RefCell::new(Vec::new()), renders: RefCell::new(Vec::new()) };
    let mut state = PageViewState::default();
    setup(&mut state);
    let app = App {
        pages,
        state,
        options,
        layout: PagedLayout::default(),
        shown: Vec::new(),
        visible: 0..0,
        view_height: 0.0,
        canvas_top: 0.0,
        changes: 0,
        pasteboard: PasteboardResponse::default(),
    };
    let mut harness =
        HarnessBuilder::default().with_size(vec2(900.0, 600.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, app: &mut App| {
                let out = PageView::new("doc", &app.pages, &mut app.state).options(app.options).name("Doc").show(ui);
                app.changes += usize::from(out.current_changed);
                app.view_height = f64::from(out.canvas.rect.height());
                app.canvas_top = out.canvas.rect.min.y;
                app.pasteboard = out.pasteboard;
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

fn ctrl_wheel(harness: &mut Harness<'_, App>, lines: f32) {
    harness.hover_at(egui::pos2(450.0, 300.0));
    harness.event(Event::MouseWheel {
        unit: MouseWheelUnit::Line,
        delta: vec2(0.0, lines),
        modifiers: Modifiers::CTRL,
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
    });
    harness.run();
}

// Ctrl+wheel zooms (Acrobat), and the plain wheel scrolls; neither turns into the other.
#[test]
fn ctrl_wheel_zooms_and_the_plain_wheel_scrolls() {
    let mut harness = open(5, LayoutOptions::ACROBAT, |s| {
        s.fit = Fit::None;
        s.zoom = 1.0;
    });
    ctrl_wheel(&mut harness, 2.0);
    let app = harness.state();
    assert!(app.state.zoom > 1.0, "zoom {}", app.state.zoom);
    assert_eq!(app.state.fit, Fit::None);
    let zoomed = app.state.zoom;
    wheel(&mut harness, -2.0);
    let app = harness.state();
    assert_eq!(app.state.zoom, zoomed);
    assert!(app.state.offset.1 > 0.0, "the wheel scrolled");
}

// A page bigger than the texture side asks for the part in view as a region of the page, and 100 % asks for none.
#[test]
fn a_page_larger_than_the_texture_side_asks_for_the_part_in_view() {
    let harness = open(3, LayoutOptions::ACROBAT, |s| {
        s.fit = Fit::None;
        s.zoom = 1.0;
    });
    assert!(harness.state().pages.renders.borrow().iter().all(|r| r.region.is_none()), "a page that fits asks for the whole page");

    let harness = open(3, LayoutOptions::ACROBAT, |s| {
        s.fit = Fit::None;
        s.zoom = 4.0;
    });
    let renders = harness.state().pages.renders.borrow();
    let part = renders.iter().find_map(|r| r.region).expect("a region was asked");
    assert!(part.min.x >= 0.0 && part.min.y >= 0.0 && part.max.x <= 1.0 && part.max.y <= 1.0, "{part:?}");
    assert!(part.width() < 1.0 && part.height() < 1.0, "only the part in view: {part:?}");
    let side = renders.iter().filter(|r| r.region.is_some()).map(|r| r.side).max().expect("sides");
    assert!(side <= 2048, "within the texture side: {side}");
    assert!(renders.iter().any(|r| r.region.is_none()), "the page's own picture is asked too");
}

// Sharp renders wait while the scale still moves: a zoom step asks for nothing new that pass.
#[test]
fn sharp_renders_are_held_while_the_zoom_moves() {
    let mut harness = open(3, LayoutOptions::ACROBAT, |s| {
        s.fit = Fit::None;
        s.zoom = 1.0;
    });
    harness.state_mut().state.zoom = 1.5;
    harness.step();
    harness.state().pages.renders.borrow_mut().clear();
    harness.state_mut().state.zoom = 1.7;
    harness.step();
    assert!(harness.state().pages.renders.borrow().is_empty(), "held while the zoom moves");
}

fn scroll_bar_drag(harness: &mut Harness<'_, App>) {
    let at = egui::pos2(895.0, 20.0);
    harness.hover_at(at);
    harness.event(Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    harness.step();
    harness.event(Event::PointerMoved(egui::pos2(895.0, 40.0)));
    harness.step();
    harness.event(Event::PointerMoved(egui::pos2(895.0, 80.0)));
    harness.step();
}

// A drag on the vertical scroll bar moves the pages in the same pass: what the pass drew is where its offset says.
#[test]
fn a_scroll_bar_drag_lands_in_the_same_pass() {
    let mut harness = open(6, LayoutOptions::ACROBAT, |s| {
        s.fit = Fit::None;
        s.zoom = 1.0;
    });
    scroll_bar_drag(&mut harness);
    let app = harness.state();
    assert!(app.state.offset.1 > 0.0, "the bar moved the view");
    let first = app.shown.first().expect("a page shows");
    let drawn = first.rect.min.y;
    let wanted = app.canvas_top + (app.layout.rects[first.page].y0 - app.state.offset.1) as f32;
    assert!((drawn - wanted).abs() < 0.01, "drawn at {drawn}, offset says {wanted}");
}

// The Menu key on the focused view asks for the pasteboard's colour menu.
#[test]
fn the_menu_key_reports_the_pasteboard_menu() {
    let mut harness = open(3, LayoutOptions::ACROBAT, |_| {});
    focus(&mut harness);
    harness.event(Event::Key { key: Key::Menu, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
    harness.step();
    assert!(harness.state().pasteboard.context_at.is_some());
}

// A zoom the app sets keeps the page at the top of the view at the same fraction, its middle in the middle.
#[test]
fn a_changed_zoom_keeps_the_page_at_the_top_in_place() {
    let mut harness = open(5, LayoutOptions::ACROBAT, |s| {
        s.fit = Fit::None;
        s.zoom = 1.0;
    });
    let app = harness.state_mut();
    let r = app.layout.rects[2];
    app.state.offset.1 = r.y0 + 0.4 * r.height();
    harness.run();
    harness.state_mut().state.zoom = 2.0;
    harness.run();
    let app = harness.state();
    let r = app.layout.rects[2];
    assert!((app.state.offset.1 - (r.y0 + 0.4 * r.height())).abs() < 1.0, "offset {} page {r:?}", app.state.offset.1);
}

// Wheel gathered past the bottom of a zoomed-in page is forgotten by a pass without wheel input: two half notches apart don't turn the page.
#[test]
fn wheel_overshoot_resets_on_a_pass_without_wheel_input() {
    let mut harness = open(4, LayoutOptions::ACROBAT, |s| {
        s.continuous = false;
        s.fit = Fit::None;
        s.zoom = 2.0;
    });
    harness.state_mut().state.offset.1 = 1e9;
    harness.run();
    wheel(&mut harness, -0.5);
    harness.run();
    wheel(&mut harness, -0.5);
    harness.run();
    assert_eq!(harness.state().state.current, 0, "each half notch alone is not a turn");

    wheel(&mut harness, -3.0);
    assert_eq!(harness.state().state.current, 1, "a whole notch at once is");
}
