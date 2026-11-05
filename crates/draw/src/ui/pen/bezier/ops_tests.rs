//! Tests of the path operations.

use kurbo::{ParamCurve, Point, Vec2};

use super::model::{EditPath, EditSubpath, HandleSide, Knot, KnotRef, PathEdit};

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn path(sub: EditSubpath) -> EditPath {
    EditPath { key: 7, subpaths: vec![sub], color: None }
}

fn square(smooth: bool) -> EditSubpath {
    let corners = [p(0.0, 0.0), p(100.0, 0.0), p(100.0, 100.0), p(0.0, 100.0)];
    EditSubpath { knots: corners.iter().map(|&c| Knot { smooth, ..Knot::corner(c) }).collect(), closed: true }
}

fn kr(knot: usize) -> KnotRef {
    KnotRef::new(0, knot)
}

fn near(a: Point, b: Point) -> bool {
    a.distance(b) < 1e-9
}

// A curve split at t = 0.3 draws the same curve (sampled); a straight segment gets a corner on the
// cubic's point at t; t of 0, 1 or NaN is refused and changes nothing.
#[test]
fn edit_subpath_insert_at_splits_a_curve_without_changing_it() {
    let mut sp = EditSubpath { knots: vec![Knot::smooth(p(0.0, 0.0), p(30.0, 60.0)), Knot::smooth(p(100.0, 0.0), p(130.0, -20.0))], closed: false };
    let before = sp.segment(0).unwrap();
    assert_eq!(sp.insert_at(0, 0.3), Some(1));
    assert_eq!(sp.knots.len(), 3);
    assert!(near(sp.knots[1].anchor, before.eval(0.3)));
    let (a, b) = (sp.segment(0).unwrap(), sp.segment(1).unwrap());
    for i in 0..=10 {
        let s = f64::from(i) / 10.0;
        assert!(near(a.eval(s), before.eval(0.3 * s)));
        assert!(near(b.eval(s), before.eval(0.3 + 0.7 * s)));
    }

    let mut line = EditSubpath { knots: vec![Knot::corner(p(0.0, 0.0)), Knot::corner(p(100.0, 0.0))], closed: false };
    let at = line.segment(0).unwrap().eval(0.3);
    assert_eq!(line.insert_at(0, 0.3), Some(1));
    assert!(near(line.knots[1].anchor, at));
    assert!(!line.knots[1].smooth);

    let snapshot = sp.clone();
    for t in [0.0, 1.0, f64::NAN] {
        assert_eq!(sp.insert_at(0, t), None);
    }
    assert_eq!(sp.insert_at(5, 0.5), None);
    assert_eq!(sp, snapshot);
}

// Removing an inner knot joins its neighbours with the previous out and next in handles; a closed
// subpath stays closed; removing the only knot leaves an empty subpath.
#[test]
fn edit_subpath_remove_knot_joins_neighbours_and_keeps_closed() {
    let mut sp = EditSubpath {
        knots: vec![Knot::smooth(p(0.0, 0.0), p(10.0, 10.0)), Knot::smooth(p(50.0, 0.0), p(60.0, 0.0)), Knot::smooth(p(100.0, 0.0), p(110.0, 5.0))],
        closed: true,
    };
    let (out, inn) = (sp.knots[0].out_ctrl, sp.knots[2].in_ctrl);
    assert!(sp.remove_knot(1));
    assert!(sp.closed);
    let seg = sp.segment(0).unwrap();
    assert_eq!((seg.p1, seg.p2), (out, inn));
    assert!(!sp.remove_knot(9));
    let mut one = EditSubpath { knots: vec![Knot::corner(p(1.0, 1.0))], closed: false };
    assert!(one.remove_knot(0));
    assert!(one.knots.is_empty());
}

// PhotoCraft's closed four-anchor loop: moving one anchor reflows every smooth knot (the
// neighbours turn), corner knots keep their handles.
#[test]
fn path_edit_move_curvature_reflows_every_smooth_knot() {
    let mut sp = square(true);
    sp.reflow();
    sp.knots[2] = Knot { smooth: false, ..Knot::corner(p(100.0, 100.0)) };
    let mut ep = path(sp);
    let corner = ep.subpaths[0].knots[2];
    let old_neighbour = ep.subpaths[0].knots[1];
    let edit = PathEdit::MoveCurvature { path: 7, anchors: vec![kr(0)], by: Vec2::new(-20.0, -10.0) };
    assert!(edit.apply(&mut ep));
    let k = &ep.subpaths[0].knots;
    assert_eq!(k[0].anchor, p(-20.0, -10.0));
    assert_ne!(k[1].out_ctrl, old_neighbour.out_ctrl);
    assert_eq!(k[2], corner);
    // Smooth knot handles are collinear with the chord of its neighbours.
    let chord = k[2].anchor - k[0].anchor;
    let o = k[1].out_ctrl - k[1].anchor;
    assert!((o.x * chord.y - o.y * chord.x).abs() < 1e-9);
}

// A corner square: toggling a knot makes it smooth and the segments around it curve; toggling
// again retracts its handles.
#[test]
fn path_edit_toggle_curvature_flips_smooth_and_corner() {
    let mut ep = path(square(false));
    let toggle = PathEdit::ToggleCurvature { path: 7, knot: kr(1) };
    assert!(toggle.apply(&mut ep));
    assert!(ep.subpaths[0].knots[1].smooth);
    assert!(!ep.subpaths[0].is_straight(0));
    assert!(!ep.subpaths[0].is_straight(1));
    assert!(toggle.apply(&mut ep));
    let k = ep.subpaths[0].knots[1];
    assert!(!k.smooth && k.in_ctrl == k.anchor && k.out_ctrl == k.anchor);
}

// `InsertCurvature` on a straight segment adds a smooth knot moved by `by` and reflows.
#[test]
fn path_edit_insert_curvature_adds_a_smooth_anchor_and_moves_it() {
    let mut ep = path(square(false));
    let at = ep.subpaths[0].segment(0).unwrap().eval(0.5);
    let edit = PathEdit::InsertCurvature { path: 7, segment: kr(0), t: 0.5, by: Vec2::new(0.0, -30.0) };
    assert!(edit.apply(&mut ep));
    let sp = &ep.subpaths[0];
    assert_eq!(sp.knots.len(), 5);
    assert!(sp.knots[1].smooth);
    assert!(near(sp.knots[1].anchor, at + Vec2::new(0.0, -30.0)));
    assert_ne!(sp.knots[1].out_ctrl, sp.knots[1].anchor);
}

// MoveAnchors, MoveHandle (smooth and alone), BendSegment (straight and curved), Convert give
// PhotoCraft's kernel results; a wrong key or index changes nothing and returns false.
#[test]
fn path_edit_apply_matches_the_kernel_for_every_existing_edit() {
    let base = || {
        path(EditSubpath {
            knots: vec![Knot::smooth(p(0.0, 0.0), p(20.0, 0.0)), Knot::smooth(p(100.0, 0.0), p(120.0, 0.0)), Knot::corner(p(100.0, 80.0))],
            closed: false,
        })
    };
    // MoveAnchors: duplicates move once, with handles.
    let mut ep = base();
    let by = Vec2::new(5.0, 7.0);
    assert!(PathEdit::MoveAnchors { path: 7, anchors: vec![kr(1), kr(1)], by }.apply(&mut ep));
    assert_eq!(ep.subpaths[0].knots[1].anchor, p(105.0, 7.0));
    assert_eq!(ep.subpaths[0].knots[1].out_ctrl, p(125.0, 7.0));

    // MoveHandle on a smooth knot turns the other handle, keeping its length.
    let mut ep = base();
    let to = p(0.0, 30.0);
    let h = PathEdit::MoveHandle { path: 7, knot: kr(0), side: HandleSide::Out, to, alone: false };
    assert!(h.apply(&mut ep));
    let k = ep.subpaths[0].knots[0];
    assert_eq!(k.out_ctrl, to);
    assert!(near(k.in_ctrl, p(0.0, -20.0)));
    // alone: the knot becomes a corner and the other handle stays.
    let mut ep = base();
    let h = PathEdit::MoveHandle { path: 7, knot: kr(0), side: HandleSide::Out, to, alone: true };
    assert!(h.apply(&mut ep));
    assert!(!ep.subpaths[0].knots[0].smooth);
    assert_eq!(ep.subpaths[0].knots[0].in_ctrl, p(-20.0, 0.0));

    // BendSegment on a curve moves the point at t exactly by `by`.
    let mut ep = base();
    let before = ep.subpaths[0].segment(0).unwrap().eval(0.5);
    let bend = PathEdit::BendSegment { path: 7, segment: kr(0), t: 0.5, by: Vec2::new(0.0, 10.0) };
    assert!(bend.apply(&mut ep));
    assert!(near(ep.subpaths[0].segment(0).unwrap().eval(0.5), before + Vec2::new(0.0, 10.0)));
    // On a straight segment both anchors move.
    let mut ep = base();
    let bend = PathEdit::BendSegment { path: 7, segment: kr(1), t: 0.5, by: Vec2::new(0.0, 10.0) };
    assert!(ep.subpaths[0].knots[1].out_ctrl != ep.subpaths[0].knots[1].anchor);
    ep.subpaths[0].knots[1].out_ctrl = ep.subpaths[0].knots[1].anchor;
    ep.subpaths[0].knots[1].in_ctrl = ep.subpaths[0].knots[1].anchor;
    assert!(bend.apply(&mut ep));
    assert_eq!(ep.subpaths[0].knots[1].anchor, p(100.0, 10.0));
    assert_eq!(ep.subpaths[0].knots[2].anchor, p(100.0, 90.0));

    // Convert.
    let mut ep = base();
    assert!(PathEdit::Convert { path: 7, knot: kr(0), out: None }.apply(&mut ep));
    assert_eq!(ep.subpaths[0].knots[0], Knot::corner(p(0.0, 0.0)));
    assert!(PathEdit::Convert { path: 7, knot: kr(2), out: Some(p(110.0, 80.0)) }.apply(&mut ep));
    assert_eq!(ep.subpaths[0].knots[2], Knot::smooth(p(100.0, 80.0), p(110.0, 80.0)));

    // Wrong key, knot or segment: unchanged, false.
    let mut ep = base();
    let untouched = base();
    let edits = [
        PathEdit::MoveAnchors { path: 8, anchors: vec![kr(0)], by },
        PathEdit::MoveAnchors { path: 7, anchors: vec![kr(0), kr(9)], by },
        PathEdit::MoveHandle { path: 7, knot: KnotRef::new(3, 0), side: HandleSide::In, to, alone: false },
        PathEdit::BendSegment { path: 7, segment: kr(2), t: 0.5, by },
        PathEdit::Convert { path: 7, knot: kr(5), out: None },
        PathEdit::ToggleCurvature { path: 7, knot: kr(5) },
        PathEdit::InsertCurvature { path: 7, segment: kr(0), t: 0.0, by },
    ];
    for e in edits {
        assert!(!e.apply(&mut ep));
        assert_eq!(ep, untouched);
    }
}
