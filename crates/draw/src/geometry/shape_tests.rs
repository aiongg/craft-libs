//! Tests of shape outlines and drags.

use std::f64::consts::{FRAC_PI_2, PI};

use kurbo::{PathEl, Point, Rect, RoundedRect, Shape};

use super::shape::{RadialFrame, STAR_RATIO, ShapeGeometry, box_drag, line_drag};

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

// `box_drag`: Shift makes the box square on the larger extent, on the side the pointer is; Alt grows it about the
// press; the corners are whole pixels; a drag under one pixel draws nothing (also for `line_drag`).
#[test]
fn shape_drag_rules() {
    let start = Point::new(100.0, 100.0);
    for to in [Point::new(130.0, 110.0), Point::new(90.0, 60.0)] {
        let square = box_drag(start, to, true, false).expect("a box");
        assert!(near(square.width(), square.height()));
        assert!(square.contains(start) && (square.x0 == start.x || square.x1 == start.x) && (square.y0 == start.y || square.y1 == start.y));
        assert_eq!((square.x0 < start.x, square.y0 < start.y), (to.x < start.x, to.y < start.y), "on the pointer's side");
    }
    let about = box_drag(start, Point::new(110.0, 105.0), false, true).expect("a box");
    assert_eq!(about.center(), start);
    let whole = box_drag(Point::new(0.4, 0.4), Point::new(10.4, 20.6), false, false).expect("a box");
    assert!([whole.x0, whole.y0, whole.x1, whole.y1].iter().all(|v| v.fract() == 0.0));
    assert_eq!(box_drag(Point::new(10.0, 10.0), Point::new(10.4, 50.0), false, false), None);
    assert_eq!(line_drag(Point::new(10.0, 10.0), Point::new(10.9, 10.0)), None);
}

fn corners(path: &kurbo::BezPath) -> Vec<Point> {
    path.elements()
        .iter()
        .filter_map(|e| match e {
            PathEl::MoveTo(p) | PathEl::LineTo(p) => Some(*p),
            _ => None,
        })
        .collect()
}

// `Polygon` puts its points on the ellipse of the rect, the first at the top (PhotoCraft
// `vector/src/shapes.rs:96-112`): a triangle's base sits at cy + ry/2, a pentagon's points follow
// `-90° + 360° i / 5`; a Rect with four equal radii is kurbo's rounded rect of that radius.
#[test]
fn polygon_points_lie_on_the_rect_ellipse_and_rounded_rectangle_has_four_equal_radii() {
    let rect = Rect::new(10.0, 20.0, 110.0, 80.0);
    let (cx, cy, rx, ry) = (60.0, 50.0, 50.0, 30.0);
    for sides in [3_u32, 5] {
        let path = ShapeGeometry::Polygon { rect, sides, star_ratio: 1.0 }.to_path(0.1);
        let points = corners(&path);
        assert_eq!(points.len(), sides as usize);
        for (i, p) in points.iter().enumerate() {
            let a = -FRAC_PI_2 + 2.0 * PI * i as f64 / f64::from(sides);
            assert!(near(p.x, cx + rx * a.cos()) && near(p.y, cy + ry * a.sin()), "{sides} sides, point {i}: {p:?}");
        }
    }
    let triangle = corners(&ShapeGeometry::Polygon { rect, sides: 3, star_ratio: 1.0 }.to_path(0.1));
    assert!(near(triangle[0].y, rect.y0) && near(triangle[1].y, cy + ry / 2.0));

    let round = ShapeGeometry::Rect { rect, radii: [8.0; 4] }.to_path(0.1);
    assert_eq!(round, RoundedRect::from_rect(rect, 8.0).to_path(0.1));
    assert_eq!(ShapeGeometry::Rect { rect, radii: [0.0; 4] }.to_path(0.1), rect.to_path(0.1));
}

// A Rect with radii [1, 2, 3, 4] rounds top-left 1, top-right 2, bottom-right 3, bottom-left 4.
#[test]
fn shape_geometry_to_path_matches_corner_order_tl_tr_br_bl() {
    let rect = Rect::new(0.0, 0.0, 100.0, 100.0);
    let path = ShapeGeometry::Rect { rect, radii: [1.0, 2.0, 3.0, 4.0] }.to_path(0.01);
    // The first line segment's start is where the top-left arc ends: x = its radius.
    assert_eq!(path.elements()[0], PathEl::MoveTo(Point::new(0.0, 1.0)));
    let bbox = path.bounding_box();
    assert!(near(bbox.x0, 0.0) && near(bbox.x1, 100.0) && near(bbox.y1, 100.0));
    // Each corner point is cut off by its radius: the path passes (100 - 2, 0) and (0, 100 - 4).
    let on = |p: Point| path.segments().any(|s| kurbo::ParamCurveNearest::nearest(&s, p, 1e-6).distance_sq < 1e-8);
    assert!(on(Point::new(98.0, 0.0)) && on(Point::new(100.0, 97.0)) && on(Point::new(4.0, 100.0)) && on(Point::new(1.0, 0.0)));
}

// `RadialFrame::drag` follows the pointer's direction; with `upright` the rotation is 0.
// `ratio_holding` keeps the inner radius; `straight_shoulders(5)` is cos 72° / cos 36° and
// `straight_shoulders(4)` is `None` (VectorCraft `straight_ratio`).
#[test]
fn radial_frame_rotation_inner_radius_and_straight_shoulders() {
    let c = Point::new(50.0, 50.0);
    let Some(up) = RadialFrame::drag(c, Point::new(50.0, 30.0), false) else { panic!("a frame") };
    assert!(near(up.radius, 20.0) && near(up.rotation, 0.0));
    // A pointer left of the centre gives a rotation in 0..TAU, not a negative turn.
    let Some(left) = RadialFrame::drag(c, Point::new(30.0, 50.0), false) else { panic!("a frame") };
    assert!(near(left.rotation, 3.0 * FRAC_PI_2));
    let Some(right) = RadialFrame::drag(c, Point::new(70.0, 50.0), false) else { panic!("a frame") };
    assert!(near(right.rotation, FRAC_PI_2));
    let Some(upright) = RadialFrame::drag(c, Point::new(70.0, 50.0), true) else { panic!("a frame") };
    assert!(near(upright.rotation, 0.0));
    assert!(RadialFrame::drag(c, Point::new(50.4, 50.0), false).is_none());

    assert!(near(right.ratio_holding(10.0), 0.5));
    assert!(near(right.ratio_holding(100.0), *STAR_RATIO.end()));
    assert!(near(RadialFrame::straight_shoulders(5).unwrap_or(0.0), (2.0 * PI / 5.0).cos() / (PI / 5.0).cos()));
    assert_eq!(RadialFrame::straight_shoulders(4), None);

    // The first outer point of a rotated polygon sits at the frame's direction.
    let path = right.geometry(5, 1.0).to_path(0.1);
    let first = corners(&path)[0];
    assert!(near(first.x, 70.0) && near(first.y, 50.0));
}
