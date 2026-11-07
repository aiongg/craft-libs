//! The transform box: which keys change a corner drag, and the skew limit.

use craft_behavior::geometry::{BoxRules, BoxState, Parts, SKEW_LIMIT, TransformMode, drag};
use craft_keymap::Mods;
use kurbo::{Point, Rect};

const ALT_SHIFT: Mods = Mods { alt: true, shift: true, cmd: false, ctrl: false };

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

/// 100 by 50 at the origin, the reference point at its centre.
fn session() -> BoxState {
    BoxState::from_rect(Rect::new(0.0, 0.0, 100.0, 50.0))
}

fn dragged(from: Point, to: Point, mods: Mods, rules: &BoxRules) -> BoxState {
    let start = session();
    drag(&start, &start, start.part_at(from, 8.0), from, to, mods, rules)
}

fn near(a: Point, b: Point) -> bool {
    (a - b).hypot() < 1e-6
}

fn corners_are(s: &BoxState, x0: f64, y0: f64, x1: f64, y1: f64) -> bool {
    let want = BoxState::from_rect(Rect::new(x0, y0, x1, y1)).quad;
    s.quad.iter().zip(&want).all(|(a, b)| near(*a, *b))
}

#[test]
fn a_corner_drag_keeps_the_proportion_and_shift_frees_it() {
    let rules = BoxRules::default();
    let kept = dragged(p(100.0, 50.0), p(200.0, 60.0), Mods::NONE, &rules);
    assert!(corners_are(&kept, 0.0, 0.0, 200.0, 100.0), "{:?}", kept.quad);
    let freed = dragged(p(100.0, 50.0), p(200.0, 60.0), Mods::SHIFT, &rules);
    assert!(corners_are(&freed, 0.0, 0.0, 200.0, 60.0), "{:?}", freed.quad);
}

#[test]
fn alt_scales_about_the_centre_in_a_corner_drag() {
    let scaled = dragged(p(100.0, 50.0), p(150.0, 75.0), Mods::ALT, &BoxRules::default());
    assert!(corners_are(&scaled, -50.0, -25.0, 150.0, 75.0), "{:?}", scaled.quad);
    let stretched = dragged(p(100.0, 50.0), p(150.0, 60.0), ALT_SHIFT, &BoxRules::default());
    assert!(corners_are(&stretched, -50.0, -10.0, 150.0, 60.0), "{:?}", stretched.quad);
}

#[test]
fn shift_move_inside_keeps_to_eight_directions() {
    let moved = dragged(p(30.0, 25.0), p(50.0, 29.0), Mods::SHIFT, &BoxRules::default());
    assert!(corners_are(&moved, 20.0, 0.0, 120.0, 50.0), "{:?}", moved.quad);
}

#[test]
fn a_distort_mode_refuses_the_outside_and_the_edges() {
    let rules = BoxRules { mode: TransformMode::Distort, ..BoxRules::default() };
    let start = session();
    let outside = dragged(p(130.0, 25.0), p(160.0, 25.0), Mods::NONE, &rules);
    assert_eq!(outside.quad, start.quad);
    let edge = dragged(p(100.0, 25.0), p(150.0, 25.0), Mods::NONE, &rules);
    assert_eq!(edge.quad, start.quad);
}

#[test]
fn a_skew_past_the_limit_stops_at_the_limit() {
    let s = session();
    let rect = Rect::new(0.0, 0.0, 100.0, 50.0);
    let now = s.parts(rect, 0.0);
    let leaned = s.with_parts(rect, 0.0, Parts { skew_h: 120.0, ..now });
    let reading = leaned.parts(rect, 0.0);
    assert!((reading.skew_h - SKEW_LIMIT).abs() < 1e-6, "{reading:?}");
}
