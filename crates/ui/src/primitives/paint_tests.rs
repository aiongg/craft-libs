//! Painter helpers.
use egui::{Color32, FontId, Pos2, pos2};
use egui_kittest::Harness;

use super::paint::{fit_font, ink_on, is_convex, triangulate};

fn l_shape() -> Vec<Pos2> {
    vec![pos2(0.0, 0.0), pos2(2.0, 0.0), pos2(2.0, 1.0), pos2(1.0, 1.0), pos2(1.0, 2.0), pos2(0.0, 2.0)]
}

#[test]
fn ear_clipping_splits_a_concave_polygon() {
    let l = l_shape();
    assert_eq!(triangulate(&l).len(), (l.len() - 2) * 3);
    let reversed: Vec<Pos2> = l.iter().rev().copied().collect();
    assert_eq!(triangulate(&reversed).len(), (l.len() - 2) * 3);
    assert!(triangulate(&[pos2(0.0, 0.0), pos2(1.0, 0.0)]).is_empty());
}

#[test]
fn ear_clipping_skips_repeated_and_collinear_vertices() {
    // An area chart whose value is 0 at the right end: the top point equals the baseline point.
    let chart = [pos2(0.0, 100.0), pos2(10.0, 50.0), pos2(20.0, 100.0), pos2(20.0, 100.0), pos2(0.0, 100.0)];
    assert_eq!(triangulate(&chart).len(), 3);
    let with_midpoint = [pos2(0.0, 0.0), pos2(1.0, 0.0), pos2(2.0, 0.0), pos2(2.0, 2.0), pos2(0.0, 2.0)];
    assert_eq!(triangulate(&with_midpoint).len(), 6);
}

#[test]
fn convexity_ignores_collinear_points() {
    let square = [pos2(0.0, 0.0), pos2(1.0, 0.0), pos2(2.0, 0.0), pos2(2.0, 2.0), pos2(0.0, 2.0)];
    assert!(is_convex(&square));
    assert!(!is_convex(&l_shape()));
}

#[test]
fn fitting_text_shrinks_only_what_overflows() {
    let mut harness = Harness::new_ui(|ui| {
        let font = FontId::proportional(20.0);
        let painter = ui.painter();
        let natural = painter.layout_no_wrap("Hello, world".to_string(), font.clone(), Color32::WHITE).size().x;
        assert!(fit_font(painter, "Hello, world", &font, natural + 1.0).is_none());
        let smaller = fit_font(painter, "Hello, world", &font, natural / 2.0).expect("overflowing text shrinks");
        assert!(smaller.size < 20.0 * 0.5 + 0.01, "{}", smaller.size);
        assert_eq!(smaller.family, font.family);
    });
    harness.run();
}

#[test]
fn ink_reads_on_its_fill() {
    assert_eq!(ink_on(Color32::from_rgb(0, 0, 0x80)), Color32::WHITE);
    assert_eq!(ink_on(Color32::from_rgb(0xff, 0xe0, 0)), Color32::BLACK);
    assert_eq!(ink_on(Color32::from_rgb(0, 0xc8, 0)), Color32::BLACK);
}
