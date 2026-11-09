//! The crop frame's geometry: hits, resizing, turning, bounds, straighten, image mode and the guides.

use craft_behavior::CanvasView;
use craft_behavior::geometry::crop::{direction, drawn, turn};
use craft_behavior::geometry::{BoxPart, CropBox, CropGuide, GuideShow, crop_view_about, hit_rect};
use craft_keymap::Mods;
use kurbo::{Point, Rect, Size, Vec2};

/// The frame before any gesture.
const RECT: Rect = Rect::new(100.0, 80.0, 300.0, 220.0);

fn near(a: &CropBox, b: &CropBox) -> bool {
    a.corners().iter().zip(b.corners()).all(|(p, q)| p.distance(q) < 1e-6)
}

#[test]
fn hit_finds_handles_inside_and_outside_with_a_reach_that_shrinks_inside_small_frames() {
    let b = CropBox::upright(Rect::new(10.0, 10.0, 110.0, 60.0));
    let at = |x, y| b.part_at(Point::new(x, y), 4.0);
    assert_eq!(at(10.0, 10.0), BoxPart::Corner(0));
    assert_eq!(at(112.0, 61.0), BoxPart::Corner(2));
    assert_eq!(at(60.0, 8.0), BoxPart::Edge(0));
    assert_eq!(at(108.0, 30.0), BoxPart::Edge(1));
    assert_eq!(at(60.0, 30.0), BoxPart::Inside);
    assert_eq!(at(150.0, 30.0), BoxPart::Outside);
    assert_eq!(at(113.9, 30.0), BoxPart::Edge(1), "outside the full reach");
    assert_eq!(at(114.1, 30.0), BoxPart::Outside);
    // A tiny frame keeps a movable middle: inside, the reach is a quarter of the side.
    let tiny = CropBox::upright(Rect::new(0.0, 0.0, 4.0, 4.0));
    assert_eq!(tiny.part_at(Point::new(2.0, 2.0), 8.0), BoxPart::Inside);
    assert_eq!(tiny.part_at(Point::new(-7.0, 2.0), 8.0), BoxPart::Edge(3));
    assert_eq!(tiny.part_at(Point::new(0.9, 2.0), 8.0), BoxPart::Edge(3), "a quarter of the side: 1");
    assert_eq!(tiny.part_at(Point::new(1.1, 2.0), 8.0), BoxPart::Inside);
}

#[test]
fn a_turned_frame_is_hit_in_its_own_axes() {
    let b = CropBox::new(Rect::new(40.0, 30.0, 100.0, 70.0), 90.0);
    // A quarter turn: the frame covers x 50..90, y 20..80 on the page.
    assert_eq!(b.part_at(Point::new(70.0, 75.0), 2.0), BoxPart::Inside);
    assert_eq!(CropBox::upright(b.rect).part_at(Point::new(70.0, 75.0), 2.0), BoxPart::Outside);
    assert_eq!(b.part_at(Point::new(45.0, 50.0), 2.0), BoxPart::Outside);
    assert_eq!(b.part_at(Point::new(90.0, 20.0), 2.0), BoxPart::Corner(0), "the upright top left is now top right");
    assert_eq!(b.part_at(Point::new(70.0, 80.0), 2.0), BoxPart::Edge(1), "its right edge faces down");
    let want = [(90.0, 20.0), (90.0, 80.0), (50.0, 80.0), (50.0, 20.0)];
    for (c, (x, y)) in b.corners().iter().zip(want) {
        assert!(c.distance(Point::new(x, y)) < 1e-9, "{c:?}");
    }
    let bounds = b.turned_bounds();
    assert!((bounds.x0 - 50.0).abs() < 1e-9 && (bounds.y1 - 80.0).abs() < 1e-9, "{bounds:?}");
}

#[test]
fn handles_resize_with_shift_alt_and_a_ratio() {
    let d = Vec2::new;
    let resize = |r: Rect, part: BoxPart, by: Vec2, ratio: Option<f64>, alt: bool| CropBox::upright(r).resized(part, by, ratio, alt).rect;
    let r = Rect::new(20.0, 20.0, 100.0, 60.0);
    assert_eq!(resize(r, BoxPart::Edge(1), d(20.0, 10.0), None, false), Rect::new(20.0, 20.0, 120.0, 60.0));
    assert_eq!(resize(r, BoxPart::Corner(2), d(40.0, 10.0), None, false), Rect::new(20.0, 20.0, 140.0, 70.0));
    // A ratio: the larger relative extent wins at a corner, the opposite corner stays.
    let k = 120.0 / 50.0;
    let held = resize(Rect::new(20.0, 20.0, 140.0, 70.0), BoxPart::Corner(0), d(-10.0, -20.0), Some(k), false);
    assert_eq!((held.x1, held.y1), (140.0, 70.0));
    assert!((held.width() / held.height() - k).abs() < 1e-9, "{held:?}");
    assert!((held.height() - 70.0).abs() < 1e-9, "the larger relative extent: {held:?}");
    // Alt about the centre.
    assert_eq!(resize(Rect::new(40.0, 20.0, 80.0, 60.0), BoxPart::Edge(1), d(10.0, 0.0), None, true), Rect::new(30.0, 20.0, 90.0, 60.0));
    // A ratio on an edge: centred on the other axis.
    assert_eq!(resize(Rect::new(40.0, 20.0, 80.0, 60.0), BoxPart::Edge(1), d(20.0, 0.0), Some(1.0), false), Rect::new(40.0, 10.0, 100.0, 70.0));
    // Past the other side the frame flips.
    assert_eq!(resize(Rect::new(20.0, 20.0, 60.0, 60.0), BoxPart::Edge(1), d(-50.0, 0.0), None, false), Rect::new(10.0, 20.0, 20.0, 60.0));
}

#[test]
fn a_turned_frame_resizes_along_its_axes_and_keeps_the_opposite_edge() {
    let b = CropBox::new(Rect::new(40.0, 30.0, 100.0, 70.0), 90.0);
    // At 90 degrees the right edge faces down the page: dragging down 10 grows the width.
    let n = b.resized(BoxPart::Edge(1), Vec2::new(0.0, 10.0), None, false);
    assert!((n.rect.width() - 70.0).abs() < 1e-9 && (n.rect.height() - 40.0).abs() < 1e-9, "{n:?}");
    let (old, new) = (b.corners(), n.corners());
    assert!(old[0].distance(new[0]) < 1e-9 && old[3].distance(new[3]) < 1e-9, "the left edge stays on the page");
    assert!(near(&b.resized(BoxPart::Edge(1), Vec2::new(25.0, 0.0), None, false), &b), "a sideways drag does nothing to that handle");
    let plain = CropBox::upright(RECT).resized(BoxPart::Corner(2), Vec2::new(5.0, 6.0), None, false);
    assert_eq!(plain.rect, Rect::new(100.0, 80.0, 305.0, 226.0));
}

#[test]
fn a_new_frame_is_square_from_the_centre_or_holds_the_ratio() {
    let (a, b) = (Point::new(100.0, 100.0), Point::new(140.0, 120.0));
    assert_eq!(drawn(a, b, None, Mods::NONE), Rect::new(100.0, 100.0, 140.0, 120.0));
    assert_eq!(drawn(a, b, None, Mods::SHIFT), Rect::new(100.0, 100.0, 140.0, 140.0));
    assert_eq!(drawn(a, b, None, Mods::ALT), Rect::new(60.0, 80.0, 140.0, 120.0));
    assert_eq!(drawn(a, b, Some(2.0), Mods::NONE), Rect::new(100.0, 100.0, 140.0, 120.0));
    assert_eq!(drawn(a, Point::new(120.0, 140.0), Some(2.0), Mods::NONE), Rect::new(100.0, 100.0, 180.0, 140.0));
}

#[test]
fn turning_snaps_to_the_step_and_stays_in_range() {
    let c = Point::new(0.0, 0.0);
    let at = |deg: f64| Point::new(deg.to_radians().cos() * 10.0, deg.to_radians().sin() * 10.0);
    let turned = |from: f64, to: f64, step: Option<f64>| {
        CropBox::new(Rect::new(-5.0, -5.0, 5.0, 5.0), from).turned_to(direction(c, at(0.0)), at(to), step, false).angle
    };
    assert!((turned(0.0, 40.0, None) - 40.0).abs() < 1e-9);
    assert!((turned(0.0, 40.0, Some(15.0)) - 45.0).abs() < 1e-9);
    assert!((turned(170.0, 40.0, None) - -150.0).abs() < 1e-9, "wraps into -180..=180");
    assert!(turn(Point::new(1.0, 0.0), c, 90.0).distance(Point::new(0.0, 1.0)) < 1e-12, "clockwise on screen");
}

#[test]
fn bounds_cut_an_unturned_frame_and_move_a_turned_one() {
    let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);
    let out = CropBox::upright(Rect::new(-20.0, 10.0, 450.0, 200.0));
    assert_eq!(out.within(bounds, true, false).unwrap().rect, Rect::new(0.0, 10.0, 400.0, 200.0));
    assert_eq!(CropBox::upright(Rect::new(-20.0, 10.0, 80.0, 60.0)).within(bounds, false, false).unwrap().rect, Rect::new(0.0, 10.0, 100.0, 60.0));
    let turned = CropBox::new(Rect::new(-30.0, 100.0, 70.0, 150.0), 90.0);
    let moved = turned.within(bounds, false, false).unwrap();
    assert!(moved.turned_bounds().x0 >= -1e-9 && moved.angle == 90.0);
    assert!(turned.within(bounds, true, false).is_none(), "a resize that leaves is refused");
}

#[test]
fn a_held_ratio_is_never_cut_by_the_bounds() {
    let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);
    let wide = CropBox::upright(Rect::new(100.0, 80.0, 500.0, 280.0));
    assert_eq!(wide.within(bounds, true, false).unwrap().rect, Rect::new(100.0, 80.0, 400.0, 280.0));
    assert!(wide.within(bounds, true, true).is_none(), "the cut would change the 2:1 ratio");
    assert!(CropBox::upright(Rect::new(100.0, 80.0, 300.0, 180.0)).within(bounds, true, true).is_some());
}

// Straighten.

#[test]
fn a_line_levels_a_horizon_and_plumbs_a_nearer_vertical_one() {
    let from = Point::new(60.0, 60.0);
    let polar = |len: f64, deg: f64| from + Vec2::new(deg.to_radians().cos() * len, deg.to_radians().sin() * len);
    // Falling to the right, rising to the right, drawn leftwards, a wall leaning either way.
    let cases = [(10.0, 10.0), (-7.0, -7.0), (170.0, -10.0), (95.0, 5.0), (85.0, -5.0), (-85.0, 5.0), (44.0, 44.0), (0.0, 0.0), (90.0, 0.0)];
    for (line, want) in cases {
        let got = CropBox::level_angle(from, polar(120.0, line)).unwrap();
        assert!((got - want).abs() < 1e-3, "line at {line}: {got}");
    }
}

#[test]
fn fitted_is_the_largest_frame_of_the_same_proportions_inside_the_canvas() {
    let canvas = Size::new(200.0, 100.0);
    let frame = CropBox::upright(Rect::new(0.0, 0.0, 200.0, 100.0));
    assert_eq!(frame.fitted(canvas, 0.0), Some(frame));
    for deg in [-45.0, -10.0, 3.0, 10.0, 30.0, 45.0, 90.0] {
        let f = frame.fitted(canvas, deg).unwrap();
        let inside = |b: &CropBox| b.corners().iter().all(|p| p.x >= -1e-6 && p.y >= -1e-6 && p.x <= 200.0 + 1e-6 && p.y <= 100.0 + 1e-6);
        assert!(inside(&f), "{deg}: {f:?}");
        assert!((f.rect.width() / f.rect.height() - 2.0).abs() < 1e-9, "keeps 2:1 at {deg}");
        assert!(f.rect.center().distance(Point::new(100.0, 50.0)) < 1e-9, "centred at {deg}");
        assert_eq!(f.angle, deg);
        let c = f.rect.center();
        let bigger = CropBox::new(
            Rect::new(c.x - f.rect.width() * 0.505, c.y - f.rect.height() * 0.505, c.x + f.rect.width() * 0.505, c.y + f.rect.height() * 0.505),
            deg,
        );
        assert!(!inside(&bigger), "{deg}: 1% larger still fits");
    }
    // A 100x100 frame in a 200x100 canvas at 0 degrees is as large as the canvas allows.
    let square = CropBox::upright(Rect::new(10.0, 10.0, 110.0, 110.0)).fitted(canvas, 0.0).unwrap();
    assert_eq!(square.rect, Rect::new(50.0, 0.0, 150.0, 100.0));
    // A flipped rect keeps its proportions.
    assert_eq!(CropBox::upright(Rect::new(200.0, 100.0, 0.0, 0.0)).fitted(canvas, 0.0).unwrap().rect, Rect::new(0.0, 0.0, 200.0, 100.0));
    for bad in [Size::new(0.0, 100.0), Size::new(200.0, -1.0), Size::new(f64::NAN, 100.0)] {
        assert_eq!(frame.fitted(bad, 0.0), None);
    }
    assert_eq!(frame.fitted(canvas, f64::NAN), None);
    assert_eq!(CropBox::upright(Rect::new(1.0, 1.0, 1.0, 5.0)).fitted(canvas, 0.0), None);
}

// Image mode.

#[test]
fn the_view_for_a_frame_keeps_the_pivot_on_screen_and_round_trips() {
    let v0 = CanvasView::new(emath::Rect::from_min_size(emath::pos2(5.0, 7.0), emath::vec2(400.0, 300.0)), Point::new(200.0, 150.0), 1.5)
        .rotation(10.0)
        .flip_x(true);
    let b = CropBox::new(RECT, 37.0);
    let pivot = Point::new(120.0, 90.0);
    let v = crop_view_about(&b, &v0, pivot);
    assert!((v.rotation + 37.0).abs() < 1e-9);
    assert!(v.to_screen(pivot).distance(v0.to_screen(pivot)) < 1e-3);
    let back = crop_view_about(&CropBox::new(RECT, -10.0), &v, pivot);
    assert!((back.rotation - 10.0).abs() < 1e-9 && (back.center - v0.center).hypot() < 1e-6, "{back:?}");
    let upright = craft_behavior::geometry::crop_view(&b, &v0);
    let q = b.corners().map(|p| upright.to_screen(p));
    assert!((q[0].y - q[1].y).abs() < 1e-3 && (q[0].x - q[3].x).abs() < 1e-3, "{q:?}");
}

// Guides.

fn points(g: &[Vec<Point>]) -> impl Iterator<Item = Point> + '_ {
    g.iter().flatten().copied()
}

fn same(a: Point, b: Point) -> bool {
    a.distance(b) < 1e-6
}

#[test]
fn every_guide_stays_in_the_unit_square() {
    for kind in CropGuide::ALL {
        for o in 0..8 {
            for (w, h) in [(1.0, 1.0), (300.0, 200.0), (200.0, 300.0), (5000.0, 3.0), (3.0, 5000.0), (1e7, 1e7), (1e30, 2.0)] {
                let g = kind.guides(o, w, h, 32.0, 96);
                assert!(!g.is_empty(), "{kind:?} {w}x{h}");
                for p in points(&g) {
                    assert!([p.x, p.y].iter().all(|v| v.is_finite() && (-1e-9..=1.0 + 1e-9).contains(v)), "{kind:?} o{o} {w}x{h}: {p:?}");
                }
                assert!(g.iter().all(|l| l.len() >= 2));
            }
        }
    }
}

#[test]
fn diagonals_run_at_45_degrees_on_screen() {
    let (w, h) = (300.0, 200.0);
    for l in CropGuide::Diagonal.guides(0, w, h, 32.0, 96) {
        let (dx, dy) = ((l[1].x - l[0].x) * w, (l[1].y - l[0].y) * h);
        assert!((dx.abs() - dy.abs()).abs() < 1e-6 && (dx.abs() - 200.0).abs() < 1e-6, "{l:?}");
    }
    // A square frame: corner to corner.
    let g = CropGuide::Diagonal.guides(0, 100.0, 100.0, 32.0, 96);
    assert_eq!(g[0], vec![Point::new(0.0, 0.0), Point::new(1.0, 1.0)]);
}

#[test]
fn triangle_perpendiculars_meet_the_diagonal_at_right_angles() {
    let (w, h) = (300.0, 200.0);
    let g = CropGuide::Triangle.guides(0, w, h, 32.0, 96);
    for l in &g[1..] {
        let d = [(l[1].x - l[0].x) * w, (l[1].y - l[0].y) * h];
        assert!((d[0] * w + d[1] * h).abs() < 1e-6, "{l:?}");
        assert!((l[1].x - l[1].y).abs() < 1e-9, "foot on the diagonal: {l:?}");
    }
    // The other orientation mirrors it: the diagonal runs top-right to bottom-left.
    let m = CropGuide::Triangle.guides(1, w, h, 32.0, 96);
    assert_eq!(m[0], vec![Point::new(1.0, 0.0), Point::new(0.0, 1.0)]);
    for (a, b) in g.iter().zip(&m) {
        assert!(a.iter().zip(b).all(|(p, q)| same(*p, Point::new(1.0 - q.x, q.y))));
    }
    // Two orientations: 2 is 0 again.
    assert_eq!(CropGuide::Triangle.guides(2, w, h, 32.0, 96), g);
}

#[test]
fn golden_spiral_orientations_mirror_it_into_each_corner() {
    let base = CropGuide::GoldenSpiral.guides(0, 300.0, 200.0, 32.0, 96);
    let pts = &base[0];
    assert!(pts.len() > 40);
    // It starts at the bottom-left corner and winds in towards the golden point (lower right).
    assert_eq!(pts[0], Point::new(0.0, 1.0));
    let end = pts[pts.len() - 1];
    assert!((end.x - 0.7236).abs() < 0.02 && (end.y - 0.7236).abs() < 0.02, "{end:?}");
    for o in 1..4u8 {
        let g = CropGuide::GoldenSpiral.guides(o, 300.0, 200.0, 32.0, 96);
        for (p, q) in pts.iter().zip(&g[0]) {
            let want = Point::new(if o & 1 != 0 { 1.0 - p.x } else { p.x }, if o & 2 != 0 { 1.0 - p.y } else { p.y });
            assert!(same(want, *q));
        }
    }
    assert_eq!(CropGuide::GoldenSpiral.guides(4, 300.0, 200.0, 32.0, 96), base);
    // Symmetric guides ignore the orientation.
    assert_eq!(CropGuide::Thirds.guides(3, 300.0, 200.0, 32.0, 96), CropGuide::Thirds.guides(0, 300.0, 200.0, 32.0, 96));
}

#[test]
fn persisted_names_are_camel_case() {
    assert_eq!(serde_json::to_value(CropGuide::GoldenSpiral).unwrap(), "goldenSpiral");
    assert_eq!(serde_json::to_value(GuideShow::Always).unwrap(), "always");
}
