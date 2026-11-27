use std::sync::Arc;

use craft_ui_rt::{BezPath, Fill, FillRule, Rect, SizeU, Vec2, pos2, vec2};

use crate::common::{Painted, Scene, Stage, paint, px, renderer, rgba};

fn poly(points: &[(f64, f64)]) -> BezPath {
    let mut p = BezPath::new();
    p.move_to(points[0]);
    for &q in &points[1..] {
        p.line_to(q);
    }
    p.close_path();
    p
}

fn scene() -> Scene {
    Scene::new(renderer(), SizeU::new(220, 100), 1.0, |_, _| {
        // An L: the top-right quarter of the 50-pixel square at (10, 10) is missing.
        let l = poly(&[(10.0, 10.0), (35.0, 10.0), (35.0, 35.0), (60.0, 35.0), (60.0, 60.0), (10.0, 60.0)]);
        // A ring: two contours wound the same way, so only even-odd leaves the middle open.
        let mut ring = poly(&[(100.0, 10.0), (160.0, 10.0), (160.0, 70.0), (100.0, 70.0)]);
        ring.extend(poly(&[(115.0, 25.0), (145.0, 25.0), (145.0, 55.0), (115.0, 55.0)]));
        let red = rgba(255, 0, 0, 255);
        Stage {
            kids: vec![(
                Vec2::ZERO,
                Painted {
                    id: 1,
                    size: vec2(220.0, 100.0),
                    paint: paint(move |cx| {
                        cx.path(Arc::new(l.clone()), Some(Fill::new(red)), None);
                        cx.path(Arc::new(ring.clone()), Some(Fill { color: red, rule: FillRule::EvenOdd, convex: false }), None);
                        cx.fill_rect(Rect::from_min_size(pos2(0.0, 0.0), vec2(0.0, 0.0)), red);
                    }),
                },
            )],
        }
    })
}

#[test]
fn a_concave_fill_renders_inside_only() {
    let mut s = scene();
    s.frame();
    let img = s.pixels();
    let red = [255, 0, 0, 255];
    // The L: the arms are filled, the notch (top right) and everything outside is empty.
    for (x, y) in [(20, 20), (30, 30), (20, 50), (50, 45), (55, 55)] {
        assert_eq!(px(&img, x, y), red, "inside the L at ({x}, {y})");
    }
    for (x, y) in [(48, 20), (55, 15), (45, 25), (5, 5), (65, 65), (30, 70), (70, 45)] {
        assert_eq!(px(&img, x, y)[3], 0, "outside the L at ({x}, {y})");
    }
    // The edge is anti-aliased: no pixel two or more pixels outside the outline is touched.
    for x in 0..220u32 {
        for y in 0..100u32 {
            let near_l = (8..=62).contains(&x) && (8..=62).contains(&y);
            let near_ring = (98..=162).contains(&x) && (8..=72).contains(&y);
            if !near_l && !near_ring {
                assert_eq!(px(&img, x, y)[3], 0, "stray pixel at ({x}, {y})");
            }
        }
    }
    // The ring: the band is filled, the hole is not.
    assert_eq!(px(&img, 105, 40), red, "the ring's band");
    assert_eq!(px(&img, 130, 40)[3], 0, "the hole under even-odd");
    assert_eq!(px(&img, 130, 15), red);
}
