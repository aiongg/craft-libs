use egui::pos2;
use kurbo::Point;

use super::*;
use crate::canvas::CanvasView;
use crate::composites::ColorStop;
use craft_theme::Srgba;

fn stops(locations: &[f32]) -> GradientStops {
    let black = Srgba { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    GradientStops { color: locations.iter().map(|&l| ColorStop::new(l, black)).collect(), opacity: Vec::new() }
}

#[test]
fn locations_follow_reverse() {
    let line = GradientLine { start: Point::new(10.0, 10.0), end: Point::new(110.0, 10.0) };
    assert_eq!(at(&line, 0.25, false), Point::new(35.0, 10.0));
    assert_eq!(at(&line, 0.25, true), Point::new(85.0, 10.0));
    let (t, d) = project(&line, Point::new(85.0, 40.0), true);
    assert!((t - 0.25).abs() < 1e-6 && (d - 30.0).abs() < 1e-9);
    // Past the end the projection clamps to the end handle.
    let (t, d) = project(&line, Point::new(210.0, 10.0), false);
    assert_eq!((t, d), (1.0, 100.0));
}

#[test]
fn snap_steps_start_at_the_base_angle() {
    let fixed = Point::new(0.0, 0.0);
    let snapped = snap45(fixed, Point::new(10.0, 1.0), 0.0);
    assert!((snapped.x - 10.0_f64.hypot(1.0)).abs() < 1e-9 && snapped.y.abs() < 1e-9);
    let diagonal = snap45(fixed, Point::new(10.0, 9.0), 0.0);
    assert!((diagonal.x - diagonal.y).abs() < 1e-9);
    // With the steps counted from 10° the nearest to the horizontal is 10° itself.
    let tilted = snap45(fixed, Point::new(10.0, 0.5), 10.0);
    assert!((tilted.y.atan2(tilted.x).to_degrees() - 10.0).abs() < 1e-9);
}

#[test]
fn hit_in_a_rotated_flipped_view() {
    let rect = egui::Rect::from_min_size(pos2(0.0, 0.0), egui::vec2(400.0, 300.0));
    let view = CanvasView { rotation: 30.0, flip_x: true, aspect: 1.5, ..CanvasView::new(rect, Point::new(200.0, 150.0), 1.0) };
    let line = GradientLine { start: Point::new(120.0, 150.0), end: Point::new(280.0, 150.0) };
    let (s, e) = (view.to_screen(line.start), view.to_screen(line.end));
    let stops = stops(&[0.0, 0.5, 1.0]);
    let stop = at_screen(s, e, 0.5, true);
    let along = (e - s).normalized();
    let across = egui::vec2(-along.y, along.x);
    assert_eq!(hit(s, e, &stops, true, true, stop + across * 7.9, 8.0), Some(LinePart::Stop(1)));
    // 8.1 pt off the stop is still on the line only if within 8 of it, and it is not.
    assert_eq!(hit(s, e, &stops, true, true, stop + across * 8.1, 8.0), None);
    // The end handle beats a stop under it.
    let on_end = hit(s, e, &self::stops(&[0.0, 1.0]), false, true, e, 8.0);
    assert_eq!(on_end, Some(LinePart::End));
    // A point along the line between parts is the line; a midpoint sits at its stops' share.
    assert_eq!(hit(s, e, &stops, true, true, at_screen(s, e, 0.25, true), 8.0), Some(LinePart::Midpoint(0)));
    assert_eq!(hit(s, e, &stops, true, false, at_screen(s, e, 0.25, true), 8.0), Some(LinePart::Line));
}
