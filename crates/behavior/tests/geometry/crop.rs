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
fn crop_handle_hit() {
    let rect = Rect::new(10.0, 10.0, 110.0, 60.0);
    let b = CropBox::upright(rect);
    let reach = 4.0;
    let at = |p: Point| b.part_at(p, reach);
    let mid = Point::new(60.0, 35.0);
    assert_eq!(at(mid), BoxPart::Inside);
    assert_eq!(at(Point::new(rect.x0, rect.y0)), BoxPart::Corner(0));
    assert_eq!(at(Point::new(rect.x1, rect.y1)), BoxPart::Corner(2));
    assert_eq!(at(Point::new(mid.x, rect.y0)), BoxPart::Edge(0));
    assert_eq!(at(Point::new(rect.x1, mid.y)), BoxPart::Edge(1));
    // A handle reaches outside the frame by the reach, and not beyond.
    assert_eq!(at(Point::new(rect.x1 + reach - 0.1, mid.y)), BoxPart::Edge(1));
    assert_eq!(at(Point::new(rect.x1 + 2.0 * reach, mid.y)), BoxPart::Outside);
    // A tiny frame keeps a movable middle: inside, the reach is shrunk to a part of the side.
    let tiny = CropBox::upright(Rect::new(0.0, 0.0, 4.0, 4.0));
    assert_eq!(tiny.part_at(Point::new(2.0, 2.0), 8.0), BoxPart::Inside);
    assert_eq!(tiny.part_at(Point::new(-7.0, 2.0), 8.0), BoxPart::Edge(3));
    // A turned frame is hit in its own axes.
    let turned = CropBox::new(Rect::new(40.0, 30.0, 100.0, 70.0), 90.0);
    let centre = turned.center();
    assert_eq!(turned.part_at(centre, 2.0), BoxPart::Inside);
    for (i, corner) in turned.corners().iter().enumerate() {
        assert_eq!(turned.part_at(*corner, 2.0), BoxPart::Corner(i as u8), "corner {i} is where the frame's own corner is");
    }
    let bounds = turned.turned_bounds();
    assert!(
        turned.corners().iter().all(|c| c.x >= bounds.x0 - 1e-9 && c.x <= bounds.x1 + 1e-9 && c.y >= bounds.y0 - 1e-9 && c.y <= bounds.y1 + 1e-9)
    );
    let inside_turned_only = Point::new(centre.x, centre.y + 0.8 * (bounds.y1 - centre.y));
    assert_eq!(turned.part_at(inside_turned_only, 2.0), BoxPart::Inside);
    assert_eq!(
        CropBox::upright(turned.rect).part_at(inside_turned_only, 2.0),
        BoxPart::Outside,
        "the upright frame misses what the turned one covers"
    );
}

#[test]
fn crop_resize() {
    let d = Vec2::new;
    let resize = |r: Rect, part: BoxPart, by: Vec2, ratio: Option<f64>, alt: bool| CropBox::upright(r).resized(part, by, ratio, alt).rect;
    let r = Rect::new(20.0, 20.0, 100.0, 60.0);
    // An edge moves by the drag and the opposite edge stays; a corner moves both of its edges.
    let edge = resize(r, BoxPart::Edge(1), d(20.0, 10.0), None, false);
    assert_eq!((edge.x1 - r.x1, edge.x0, edge.y0, edge.y1), (20.0, r.x0, r.y0, r.y1));
    let corner = resize(r, BoxPart::Corner(2), d(40.0, 10.0), None, false);
    assert_eq!((corner.x1 - r.x1, corner.y1 - r.y1, corner.x0, corner.y0), (40.0, 10.0, r.x0, r.y0));
    // A ratio holds, and the opposite corner stays.
    let k = 120.0 / 50.0;
    let start = Rect::new(20.0, 20.0, 140.0, 70.0);
    let held = resize(start, BoxPart::Corner(0), d(-10.0, -20.0), Some(k), false);
    assert_eq!((held.x1, held.y1), (start.x1, start.y1));
    assert!((held.width() / held.height() - k).abs() < 1e-9, "{held:?}");
    // Alt resizes about the centre.
    let centred = resize(Rect::new(40.0, 20.0, 80.0, 60.0), BoxPart::Edge(1), d(10.0, 0.0), None, true);
    assert_eq!(centred.center(), Rect::new(40.0, 20.0, 80.0, 60.0).center());
    assert!(centred.width() > 40.0);
    // A ratio on an edge centres the frame on the other axis.
    let edge_ratio = resize(Rect::new(40.0, 20.0, 80.0, 60.0), BoxPart::Edge(1), d(20.0, 0.0), Some(1.0), false);
    assert_eq!(edge_ratio.center().y, 40.0);
    // Past the other side the frame flips instead of going negative.
    let flipped = resize(Rect::new(20.0, 20.0, 60.0, 60.0), BoxPart::Edge(1), d(-50.0, 0.0), None, false);
    assert!(flipped.width() > 0.0 && flipped.x1 <= 20.0);
    // A turned frame resizes along its own axes and keeps the opposite edge.
    let b = CropBox::new(Rect::new(40.0, 30.0, 100.0, 70.0), 90.0);
    let n = b.resized(BoxPart::Edge(1), Vec2::new(0.0, 10.0), None, false);
    assert!(n.rect.width() > b.rect.width() && n.rect.height() == b.rect.height(), "dragging along the right edge's facing grows the width");
    let (old, new) = (b.corners(), n.corners());
    assert!(old[0].distance(new[0]) < 1e-9 && old[3].distance(new[3]) < 1e-9, "the opposite edge stays on the page");
    assert!(near(&b.resized(BoxPart::Edge(1), Vec2::new(25.0, 0.0), None, false), &b), "a sideways drag does nothing to that handle");
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
fn bounds_rules() {
    let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);
    let inside = |r: Rect| r.x0 >= bounds.x0 && r.y0 >= bounds.y0 && r.x1 <= bounds.x1 && r.y1 <= bounds.y1;
    // An unturned frame is cut by a resize and moved by a move.
    let out = CropBox::upright(Rect::new(-20.0, 10.0, 450.0, 200.0));
    assert!(inside(out.within(bounds, true, false).unwrap().rect));
    let pushed = CropBox::upright(Rect::new(-20.0, 10.0, 80.0, 60.0));
    let moved = pushed.within(bounds, false, false).unwrap().rect;
    assert!(inside(moved) && moved.width() == pushed.rect.width());
    // A turned frame is moved; a resize that leaves is refused.
    let turned = CropBox::new(Rect::new(-30.0, 100.0, 70.0, 150.0), 90.0);
    let moved = turned.within(bounds, false, false).unwrap();
    assert!(moved.turned_bounds().x0 >= -1e-9 && moved.angle == 90.0);
    assert!(turned.within(bounds, true, false).is_none());
    // A turn stops at the bounds.
    let (tight, roomy) = (Rect::new(100.0, 80.0, 300.0, 220.0), Rect::new(0.0, 0.0, 400.0, 300.0));
    let frame = CropBox::upright(RECT);
    let grab = direction(frame.center(), Point::new(340.0, 260.0));
    let turned = frame.turned_to(grab, Point::new(300.0, 290.0), None, false);
    assert!(turned.angle.abs() > 1.0);
    assert!(turned.within(tight, true, false).is_none(), "a turn that would leave the bounds is refused");
    let t = turned.within(roomy, true, false).unwrap().turned_bounds();
    assert!(t.x0 >= 0.0 && t.y0 >= 0.0 && t.x1 <= 400.0 && t.y1 <= 300.0, "{t:?}");
}

#[test]
fn a_held_ratio_is_never_cut_by_the_bounds() {
    let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);
    let wide = CropBox::upright(Rect::new(100.0, 80.0, 500.0, 280.0));
    assert_eq!(wide.within(bounds, true, false).unwrap().rect, Rect::new(100.0, 80.0, 400.0, 280.0));
    assert!(wide.within(bounds, true, true).is_none(), "the cut would change the 2:1 ratio");
    assert!(CropBox::upright(Rect::new(100.0, 80.0, 300.0, 180.0)).within(bounds, true, true).is_some());
}

#[test]
fn crop_size_is_whole_units_of_at_least_one() {
    assert_eq!(CropBox::upright(Rect::new(0.0, 0.0, 10.4, 0.2)).crop_size(), Some((10.0, 1.0)));
    assert_eq!(CropBox::upright(Rect::new(0.0, 0.0, f64::NAN, 5.0)).crop_size(), None);
    assert_eq!(CropBox::new(Rect::new(0.0, 0.0, 30.0, 10.0), 20.0).swapped().rect, Rect::new(10.0, -10.0, 20.0, 20.0));
}

#[test]
fn the_rect_hit_is_the_frames_hit() {
    let b = CropBox::upright(RECT);
    for p in [(150.0, 110.0), (300.0, 220.0), (205.0, 76.0), (100.0, 150.0), (20.0, 20.0), (306.0, 150.0), (320.0, 150.0)] {
        let p = Point::new(p.0, p.1);
        assert_eq!(b.part_at(p, 8.0), hit_rect(RECT, p, 8.0), "{p:?}");
    }
    assert_eq!(hit_rect(RECT, Point::new(100.0, 80.0), 8.0), BoxPart::Corner(0));
    assert_eq!(hit_rect(RECT, Point::new(200.0, 224.0), 8.0), BoxPart::Edge(2));
    assert_eq!(hit_rect(RECT, Point::new(20.0, 20.0), 8.0), BoxPart::Outside);
}

// Straighten.

#[test]
fn level_line_angle() {
    // a line levels a horizon and plumbs a nearer vertical one
    {
        let from = Point::new(60.0, 60.0);
        let polar = |len: f64, deg: f64| from + Vec2::new(deg.to_radians().cos() * len, deg.to_radians().sin() * len);
        // Falling to the right, rising to the right, drawn leftwards, a wall leaning either way.
        let cases = [(10.0, 10.0), (-7.0, -7.0), (170.0, -10.0), (95.0, 5.0), (85.0, -5.0), (-85.0, 5.0), (44.0, 44.0), (0.0, 0.0), (90.0, 0.0)];
        for (line, want) in cases {
            let got = CropBox::level_angle(from, polar(120.0, line)).unwrap();
            assert!((got - want).abs() < 1e-3, "line at {line}: {got}");
        }
    }
    // a line is levelled in the document not on the screen
    {
        // A page turned a quarter: a line along the screen's horizontal is along the document's vertical.
        let view = CanvasView::new(emath::Rect::from_min_size(emath::pos2(0.0, 0.0), emath::vec2(400.0, 300.0)), Point::new(200.0, 150.0), 1.0)
            .rotation(90.0);
        let a = emath::pos2(200.0, 150.0);
        let level = |to: emath::Pos2| CropBox::level_angle(view.to_doc(a), view.to_doc(to)).unwrap();
        assert!(level(a + emath::vec2(100.0, 0.0)).abs() < 1e-3);
        assert!((level(a + emath::vec2(100.0, 10.0)).abs() - (10.0f64 / 100.0).atan().to_degrees()).abs() < 1e-3);
    }
}

#[test]
fn level_angle_ignores_points_and_non_finite_lines() {
    let p = Point::new(5.0, 5.0);
    assert_eq!(CropBox::level_angle(p, p), None);
    assert_eq!(CropBox::level_angle(p, Point::new(f64::NAN, 1.0)), None);
    assert_eq!(CropBox::level_angle(p, Point::new(f64::INFINITY, 1.0)), None);
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
fn guides_stay_inside_and_degenerate_frames_have_none() {
    // every guide stays in the unit square
    {
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
    // degenerate frames have no guides
    {
        for kind in CropGuide::ALL {
            for (w, h) in [(0.0, 10.0), (10.0, 0.0), (f64::NAN, 10.0), (f64::INFINITY, 10.0), (-5.0, -5.0), (0.5, 100.0)] {
                assert!(kind.guides(0, w, h, 32.0, 96).is_empty(), "{kind:?} {w}x{h}");
            }
        }
    }
}

#[test]
fn guide_line_counts() {
    let xs = |g: Vec<Vec<Point>>| -> Vec<f64> { g.iter().filter(|l| l[0].y == 0.0 && l[1].y == 1.0).map(|l| l[0].x).collect() };
    let t = xs(CropGuide::Thirds.guides(0, 300.0, 200.0, 32.0, 96));
    assert_eq!(t.len(), 2);
    assert!((t[0] - 1.0 / 3.0).abs() < 1e-9 && (t[1] - 2.0 / 3.0).abs() < 1e-9);
    let golden = CropGuide::GoldenRatio.guides(0, 300.0, 200.0, 32.0, 96);
    let g = xs(golden.clone());
    assert!((g[0] - 0.381_966).abs() < 1e-5 && (g[1] - 0.618_034).abs() < 1e-5, "{g:?}");
    assert_eq!(golden.len(), 4);
    // The grid has about one line per step on screen, never fewer than one and never more than the cap.
    let count = |g: &[Vec<Point>], vertical: bool| g.iter().filter(|l| (l[0].x == l[1].x) == vertical).count();
    for (w, h) in [(10.0, 10.0), (64.0, 64.0), (640.0, 320.0), (1e6, 1e6)] {
        let g = CropGuide::Grid.guides(0, w, h, 32.0, 96);
        let (v, hz) = (count(&g, true), count(&g, false));
        assert!((1..96).contains(&v) && (1..96).contains(&hz), "{w}x{h}: {v} {hz}");
    }
    let fine = count(&CropGuide::Grid.guides(0, 640.0, 320.0, 32.0, 96), true);
    let coarse = count(&CropGuide::Grid.guides(0, 640.0, 320.0, 64.0, 96), true);
    assert!(coarse < fine, "a larger step gives fewer lines");
    assert!(count(&CropGuide::Grid.guides(0, 640.0, 320.0, 64.0, 4), true) <= 4, "the cap bounds the count");
}

#[test]
fn diagonal_guides_are_geometrically_true() {
    // diagonals run at 45 degrees on screen
    {
        let (w, h) = (300.0, 200.0);
        for l in CropGuide::Diagonal.guides(0, w, h, 32.0, 96) {
            let (dx, dy) = ((l[1].x - l[0].x) * w, (l[1].y - l[0].y) * h);
            assert!((dx.abs() - dy.abs()).abs() < 1e-6 && (dx.abs() - 200.0).abs() < 1e-6, "{l:?}");
        }
        // A square frame: corner to corner.
        let g = CropGuide::Diagonal.guides(0, 100.0, 100.0, 32.0, 96);
        assert_eq!(g[0], vec![Point::new(0.0, 0.0), Point::new(1.0, 1.0)]);
    }
    // triangle perpendiculars meet the diagonal at right angles
    {
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
fn cycling_guides_and_orientations() {
    let mut kind = CropGuide::default();
    assert_eq!((kind, GuideShow::default()), (CropGuide::Thirds, GuideShow::Auto));
    let mut seen = vec![kind];
    for _ in 0..6 {
        kind = kind.next();
        seen.push(kind);
    }
    assert_eq!(&seen[..6], &CropGuide::ALL);
    assert_eq!(seen[6], CropGuide::Thirds, "wraps around");
    assert_eq!(CropGuide::Thirds.next_orientation(0), 0, "thirds has one orientation");
    let turns: Vec<u8> = (0..5)
        .scan(0u8, |o, _| {
            *o = CropGuide::GoldenSpiral.next_orientation(*o);
            Some(*o)
        })
        .collect();
    assert_eq!(turns, [1, 2, 3, 0, 1]);
    // A stray orientation still cycles in range.
    assert!(CropGuide::Triangle.next_orientation(200) < 2);
}

#[test]
fn persisted_names_are_camel_case() {
    assert_eq!(serde_json::to_value(CropGuide::GoldenSpiral).unwrap(), "goldenSpiral");
    assert_eq!(serde_json::to_value(GuideShow::Always).unwrap(), "always");
}
