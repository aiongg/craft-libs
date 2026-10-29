//! Keeping dialogs on screen and pinning their top-left.

use egui::{Vec2, vec2};

use super::*;

const WINDOW: Vec2 = vec2(1000.0, 700.0);
const TITLE: f32 = 24.0;
const KEEP: f32 = 100.0;

fn keep(offset: Vec2, size: Vec2, content: Vec2) -> Vec2 {
    keep_on_screen(offset, size, TITLE, content, KEEP)
}

#[test]
fn a_dialog_that_fits_stays_whole_on_screen() {
    let size = vec2(400.0, 300.0);
    assert_eq!(keep(vec2(100.0, 50.0), size, WINDOW), vec2(100.0, 50.0));
    assert_eq!(keep(vec2(-30.0, -5.0), size, WINDOW), vec2(0.0, 0.0));
    assert_eq!(keep(vec2(900.0, 650.0), size, WINDOW), vec2(600.0, 400.0));
}

#[test]
fn a_dialog_wider_than_the_window_keeps_part_of_its_width() {
    let size = vec2(1200.0, 300.0);
    assert_eq!(keep(vec2(-50.0, 0.0), size, WINDOW).x, -50.0);
    assert_eq!(keep(vec2(-5000.0, 0.0), size, WINDOW).x, KEEP - 1200.0, "100 points stay at the right edge");
    assert_eq!(keep(vec2(5000.0, 0.0), size, WINDOW).x, 900.0, "100 points stay at the left edge");
}

#[test]
fn a_dialog_taller_than_the_window_keeps_its_top_and_title_row() {
    let size = vec2(400.0, 900.0);
    assert_eq!(keep(vec2(10.0, -20.0), size, WINDOW), vec2(10.0, 0.0));
    assert_eq!(keep(vec2(10.0, 800.0), size, WINDOW), vec2(10.0, 700.0 - TITLE));
}

#[test]
fn a_shrinking_window_moves_the_dialog_in_and_a_growing_one_leaves_it() {
    let size = vec2(400.0, 300.0);
    let offset = vec2(500.0, 350.0);
    assert_eq!(keep(offset, size, vec2(800.0, 500.0)), vec2(400.0, 200.0));
    assert_eq!(keep(offset, size, vec2(2000.0, 1400.0)), offset);
}

#[test]
fn a_window_smaller_than_the_kept_width_does_not_panic() {
    let size = vec2(400.0, 300.0);
    let out = keep(vec2(10.0, 10.0), size, vec2(60.0, 50.0));
    assert!(out.is_finite());
}

#[test]
fn the_first_laid_out_pass_pins_where_the_dialog_is() {
    let size = vec2(400.0, 300.0);
    let at = vec2(300.0, 200.0);
    assert_eq!(next_pin(None, at, Vec2::ZERO, size, TITLE, WINDOW, KEEP), Some(at));
}

#[test]
fn a_pinned_dialog_stays_where_it_is_while_its_body_grows() {
    let pinned = vec2(300.0, 200.0);
    // The area moved by half the growth (it was centred), but the pin does not follow it.
    assert_eq!(next_pin(Some(pinned), vec2(250.0, 150.0), Vec2::ZERO, vec2(500.0, 400.0), TITLE, WINDOW, KEEP), None);
}

#[test]
fn a_drag_moves_the_pin_and_is_kept_on_screen() {
    let pinned = vec2(300.0, 200.0);
    let size = vec2(400.0, 300.0);
    assert_eq!(next_pin(Some(pinned), pinned, vec2(25.0, -10.0), size, TITLE, WINDOW, KEEP), Some(vec2(325.0, 190.0)));
    assert_eq!(next_pin(Some(pinned), pinned, vec2(-900.0, 0.0), size, TITLE, WINDOW, KEEP), Some(vec2(0.0, 200.0)));
}

#[test]
fn place_keys_of_different_values_differ() {
    assert_ne!(PlaceKey::new("blur"), PlaceKey::new("curves"));
    assert_eq!(PlaceKey::new(("kind", 3)), PlaceKey::new(("kind", 3)));
}
