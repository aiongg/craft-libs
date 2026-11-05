//! Reading view geometry and turning.

use craft_ui::theme::current;
use egui::{Event, Modifiers, MouseWheelUnit, Vec2, vec2};

use super::reading::*;
use super::style::ReadingStyle;

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
