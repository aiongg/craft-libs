//! Reading view geometry and turning.

use std::cell::RefCell;

use craft_ui::canvas::textures::{FakeThumbSource, Revision, ThumbRender, ThumbRequest, ThumbSource};
use craft_ui::direction::{self, Direction};
use craft_ui::testing::events;
use craft_ui::theme::{Base, current, install};
use egui::accesskit::Role;
use egui::{Event, Modifiers, MouseWheelUnit, Vec2, pos2, vec2};
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, HarnessBuilder};
use kurbo::Size;

use super::reading::*;
use super::style::ReadingStyle;
use super::view::PageSource;

fn style() -> ReadingStyle {
    ReadingView::style(&current(&egui::Context::default()))
}

// A 1200 pt wide view gives four columns of at least 192 pt; a requested 612 pt column in a 500 pt view shrinks to the window less two gaps.
#[test]
fn reading_size_fits_whole_columns() {
    let style = style();
    let (width, height) = reading_size(vec2(1200.0, 900.0), 1.0, &style, None);
    assert!(width >= style.column_min, "{width}");
    assert!((4.0 * width + 5.0 * style.column_gap - 1200.0).abs() < 1e-3, "{width}");
    assert_eq!(height, 900.0 - style.column_gap - style.nav_bar);
    let (requested, _) = reading_size(vec2(500.0, 900.0), 1.0, &style, Some(612.0));
    assert_eq!(requested, 500.0 - 2.0 * style.column_gap);
    let (scaled, _) = reading_size(vec2(1200.0, 900.0), 2.0, &style, Some(300.0));
    assert_eq!(scaled, 300.0);
}

fn wheel(unit: MouseWheelUnit, delta: Vec2, modifiers: Modifiers) -> Event {
    Event::MouseWheel { unit, delta, modifiers, phase: egui::TouchPhase::Move, source: egui::MouseWheelSource::Unknown }
}

// One line notch turns once (three lines on the web); 30 then 30 touchpad points turn once and reset; Ctrl+wheel turns nothing.
#[test]
fn wheel_notches_and_touchpads_turn_once() {
    let mut accum = 0.0;
    let line = |lines| wheel(MouseWheelUnit::Line, vec2(0.0, lines), Modifiers::NONE);
    assert_eq!(wheel_turns(&[line(-1.0)], true, &mut accum), 1);
    assert_eq!(wheel_turns(&[line(1.0)], true, &mut accum), -1);
    assert_eq!(wheel_turns(&[line(-1.0)], false, &mut accum), 0);
    let points = wheel(MouseWheelUnit::Point, vec2(0.0, -30.0), Modifiers::NONE);
    assert_eq!(wheel_turns(std::slice::from_ref(&points), true, &mut accum), 0);
    assert_eq!(wheel_turns(std::slice::from_ref(&points), true, &mut accum), 1);
    assert_eq!(accum, 0.0);
    let ctrl = wheel(MouseWheelUnit::Line, vec2(0.0, -1.0), Modifiers::CTRL);
    assert_eq!(wheel_turns(&[ctrl], true, &mut accum), 0);
}

// Steps toward 300 from 0 never overshoot and end exactly on 300.
#[test]
fn animation_settles_on_the_boundary() {
    let mut offset = 0.0;
    for _ in 0..200 {
        offset = animation_step(offset, 300.0, 1.0 / 60.0);
        assert!(offset <= 300.0, "{offset}");
    }
    assert_eq!(offset, 300.0);
    assert_eq!(animation_step(0.0, 300.0, 10.0), 300.0 * (1.0 - (-1.8_f32).exp()));
}

// A chosen column gives way to a narrow window down to 144 screen points whatever the scale (WordCraft `reading_size`): 300 pt at scale 2 in a 150 pt window is 72 pt, not 144.
#[test]
fn a_narrow_window_floors_the_column_at_144_screen_points() {
    let (width, _) = reading_size(vec2(150.0, 900.0), 2.0, &style(), Some(300.0));
    assert_eq!(width, 72.0);
}

struct Pages {
    fake: FakeThumbSource<usize>,
    renders: RefCell<usize>,
}

impl ThumbSource<usize> for Pages {
    fn revision(&self, key: &usize, request: &ThumbRequest) -> Revision {
        self.fake.revision(key, request)
    }

    fn render(&self, key: &usize, request: &ThumbRequest) -> ThumbRender {
        *self.renders.borrow_mut() += 1;
        self.fake.render(key, request)
    }
}

impl PageSource for Pages {
    fn page_count(&self) -> usize {
        6
    }

    fn page_size(&self, _page: usize) -> Size {
        Size::new(200.0, 300.0)
    }
}

struct App {
    pages: Pages,
    state: ReadingState,
}

fn open(dir: Direction) -> Harness<'static, App> {
    let app = App { pages: Pages { fake: FakeThumbSource::new().aspect(200, 300), renders: RefCell::new(0) }, state: ReadingState::default() };
    let mut harness = HarnessBuilder::default()
        .with_size(vec2(900.0, 500.0))
        .with_context_setup(move |ctx| {
            install(ctx, &Base::suite_default());
            direction::apply(ctx, dir);
        })
        .build_ui_state(
            |ui, app: &mut App| {
                ReadingView::new("read", &app.pages, &mut app.state).name("Story").show(ui);
            },
            app,
        );
    harness.run();
    harness
}

/// The column the view is on or turning to.
fn going_to(harness: &Harness<'_, App>) -> usize {
    harness.state().state.target.unwrap_or(harness.state().state.column)
}

fn swipe(harness: &mut Harness<'_, App>, by: f32) {
    for frame in events::drag(pos2(450.0, 200.0), pos2(450.0 + by, 200.0), Modifiers::NONE) {
        for event in frame {
            harness.event(event);
        }
        harness.step();
    }
    harness.run();
}

// The Document node carries the name the app gave.
#[test]
fn the_document_node_is_named() {
    let harness = open(Direction::Ltr);
    harness.get_by_role_and_label(Role::Document, "Story");
}

// Right to left the row mirrors: the Next button is at the left, and a swipe to the right turns forward while a swipe to the left (the way a left-to-right row turns back) stays on the first column.
#[test]
fn the_row_mirrors_right_to_left() {
    let mut harness = open(Direction::Rtl);
    let next = harness.get_by_label("Next Screen").rect().center().x;
    let previous = harness.get_by_label("Previous Screen").rect().center().x;
    assert!(next < 450.0 && next < previous, "{next} {previous}");
    swipe(&mut harness, -150.0);
    assert_eq!(going_to(&harness), 0);
    swipe(&mut harness, 150.0);
    assert_eq!(going_to(&harness), 1);
    let mut harness = open(Direction::Ltr);
    swipe(&mut harness, 150.0);
    assert_eq!(going_to(&harness), 0);
    swipe(&mut harness, -150.0);
    assert_eq!(going_to(&harness), 1);
}
