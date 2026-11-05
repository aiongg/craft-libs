//! Tests of the corner widgets' geometry (task PE-A5).

use egui::{Pos2, pos2, vec2};
use kurbo::{Point, Vec2};

use super::geometry::{Widget, drag_edit, dragged_radius, hit, widgets};
use super::{CornerKind, CornerPoint, CornerRef, CornerTarget};
use craft_ui::canvas::CanvasView;

fn view(zoom: f64) -> CanvasView {
    CanvasView::new(egui::Rect::from_min_size(Pos2::ZERO, vec2(1000.0, 1000.0)), Point::new(50.0, 30.0), zoom)
}

fn corner(vertex: Point, prev: Point, next: Point, radius: f64) -> CornerPoint {
    CornerPoint { vertex, prev, next, radius, kind: CornerKind::Round }
}

fn polygon(points: &[Point], radius: f64) -> Vec<CornerPoint> {
    let n = points.len();
    (0..n).map(|i| corner(points[i], points[(i + n - 1) % n], points[(i + 1) % n], radius)).collect()
}

fn rect(key: u64, w: f64, h: f64, radius: f64, individual: bool) -> CornerTarget {
    let pts = [Point::new(0.0, 0.0), Point::new(w, 0.0), Point::new(w, h), Point::new(0.0, h)];
    CornerTarget { key, corners: polygon(&pts, radius), individual }
}

fn close(a: Pos2, b: Pos2) -> bool {
    a.distance(b) < 1e-3
}

// A 100 × 60 rectangle's corners at radius 0 sit 10 points inside along each axis at zoom 1 and
// 2; at radius 20 at the arc centre; a regular hexagon's corner on its bisector.
#[test]
fn corner_widget_sits_at_the_arc_centre_and_inside_by_the_inset() {
    for zoom in [1.0, 2.0] {
        let v = view(zoom);
        let ws = widgets(&[rect(1, 100.0, 60.0, 0.0, true)], &v, 10.0, 30.0, 180.0);
        assert_eq!(ws.len(), 4);
        let at = |p: Point| v.to_screen(p);
        let corner0 = at(Point::new(0.0, 0.0));
        assert!(close(ws[0].at, corner0 + vec2(10.0, 10.0)));
        assert!(close(ws[2].at, at(Point::new(100.0, 60.0)) - vec2(10.0, 10.0)));
    }
    let ws = widgets(&[rect(1, 100.0, 60.0, 20.0, true)], &view(1.0), 10.0, 30.0, 180.0);
    assert!(close(ws[0].at, view(1.0).to_screen(Point::new(20.0, 20.0))));
    assert!((ws[0].step - Vec2::new(1.0, 1.0)).hypot() < 1e-9);

    let hex: Vec<Point> = (0..6).map(|i| Point::new(50.0, 30.0) + Vec2::from_angle(f64::from(i) * std::f64::consts::FRAC_PI_3) * 20.0).collect();
    let target = CornerTarget { key: 2, corners: polygon(&hex, 0.0), individual: false };
    let v = view(1.0);
    let ws = widgets(&[target], &v, 10.0, 5.0, 180.0);
    assert_eq!(ws.len(), 6);
    // Towards the centre from the vertex, 10 / sqrt(2) * ... along the bisector.
    let dir = (v.to_screen(Point::new(50.0, 30.0)) - v.to_screen(hex[0])).normalized();
    let off = ws[0].at - v.to_screen(hex[0]);
    assert!((off.normalized() - dir).length() < 1e-4);
}

// A side under `corner.min_side` screen points hides the corner's widget, and zooming in shows it;
// an inner angle over `hide_above` hides it.
#[test]
fn corner_widgets_hide_on_short_sides_and_wide_angles() {
    let target = rect(1, 100.0, 20.0, 0.0, true);
    assert!(widgets(std::slice::from_ref(&target), &view(1.0), 10.0, 30.0, 180.0).is_empty());
    assert_eq!(widgets(std::slice::from_ref(&target), &view(2.0), 10.0, 30.0, 180.0).len(), 4);

    // A vertex of 170° shows under 180° and hides under 100°.
    let d = 10f64.to_radians();
    let pts = [Point::new(-100.0, 0.0), Point::new(0.0, 0.0), Point::new(100.0 * d.cos(), 100.0 * d.sin())];
    let flat = CornerTarget { key: 3, corners: vec![corner(pts[1], pts[0], pts[2], 0.0)], individual: false };
    assert_eq!(widgets(std::slice::from_ref(&flat), &view(1.0), 10.0, 30.0, 177.0).len(), 1);
    assert!(widgets(std::slice::from_ref(&flat), &view(1.0), 10.0, 30.0, 100.0).is_empty());
}

// A drag far past the limit gives each target's own maximum: `min(w, h) / 2` for rectangles, the
// tightest fillet for a triangle.
#[test]
fn corner_drag_caps_the_radius_at_the_fillet_limit() {
    let tri = [Point::new(0.0, 0.0), Point::new(100.0, 0.0), Point::new(50.0, 10.0)];
    let triangle = CornerTarget { key: 2, corners: polygon(&tri, 0.0), individual: false };
    let targets = [rect(1, 100.0, 60.0, 0.0, true), triangle];
    let edit = drag_edit(&targets, CornerRef { target: 1, corner: 0 }, 1e6, false);
    assert!(edit.targets[0].radii.iter().all(|r| (r - 30.0).abs() < 1e-9));
    let r = edit.targets[1].radii[0];
    assert!(r > 0.0 && r < 30.0);
    assert!(edit.targets[1].radii.iter().all(|x| *x == r));
}

// `dragged_radius` projects the move on the step and never goes below 0; `drag_edit` with
// `one_corner` changes one corner of a rectangle and every corner of a polygon.
#[test]
fn corner_drag_sets_every_target_and_alt_sets_one_corner() {
    let step = Vec2::new(1.0, 1.0);
    let press = Point::new(10.0, 10.0);
    assert!((dragged_radius(5.0, step, press, Point::new(14.0, 14.0)) - 9.0).abs() < 1e-9);
    assert_eq!(dragged_radius(5.0, step, press, Point::new(0.0, 0.0)), 0.0);
    assert_eq!(dragged_radius(5.0, step, press, press), 5.0);

    let sq = [Point::new(0.0, 0.0), Point::new(40.0, 0.0), Point::new(40.0, 40.0), Point::new(0.0, 40.0)];
    let polygon_target = CornerTarget { key: 2, corners: polygon(&sq, 0.0), individual: false };
    let targets = [rect(1, 100.0, 60.0, 4.0, true), polygon_target];
    let grabbed = CornerRef { target: 1, corner: 2 };
    let one = drag_edit(&targets, grabbed, 10.0, true);
    assert_eq!(one.targets[0].radii, vec![4.0, 4.0, 10.0, 4.0]);
    assert_eq!(one.targets[1].radii, vec![10.0; 4]);
    let all = drag_edit(&targets, grabbed, 10.0, false);
    assert_eq!(all.targets[0].radii, vec![10.0; 4]);
    // Unchanged targets are left out.
    assert!(drag_edit(&targets[..1], grabbed, 4.0, false).targets.is_empty());
}

// The nearest widget within 7 points wins; of two at the same distance the first target's.
#[test]
fn corner_hit_takes_the_nearest_widget_topmost_first() {
    let w = |target, x: f32| Widget { corner: CornerRef { target, corner: 0 }, at: pos2(x, 0.0), step: Vec2::new(1.0, 1.0), radius: 0.0 };
    let ws = [w(1, 10.0), w(2, 14.0), w(3, 18.0)];
    assert_eq!(hit(&ws, pos2(13.0, 0.0), 7.0), Some(1));
    assert_eq!(hit(&ws, pos2(12.0, 0.0), 7.0), Some(0));
    assert_eq!(hit(&ws, pos2(30.0, 0.0), 7.0), None);
}
