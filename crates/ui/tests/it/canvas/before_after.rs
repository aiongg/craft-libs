//! `BeforeAfter`: the divider's fraction counts from the reading start across the shown picture, in both directions.

use craft_i18n::Direction;
use craft_ui::canvas::{BeforeAfter, BeforeAfterMode};
use craft_ui::contract::{Bind, Show as _};
use craft_ui::testing::{Fixture, trial};
use egui::{Pos2, Rect, Vec2};
use std::cell::Cell;
use std::rc::Rc;

/// Without an aspect ratio the picture fills the area.
const AREA: Vec2 = Vec2::new(400.0, 300.0);

/// Presses the picture of a wipe at a quarter of its width from its left edge and returns the divider's fraction.
fn pressed_at_left_quarter(mode: BeforeAfterMode, direction: Direction) -> f32 {
    let value = Rc::new(Cell::new(0.5_f32));
    let shared = Rc::clone(&value);
    let mut fixture = Fixture::new(move |ui| {
        let rect = Rect::from_min_size(ui.max_rect().min, AREA);
        let mut fraction = shared.get();
        BeforeAfter::new("Compare", mode, Bind::bound(&mut fraction)).show_at(ui, rect);
        shared.set(fraction);
    });
    fixture.direction(direction);
    fixture.settle();
    let area = Rect::from_min_size(fixture.ctx().content_rect().min, AREA);
    let picture = area;
    let (x, y) = (picture.left() + picture.width() * 0.25, picture.top() + picture.height() * 0.25);
    fixture.click_at(Pos2::new(x, y));
    fixture.settle();
    value.get()
}

#[trial]
fn a_press_sets_the_fraction_from_the_reading_start_of_the_shown_picture() {
    let near = |a: f32, b: f32| (a - b).abs() < 0.02;
    let ltr = pressed_at_left_quarter(BeforeAfterMode::Split, Direction::Ltr);
    assert!(near(ltr, 0.25), "left to right counts from the left edge: {ltr}");
    let rtl = pressed_at_left_quarter(BeforeAfterMode::Split, Direction::Rtl);
    assert!(near(rtl, 0.75), "right to left counts from the right edge: {rtl}");
    let stacked = pressed_at_left_quarter(BeforeAfterMode::SplitStacked, Direction::Rtl);
    assert!(near(stacked, 0.25), "a horizontal divider counts from the top in both directions: {stacked}");
}
