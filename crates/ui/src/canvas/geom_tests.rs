//! Canvas geometry.
use kurbo::{Point, Rect, Vec2};

use super::geom::{NEAR_W, angle_deg, clip_segment, point_in_polygon, project_clipped};

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

#[test]
fn nonzero_winding_fills_a_self_crossing_outline() {
    let square = [p(0.0, 0.0), p(4.0, 0.0), p(4.0, 4.0), p(0.0, 4.0)];
    assert!(point_in_polygon(p(2.0, 2.0), &square));
    assert!(!point_in_polygon(p(5.0, 2.0), &square));
    // A figure eight: both loops wind the same way round.
    let eight = [p(0.0, 0.0), p(4.0, 4.0), p(4.0, 0.0), p(0.0, 4.0)];
    assert!(point_in_polygon(p(3.5, 2.0), &eight));
    assert!(point_in_polygon(p(0.5, 2.0), &eight));
}

#[test]
fn segments_clip_to_the_rect() {
    let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
    assert_eq!(clip_segment(p(-5.0, 5.0), p(15.0, 5.0), rect), Some((p(0.0, 5.0), p(10.0, 5.0))));
    assert_eq!(clip_segment(p(-5.0, -5.0), p(-1.0, 20.0), rect), None);
    assert_eq!(clip_segment(p(2.0, 3.0), p(7.0, 8.0), rect), Some((p(2.0, 3.0), p(7.0, 8.0))));
}

#[test]
fn angles_run_clockwise_on_screen() {
    assert_eq!(angle_deg(Vec2::new(1.0, 0.0)), 0.0);
    assert_eq!(angle_deg(Vec2::new(0.0, 1.0)), 90.0);
    assert_eq!(angle_deg(Vec2::new(-1.0, 0.0)), 180.0);
}

#[test]
fn a_polygon_behind_the_camera_is_cut() {
    let square = [p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0), p(0.0, 10.0)];
    let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    assert_eq!(project_clipped(&identity, &square), square.to_vec());
    // w = 1 − 0.15 x: 1 at the left edge and −0.5 at the right, which is behind the camera.
    let tilted = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [-0.15, 0.0, 1.0]];
    let cut = project_clipped(&tilted, &square);
    assert_eq!(cut.len(), 4);
    // The cut lies where w = NEAR_W, at x = 0.5 / 0.15, which projects to that x over NEAR_W.
    let cut_x = (1.0 - NEAR_W) / 0.15 / NEAR_W;
    let right = cut.iter().map(|q| q.x).fold(f64::MIN, f64::max);
    assert!((right - cut_x).abs() < 1e-9, "{right}");
    let all_behind = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.1]];
    assert!(project_clipped(&all_behind, &square).is_empty());
}
