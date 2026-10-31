//! PhotoCraft's box maths tests (`ui-egui/src/transform_tool.rs`), on the shared box.

use egui::Modifiers;
use kurbo::{Point, Rect, Vec2};

use super::{Anchor9, BoxAllow, BoxPart, BoxRules, BoxState, Frame, Homography, Parts, ScaleRule, TransformMode, carry_pivot, keep_convex, turns};

const ALT_SHIFT: Modifiers = Modifiers { alt: true, shift: true, ctrl: false, mac_cmd: false, command: false };
const PERSPECTIVE: Modifiers = Modifiers { alt: true, shift: true, ctrl: true, mac_cmd: false, command: true };

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn corners(x0: f64, y0: f64, x1: f64, y1: f64) -> [Point; 4] {
    BoxState::from_rect(Rect::new(x0, y0, x1, y1)).quad
}

/// PhotoCraft's test box: 100 × 50 at the origin, the reference point at its centre.
fn session() -> BoxState {
    BoxState::from_rect(Rect::new(0.0, 0.0, 100.0, 50.0))
}

fn rect() -> Rect {
    Rect::new(0.0, 0.0, 100.0, 50.0)
}

/// A drag in Free Transform from the part at `from` (8 units' reach).
fn drag(s: &mut BoxState, from: Point, to: Point, mods: Modifiers) {
    drag_with(s, from, to, mods, &BoxRules::default());
}

fn drag_with(s: &mut BoxState, from: Point, to: Point, mods: Modifiers, rules: &BoxRules) {
    *s = super::drag(s, s, s.part_at(from, 8.0), from, to, mods, rules);
}

fn drag_in(mode: TransformMode, from: Point, to: Point, mods: Modifiers) -> [Point; 4] {
    drag_on(mode, false, from, to, mods)
}

/// `parallel`: the content refuses distort (PhotoCraft's editable type).
fn drag_on(mode: TransformMode, parallel: bool, from: Point, to: Point, mods: Modifiers) -> [Point; 4] {
    let mut s = session();
    let rules = BoxRules { mode, allow: BoxAllow { distort: !parallel, ..BoxAllow::default() }, ..BoxRules::default() };
    drag_with(&mut s, from, to, mods, &rules);
    s.quad
}

fn eq6(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

fn near(a: Point, b: Point) -> bool {
    eq6(a.x, b.x) && eq6(a.y, b.y)
}

fn close(a: [Point; 4], b: [Point; 4]) -> bool {
    a.iter().zip(&b).all(|(a, b)| near(*a, *b))
}

/// PC `corner_drag_scales_proportionally_about_opposite_corner`.
#[test]
fn corner_drag_scales_proportionally_about_opposite_corner() {
    let mut s = session();
    drag(&mut s, p(100.0, 50.0), p(200.0, 60.0), Modifiers::NONE);
    assert!(close(s.quad, corners(0.0, 0.0, 200.0, 100.0)), "{:?}", s.quad);
    let mut s = session();
    drag(&mut s, p(100.0, 50.0), p(200.0, 60.0), Modifiers::SHIFT);
    assert!(close(s.quad, corners(0.0, 0.0, 200.0, 60.0)), "shift = free: {:?}", s.quad);
}

/// PC `edge_drag_scales_proportionally_about_the_opposite_edge`: an edge scales both axes about
/// the middle of the opposite edge; past it only the dragged axis flips; ⇧ stretches one axis.
#[test]
fn edge_drag_scales_proportionally_about_the_opposite_edge() {
    let mut s = session();
    drag(&mut s, p(100.0, 25.0), p(150.0, 25.0), Modifiers::NONE);
    assert!(close(s.quad, corners(0.0, -12.5, 150.0, 62.5)), "{:?}", s.quad);
    assert!(near(s.pivot, p(75.0, 25.0)), "{:?}", s.pivot);
    let mut s = session();
    drag(&mut s, p(50.0, 0.0), p(50.0, -50.0), Modifiers::NONE);
    assert!(close(s.quad, corners(-50.0, -50.0, 150.0, 50.0)), "{:?}", s.quad);
    let mut s = session();
    drag(&mut s, p(100.0, 25.0), p(-50.0, 25.0), Modifiers::NONE);
    assert!(close(s.quad, corners(0.0, 12.5, -50.0, 37.5)), "{:?}", s.quad);
    let mut s = session();
    drag(&mut s, p(50.0, 50.0), p(50.0, 80.0), Modifiers::SHIFT);
    assert!(close(s.quad, corners(0.0, 0.0, 100.0, 80.0)), "{:?}", s.quad);
}

/// PC `alt_scales_edges_about_reference_point`.
#[test]
fn alt_scales_edges_about_reference_point() {
    let mut s = session();
    drag(&mut s, p(100.0, 25.0), p(150.0, 25.0), Modifiers::ALT);
    assert!(close(s.quad, corners(-50.0, -25.0, 150.0, 75.0)), "{:?}", s.quad);
    let mut s = session();
    drag(&mut s, p(100.0, 25.0), p(150.0, 25.0), ALT_SHIFT);
    assert!(close(s.quad, corners(-50.0, 0.0, 150.0, 50.0)), "{:?}", s.quad);
}

/// PC `alt_scales_about_a_moved_reference_point`.
#[test]
fn alt_scales_about_a_moved_reference_point() {
    let mut s = BoxState { pivot: p(0.0, 0.0), ..session() };
    drag(&mut s, p(100.0, 50.0), p(200.0, 100.0), Modifiers::ALT);
    assert!(close(s.quad, corners(0.0, 0.0, 200.0, 100.0)), "{:?}", s.quad);
    assert!(near(s.pivot, p(0.0, 0.0)), "{:?}", s.pivot);
    let mut s = BoxState { pivot: p(25.0, 25.0), ..session() };
    drag(&mut s, p(100.0, 25.0), p(175.0, 25.0), Modifiers::ALT);
    assert!(close(s.quad, corners(-25.0, -25.0, 175.0, 75.0)), "{:?}", s.quad);
    let mut s = BoxState { pivot: p(25.0, 25.0), ..session() };
    drag(&mut s, p(100.0, 25.0), p(175.0, 25.0), ALT_SHIFT);
    assert!(close(s.quad, corners(-25.0, 0.0, 175.0, 50.0)), "{:?}", s.quad);
    // A reference point in line with the dragged corner across: that axis stays.
    let mut s = BoxState { pivot: p(100.0, 50.0), ..session() };
    drag(&mut s, p(100.0, 0.0), p(150.0, -50.0), ALT_SHIFT);
    assert!(close(s.quad, corners(0.0, -50.0, 100.0, 50.0)), "{:?}", s.quad);
}

/// PC `the_reference_point_moves_with_the_box`: scaling, skewing, distorting and perspective carry
/// it to the same place within the box; ⌥-scaling and rotating leave it put.
#[test]
fn the_reference_point_moves_with_the_box() {
    let mut s = session();
    drag(&mut s, p(100.0, 50.0), p(50.0, 25.0), Modifiers::NONE);
    assert!(close(s.quad, corners(0.0, 0.0, 50.0, 25.0)), "{:?}", s.quad);
    assert!(near(s.pivot, p(25.0, 12.5)), "{:?}", s.pivot);
    let mut s = BoxState { pivot: p(100.0, 0.0), ..session() };
    drag(&mut s, p(50.0, 50.0), p(50.0, 80.0), Modifiers::SHIFT);
    assert!(near(s.pivot, p(100.0, 0.0)), "{:?}", s.pivot);
    drag(&mut s, p(100.0, 40.0), p(150.0, 40.0), Modifiers::SHIFT);
    assert!(near(s.pivot, s.quad[1]), "{:?} {:?}", s.pivot, s.quad);
    drag(&mut s, p(150.0, 40.0), p(180.0, 40.0), Modifiers::NONE);
    assert!(near(s.pivot, s.quad[1]), "proportional edge drag: {:?} {:?}", s.pivot, s.quad);
    let mut s = session();
    drag(&mut s, p(50.0, 0.0), p(70.0, 0.0), Modifiers::COMMAND);
    assert!(near(s.pivot, p(60.0, 25.0)), "skew: {:?}", s.pivot);
    let mut s = BoxState { pivot: p(0.0, 50.0), ..session() };
    drag(&mut s, p(0.0, 50.0), p(-20.0, 70.0), Modifiers::COMMAND);
    assert!(near(s.pivot, p(-20.0, 70.0)), "distort: {:?}", s.pivot);
    let mut s = session();
    drag(&mut s, p(100.0, 0.0), p(120.0, 0.0), PERSPECTIVE);
    let centre = Homography::unit_to_quad(s.quad).unwrap().apply(p(0.5, 0.5));
    assert!(near(s.pivot, centre), "perspective: {:?} vs {centre:?}", s.pivot);
    let mut s = session();
    drag(&mut s, p(100.0, 50.0), p(150.0, 75.0), Modifiers::ALT);
    assert!(near(s.pivot, p(50.0, 25.0)), "{:?}", s.pivot);
    let mut s = session();
    drag(&mut s, p(150.0, 25.0), p(50.0, 125.0), Modifiers::NONE);
    assert!(near(s.pivot, p(50.0, 25.0)), "{:?}", s.pivot);
    // From a box collapsed to a point there is no place within it to keep.
    assert_eq!(carry_pivot([p(0.0, 0.0); 4], corners(0.0, 0.0, 10.0, 10.0), p(5.0, 5.0)), p(5.0, 5.0));
}

/// PC `move_rotate_and_distort`, with the perspective pair and the ⇧ move lock.
#[test]
fn move_rotate_distort_and_perspective() {
    let mut s = session();
    drag(&mut s, p(40.0, 20.0), p(50.0, 30.0), Modifiers::NONE);
    assert!(close(s.quad, corners(10.0, 10.0, 110.0, 60.0)));
    assert_eq!(s.pivot, p(60.0, 35.0));
    // A 90° turn about the centre, from right of the box to below it.
    let mut s = session();
    drag(&mut s, p(150.0, 25.0), p(50.0, 125.0), Modifiers::NONE);
    assert!(eq6(s.parts(rect(), 0.0).angle, 90.0), "{:?}", s.parts(rect(), 0.0));
    assert!(eq6(s.angle(), 90.0), "{}", s.angle());
    // ⇧ snaps the angle to the rule's step: 50° lands on 45°; 60° stays at 15° steps and lands
    // on 45° at 45° steps.
    let turned = |deg: f64, mods: Modifiers, rotate_step: f64| {
        let a = deg.to_radians();
        let mut s = session();
        let rules = BoxRules { rotate_step, ..BoxRules::default() };
        drag_with(&mut s, p(150.0, 25.0), p(50.0 + 100.0 * a.cos(), 25.0 + 100.0 * a.sin()), mods, &rules);
        s.parts(rect(), 0.0).angle
    };
    assert!(eq6(turned(50.0, Modifiers::SHIFT, 15.0), 45.0));
    assert!(eq6(turned(50.0, Modifiers::NONE, 15.0), 50.0));
    assert!(eq6(turned(60.0, Modifiers::SHIFT, 15.0), 60.0));
    assert!(eq6(turned(60.0, Modifiers::SHIFT, 45.0), 45.0));
    // ⌘ corner: distort.
    let mut s = session();
    drag(&mut s, p(100.0, 0.0), p(120.0, -10.0), Modifiers::COMMAND);
    assert_eq!(s.quad[1], p(120.0, -10.0));
    assert_eq!(s.quad[0], p(0.0, 0.0));
    // ⌘⌥⇧ corner: perspective; the top-left corner moves the other way along the top edge.
    let mut s = session();
    drag(&mut s, p(100.0, 0.0), p(120.0, 3.0), PERSPECTIVE);
    assert!(close(s.quad, [p(-20.0, 0.0), p(120.0, 0.0), p(100.0, 50.0), p(0.0, 50.0)]), "{:?}", s.quad);
    // A mostly vertical drag pairs the corner with the one below it.
    let mut s = session();
    drag(&mut s, p(100.0, 0.0), p(101.0, -10.0), PERSPECTIVE);
    assert!(close(s.quad, [p(0.0, 0.0), p(100.0, -10.0), p(100.0, 60.0), p(0.0, 50.0)]), "{:?}", s.quad);
    // ⇧ inside locks the move to the axes and diagonals, and keeps the lock past the half-way line.
    let start = session();
    let from = p(40.0, 20.0);
    let rules = BoxRules::default();
    let first = super::drag(&start, &start, BoxPart::Inside, from, from + Vec2::new(30.0, 4.0), Modifiers::SHIFT, &rules);
    assert_eq!(first.quad[0], p(30.0, 0.0));
    let past = from + Vec2::from_angle(25f64.to_radians()) * 30.0;
    let kept = super::drag(&start, &first, BoxPart::Inside, from, past, Modifiers::SHIFT, &rules);
    assert!(eq6(kept.quad[0].y, 0.0), "{:?}", kept.quad);
    let fresh = super::drag(&start, &start, BoxPart::Inside, from, past, Modifiers::SHIFT, &rules);
    assert!(eq6(fresh.quad[0].x, fresh.quad[0].y), "{:?}", fresh.quad);
}

/// PC `skew_mode_on_type_slides_the_corners_edge`: content that stays a parallelogram slides a
/// corner's whole edge.
#[test]
fn skew_mode_on_type_slides_the_corners_edge() {
    let (skew, none) = (TransformMode::Skew, Modifiers::NONE);
    let q = |a: [(f64, f64); 4]| a.map(|(x, y)| p(x, y));
    assert_eq!(drag_on(skew, true, p(100.0, 0.0), p(110.0, 3.0), none), q([(10.0, 0.0), (110.0, 0.0), (100.0, 50.0), (0.0, 50.0)]));
    assert_eq!(drag_on(skew, true, p(100.0, 0.0), p(102.0, -9.0), none), q([(0.0, 0.0), (100.0, -9.0), (100.0, 41.0), (0.0, 50.0)]));
    assert_eq!(drag_on(skew, true, p(100.0, 0.0), p(110.0, 3.0), Modifiers::ALT), q([(10.0, 0.0), (110.0, 0.0), (90.0, 50.0), (-10.0, 50.0)]));
    assert_eq!(drag_on(skew, true, p(50.0, 0.0), p(62.0, 7.0), none), q([(12.0, 0.0), (112.0, 0.0), (100.0, 50.0), (0.0, 50.0)]));
}

/// PC `skew_mode_moves_a_corner_or_an_edge_along_one_axis`.
#[test]
fn skew_mode_moves_a_corner_or_an_edge_along_one_axis() {
    let (skew, none) = (TransformMode::Skew, Modifiers::NONE);
    let q = |a: [(f64, f64); 4]| a.map(|(x, y)| p(x, y));
    assert_eq!(drag_in(skew, p(100.0, 0.0), p(110.0, 3.0), none), q([(0.0, 0.0), (110.0, 0.0), (100.0, 50.0), (0.0, 50.0)]));
    assert_eq!(drag_in(skew, p(100.0, 0.0), p(102.0, -9.0), none), q([(0.0, 0.0), (100.0, -9.0), (100.0, 50.0), (0.0, 50.0)]));
    assert_eq!(drag_in(skew, p(50.0, 0.0), p(62.0, 7.0), none), q([(12.0, 0.0), (112.0, 0.0), (100.0, 50.0), (0.0, 50.0)]));
    assert_eq!(drag_in(skew, p(0.0, 25.0), p(6.0, 33.0), none), q([(0.0, 8.0), (100.0, 0.0), (100.0, 50.0), (0.0, 58.0)]));
    let alt = Modifiers::ALT;
    assert_eq!(drag_in(skew, p(100.0, 0.0), p(110.0, 3.0), alt), q([(0.0, 0.0), (110.0, 0.0), (100.0, 50.0), (-10.0, 50.0)]));
    assert_eq!(drag_in(skew, p(50.0, 0.0), p(62.0, 7.0), alt), q([(12.0, 0.0), (112.0, 0.0), (88.0, 50.0), (-12.0, 50.0)]));
    let shift = Modifiers::SHIFT;
    assert_eq!(drag_in(skew, p(100.0, 0.0), p(110.0, 3.0), shift), drag_in(skew, p(100.0, 0.0), p(110.0, 3.0), none));
    assert_eq!(drag_in(skew, p(50.0, 0.0), p(62.0, 7.0), shift), drag_in(skew, p(50.0, 0.0), p(62.0, 7.0), none));
    // No rotating from outside in Skew.
    assert_eq!(drag_in(skew, p(130.0, 25.0), p(120.0, 60.0), none), corners(0.0, 0.0, 100.0, 50.0));
    assert_ne!(drag_in(TransformMode::Free, p(100.0, 0.0), p(110.0, 3.0), none)[2], p(100.0, 50.0));
}

/// PC `distort_mode_moves_one_corner_and_ignores_rotate_and_edges` and
/// `rotate_mode_turns_the_box_from_a_corner_handle`, on PhotoCraft's 16 × 16 square.
#[test]
fn distort_and_rotate_modes() {
    let square = BoxState::from_rect(Rect::new(8.0, 8.0, 24.0, 24.0));
    let distort = BoxRules { mode: TransformMode::Distort, ..BoxRules::default() };
    let q0 = square.quad;
    let mut s = square;
    drag_with(&mut s, p(44.0, 44.0), p(54.0, 10.0), Modifiers::NONE, &distort);
    drag_with(&mut s, p(24.0, 16.0), p(34.0, 16.0), Modifiers::NONE, &distort);
    assert_eq!(s.quad, q0, "the outside and the edges do nothing");
    assert!(!distort.allows(BoxPart::Outside) && !distort.allows(BoxPart::Edge(1)) && distort.allows(BoxPart::Inside));
    drag_with(&mut s, q0[1], q0[1] + Vec2::new(7.0, -5.0), Modifiers::NONE, &distort);
    assert_eq!(s.quad[1], q0[1] + Vec2::new(7.0, -5.0));
    assert_eq!([s.quad[0], s.quad[2], s.quad[3]], [q0[0], q0[2], q0[3]]);

    let rotate = BoxRules { mode: TransformMode::Rotate, ..BoxRules::default() };
    let mut s = square;
    drag_with(&mut s, q0[1], q0[1] + Vec2::new(0.0, 16.0), Modifiers::NONE, &rotate);
    assert!(eq6((s.quad[1] - s.quad[0]).hypot(), 16.0), "the side stays: {:?}", s.quad);
    assert!((s.quad[0].y - s.quad[1].y).abs() > 1.0, "turned, not scaled: {:?}", s.quad);
}

/// PC `a_distort_corner_stops_before_the_box_folds_over` and
/// `keep_convex_leaves_a_degenerate_start_alone`.
#[test]
fn distort_stops_where_the_box_would_turn_concave() {
    let square = BoxState::from_rect(Rect::new(8.0, 8.0, 24.0, 24.0));
    let distort = BoxRules { mode: TransformMode::Distort, ..BoxRules::default() };
    let q0 = square.quad;
    let mut s = square;
    drag_with(&mut s, q0[0], p(60.0, 60.0), Modifiers::NONE, &distort);
    let sign = turns(&q0)[0].signum();
    assert!(turns(&s.quad).iter().all(|t| t * sign > 0.0), "still convex: {:?}", s.quad);
    assert_eq!([s.quad[1], s.quad[2], s.quad[3]], [q0[1], q0[2], q0[3]]);
    // As far as it could go: up to the other diagonal (x + y = 32).
    assert!(s.quad[0].x > 15.0 && s.quad[0].x < 16.0 && eq6(s.quad[0].x, s.quad[0].y), "{:?}", s.quad);
    let at = s.quad[0];
    drag_with(&mut s, at, p(2.0, 4.0), Modifiers::NONE, &distort);
    assert_eq!(s.quad[0], p(2.0, 4.0), "a drag that keeps the box convex is not held back");

    let flat = [p(0.0, 0.0), p(1.0, 0.0), p(2.0, 0.0), p(3.0, 0.0)];
    let to = [p(5.0, 5.0), p(1.0, 0.0), p(2.0, 0.0), p(3.0, 0.0)];
    assert_eq!(keep_convex(flat, to), to);
    let _ = keep_convex([p(f64::NAN, 0.0), p(1.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)], to);
}

/// PC `scale_not_distort`: content that refuses distort scales on ⌘ corner drags, and in a
/// Distort mode it can't take.
#[test]
fn refused_distort_scales_instead() {
    let parallel = BoxAllow { distort: false, ..BoxAllow::default() };
    for mode in [TransformMode::Free, TransformMode::Distort, TransformMode::Perspective] {
        let rules = BoxRules { mode, allow: parallel, ..BoxRules::default() };
        let mut s = session();
        drag_with(&mut s, p(100.0, 50.0), p(200.0, 60.0), Modifiers::COMMAND, &rules);
        assert!(close(s.quad, corners(0.0, 0.0, 200.0, 100.0)), "{mode:?}: {:?}", s.quad);
    }
    // Only scaling and moving: no rotating, no skewing.
    let scale_only = BoxRules { allow: BoxAllow { rotate: false, skew: false, distort: false }, ..BoxRules::default() };
    assert!(!scale_only.allows(BoxPart::Outside));
    let mut s = session();
    drag_with(&mut s, p(50.0, 0.0), p(70.0, 0.0), Modifiers::COMMAND, &scale_only);
    assert!(s.is_parallelogram() && eq6(s.quad[0].x, s.quad[3].x), "no skew: {:?}", s.quad);
}

/// PC `the_nearest_handle_wins_on_a_small_box`.
#[test]
fn the_nearest_handle_wins_on_a_small_box() {
    let s = BoxState::from_rect(Rect::new(0.0, 0.0, 16.0, 16.0));
    assert_eq!(s.part_at(p(0.0, 0.0), 12.0), BoxPart::Corner(0));
    assert_eq!(s.part_at(p(16.0, 17.0), 12.0), BoxPart::Corner(2));
    assert_eq!(s.part_at(p(8.0, -1.0), 12.0), BoxPart::Edge(0));
    assert_eq!(s.part_at(p(8.0, 8.0), 12.0), BoxPart::Pivot);
    assert_eq!(s.part_at(p(40.0, 40.0), 12.0), BoxPart::Outside);
    assert_eq!(session().part_at(p(30.0, 30.0), 12.0), BoxPart::Inside);
}

/// PC `the_legacy_preference_swaps_corner_proportions_and_stretches_edges` and
/// `the_aspect_link_decides_whether_handle_drags_keep_proportions`, through the box: a plain
/// corner drag and a plain right-edge drag of 16 on a 16 × 16 square.
#[test]
fn the_scale_rule_and_the_aspect_link_decide_proportions() {
    let (none, shift) = (Modifiers::NONE, Modifiers::SHIFT);
    for (scale, linked, mods, proportional) in [
        (ScaleRule::Proportional, true, none, true),
        (ScaleRule::Proportional, true, shift, false),
        (ScaleRule::Free, true, none, false),
        (ScaleRule::Free, true, shift, true),
        (ScaleRule::Proportional, false, none, false),
        (ScaleRule::Proportional, false, shift, true),
        (ScaleRule::Free, false, none, false),
    ] {
        for at_edge in [false, true] {
            let rules = BoxRules { scale, aspect_linked: linked, ..BoxRules::default() };
            let mut s = BoxState::from_rect(Rect::new(8.0, 8.0, 24.0, 24.0));
            let q0 = s.quad;
            let from = if at_edge { q0[1].midpoint(q0[2]) } else { q0[2] };
            drag_with(&mut s, from, from + Vec2::new(16.0, 0.0), mods, &rules);
            let size = s.quad[2] - s.quad[0];
            let case = format!("{scale:?}, linked {linked}, shift {}, edge {at_edge}: {size:?}", mods.shift);
            if scale == ScaleRule::Free && at_edge {
                // Legacy edges always stretch their one axis.
                assert!(eq6(size.x, 32.0) && eq6(size.y, 16.0), "{case}");
            } else {
                assert_eq!(eq6(size.x, size.y), proportional, "{case}");
                assert!(eq6(size.x, 32.0), "{case}");
            }
        }
    }
    // ⌘ gestures are left alone: a legacy ⌘ corner drag distorts.
    let legacy = BoxRules { scale: ScaleRule::Free, aspect_linked: false, ..BoxRules::default() };
    let mut s = session();
    drag_with(&mut s, p(100.0, 0.0), p(120.0, -10.0), Modifiers::COMMAND, &legacy);
    assert_eq!(s.quad[1], p(120.0, -10.0));
}

/// PC `parts_round_trip`: scale, skew and rotation read back exactly from the box they make,
/// flips included.
#[test]
fn parts_round_trip() {
    for (sx, sy, angle, h, v) in
        [(1.0, 1.0, 0.0, 0.0, 0.0), (1.5, 0.5, 30.0, 20.0, 0.0), (0.8, -1.2, -120.0, -35.0, 15.0), (2.0, 1.0, 179.0, 0.0, -60.0)]
    {
        let f = Frame { sx, sy, angle: f64::to_radians(angle), skew_h: f64::to_radians(h), skew_v: f64::to_radians(v) };
        let back = Frame::from_columns(f.columns(), f.skew_v);
        for (a, b) in [(back.sx, f.sx), (back.sy, f.sy), (back.angle, f.angle), (back.skew_h, f.skew_h)] {
            assert!(eq6(a, b), "{f:?} read back as {back:?}");
        }
    }
}

/// PC `a_sheared_box_reads_as_horizontal_skew`.
#[test]
fn a_sheared_box_reads_as_horizontal_skew() {
    let d = 50.0 * 20f64.to_radians().tan();
    let s = BoxState { quad: [p(0.0, 0.0), p(100.0, 0.0), p(100.0 - d, 50.0), p(-d, 50.0)], ..session() };
    let r = s.parts(rect(), 0.0);
    assert!(eq6(r.w, 1.0) && eq6(r.h, 1.0) && eq6(r.angle, 0.0) && eq6(r.skew_h, 20.0), "{r:?}");
}

const FLIPPED_H: [(f64, f64); 4] = [(100.0, 0.0), (0.0, 0.0), (0.0, 50.0), (100.0, 50.0)];
const FLIPPED_V: [(f64, f64); 4] = [(0.0, 50.0), (100.0, 50.0), (100.0, 0.0), (0.0, 0.0)];

fn boxed(q: [(f64, f64); 4]) -> BoxState {
    BoxState { quad: q.map(|(x, y)| p(x, y)), ..session() }
}

/// PC `a_flipped_box_reads_as_a_negative_scale_on_that_axis`: Flip Horizontal reads W −100 %,
/// Flip Vertical H −100 %, both at 0°; flipped both ways is a half turn.
#[test]
fn a_flipped_box_reads_as_a_negative_scale_on_that_axis() {
    for (q, want) in [(FLIPPED_H, (-1.0, 1.0)), (FLIPPED_V, (1.0, -1.0))] {
        let r = boxed(q).parts(rect(), 0.0);
        assert!(eq6(r.w, want.0) && eq6(r.h, want.1) && eq6(r.angle, 0.0) && eq6(r.skew_h, 0.0), "{q:?}: {r:?}");
    }
    let r = boxed([(100.0, 50.0), (0.0, 50.0), (0.0, 0.0), (100.0, 0.0)]).parts(rect(), 0.0);
    assert!(eq6(r.w, 1.0) && eq6(r.h, 1.0) && eq6(r.angle.abs(), 180.0), "{r:?}");
}

/// PC `bar_fields_keep_a_flipped_box_flipped`: angle 0 keeps a flipped box as it is, and a new W
/// scales it and keeps it mirrored.
#[test]
fn bar_fields_keep_a_flipped_box_flipped() {
    let flipped = boxed(FLIPPED_H);
    let now = flipped.parts(rect(), 0.0);
    let same = flipped.with_parts(rect(), 0.0, Parts { angle: 0.0, ..now });
    assert!(close(same.quad, flipped.quad), "{:?}", same.quad);
    let half = flipped.with_parts(rect(), 0.0, Parts { w: -0.5, ..now });
    let r = half.parts(rect(), 0.0);
    assert!(eq6(r.w, -0.5) && eq6(r.h, 1.0) && eq6(r.angle, 0.0), "{r:?}");
    assert!(eq6(half.quad[0].x - half.quad[1].x, 50.0), "50 wide, still mirrored: {:?}", half.quad);
    let flipped = boxed(FLIPPED_V);
    let now = flipped.parts(rect(), 0.0);
    let r = flipped.with_parts(rect(), 0.0, Parts { h: -2.0, ..now }).parts(rect(), 0.0);
    assert!(eq6(r.w, 1.0) && eq6(r.h, -2.0), "{r:?}");
}

/// PC `skew_fields_lean_the_box_about_the_reference_point`: H and V lean the box about the
/// reference point, W, H and the angle keep their values, and a skew stops at 89°.
#[test]
fn skew_fields_lean_the_box_about_the_reference_point() {
    let s = session();
    let now = s.parts(rect(), 0.0);
    let leaned = s.with_parts(rect(), 0.0, Parts { skew_h: 20.0, ..now });
    let r = leaned.parts(rect(), 0.0);
    assert!(eq6(r.skew_h, 20.0) && eq6(r.angle, 0.0) && eq6(r.w, 1.0) && eq6(r.h, 1.0), "{r:?}");
    let d = 25.0 * 20f64.to_radians().tan();
    assert!(close(leaned.quad, [p(d, 0.0), p(100.0 + d, 0.0), p(100.0 - d, 50.0), p(-d, 50.0)]), "{:?}", leaned.quad);
    assert_eq!(leaned.pivot, s.pivot);

    let leaned = s.with_parts(rect(), 0.0, Parts { skew_v: -10.0, ..now });
    let r = leaned.parts(rect(), -10.0);
    assert!(eq6(r.skew_h, 0.0) && eq6(r.angle, 0.0) && eq6(r.w, 1.0) && eq6(r.h, 1.0), "{r:?}");
    let d = 50.0 * (-10f64).to_radians().tan();
    assert!(eq6(leaned.quad[1].y, d) && eq6(leaned.quad[0].y, -d), "the right side rose: {:?}", leaned.quad);

    let r = s.with_parts(rect(), 0.0, Parts { skew_h: 120.0, ..now }).parts(rect(), 0.0);
    assert!(eq6(r.skew_h, super::SKEW_LIMIT), "{r:?}");
}

/// The nine points, the reference point found on one of them, the affine of a parallelogram and
/// none for a distorted box.
#[test]
fn points_and_the_affine_of_a_box() {
    let s = session();
    assert_eq!(s.point_at(Anchor9::TopRight), p(100.0, 0.0));
    assert_eq!(s.point_at(Anchor9::Bottom), p(50.0, 50.0));
    assert_eq!(s.anchor_of_pivot(), Some(Anchor9::Center));
    assert_eq!(BoxState { pivot: p(100.0, 25.0), ..s }.anchor_of_pivot(), Some(Anchor9::Right));
    assert_eq!(BoxState { pivot: p(30.0, 10.0), ..s }.anchor_of_pivot(), None);
    assert_eq!(s.moved(Vec2::new(5.0, -5.0)).pivot, p(55.0, 20.0));

    let mut turned = session();
    drag(&mut turned, p(150.0, 25.0), p(50.0, 125.0), Modifiers::NONE);
    let a = turned.affine_from(rect()).unwrap();
    for (k, corner) in BoxState::from_rect(rect()).quad.iter().enumerate() {
        assert!(near(a * *corner, turned.quad[k]), "{k}: {:?} vs {:?}", a * *corner, turned.quad[k]);
    }
    let mut distorted = session();
    drag(&mut distorted, p(100.0, 0.0), p(120.0, -10.0), Modifiers::COMMAND);
    assert!(!distorted.is_parallelogram());
    assert_eq!(distorted.affine_from(rect()), None);
}

/// The public modifiers and kind are what a drag acts on: dragging with the keys held under the
/// rules equals dragging with `drag_modifiers` under rules that add nothing, and the kind names
/// what happened.
#[test]
fn the_effective_modifiers_and_kind_are_what_the_drag_acts_on() {
    use super::BoxDrag;
    let none = Modifiers::NONE;
    let cmd = Modifiers::COMMAND;
    let cases = [
        // (rules, part, held keys, kind)
        (BoxRules::default(), BoxPart::Corner(2), none, BoxDrag::Scale),
        (BoxRules { scale: ScaleRule::Free, ..BoxRules::default() }, BoxPart::Corner(2), none, BoxDrag::Scale),
        (BoxRules { scale: ScaleRule::Free, ..BoxRules::default() }, BoxPart::Edge(1), none, BoxDrag::Scale),
        (BoxRules { aspect_linked: false, ..BoxRules::default() }, BoxPart::Corner(2), Modifiers::SHIFT, BoxDrag::Scale),
        (BoxRules::default(), BoxPart::Corner(2), cmd, BoxDrag::Distort),
        (BoxRules::default(), BoxPart::Edge(1), cmd, BoxDrag::Skew),
        (BoxRules { mode: TransformMode::Distort, ..BoxRules::default() }, BoxPart::Corner(2), none, BoxDrag::Distort),
        (BoxRules { mode: TransformMode::Perspective, ..BoxRules::default() }, BoxPart::Corner(2), none, BoxDrag::Perspective),
        (BoxRules { mode: TransformMode::Rotate, ..BoxRules::default() }, BoxPart::Corner(2), none, BoxDrag::Rotate),
        (BoxRules { mode: TransformMode::Skew, ..BoxRules::default() }, BoxPart::Corner(2), none, BoxDrag::SlideCorner),
        (BoxRules { mode: TransformMode::Skew, ..BoxRules::default() }, BoxPart::Edge(1), none, BoxDrag::Skew),
        // The content refuses distorting: ⌘ on a corner scales.
        (BoxRules { allow: BoxAllow { distort: false, ..BoxAllow::default() }, ..BoxRules::default() }, BoxPart::Corner(2), cmd, BoxDrag::Scale),
    ];
    let (from, to) = (p(100.0, 50.0), p(130.0, 65.0));
    let edge_from = p(100.0, 25.0);
    for (rules, part, held, kind) in cases {
        let from = if matches!(part, BoxPart::Edge(_)) { edge_from } else { from };
        let to = if matches!(part, BoxPart::Edge(_)) { p(130.0, 25.0) } else { to };
        assert_eq!(rules.drag_kind(part, held), Some(kind), "{rules:?} {part:?} {held:?}");
        let s = session();
        let held_result = super::drag(&s, &s, part, from, to, held, &rules);
        let plain = BoxRules { mode: rules.effective_mode(), allow: rules.allow, ..BoxRules::default() };
        let effective = super::drag(&s, &s, part, from, to, rules.drag_modifiers(part, held), &plain);
        assert_eq!(held_result, effective, "{rules:?} {part:?} {held:?}");
    }
    let refused = BoxRules { mode: TransformMode::Distort, ..BoxRules::default() };
    assert_eq!(refused.drag_kind(BoxPart::Edge(1), none), None, "Distort leaves the edges alone");
    assert!(BoxRules { scale: ScaleRule::Free, ..BoxRules::default() }.drag_modifiers(BoxPart::Corner(2), none).shift);
}

/// PC `w_and_h_reject_or_clamp_hostile_typed_lengths`: a negative length typed with W and H linked
/// mirrors that side (W −40 px of 100 px) and does not turn the box a half turn.
#[test]
fn a_negative_linked_length_mirrors_the_typed_side() {
    let s = session();
    let (w, h) = BoxState::linked_scale((1.0, 1.0), 0, -0.4);
    assert!(eq6(w, -0.4) && eq6(h, 0.4), "{w} {h}");
    let flipped = s.with_signed_scale(rect(), 0.0, w, h);
    assert!(eq6(flipped.quad[1].x - flipped.quad[0].x, -40.0) && eq6(flipped.quad[3].y - flipped.quad[0].y, 20.0), "{:?}", flipped.quad);
    let (sw, sh) = flipped.signed_scale(rect(), 0.0);
    assert!(eq6(sw, -0.4) && eq6(sh, 0.4) && eq6(flipped.parts(rect(), 0.0).angle, 0.0));
    // Typing H −20 px reaches the same box through the other side.
    let (w, h) = BoxState::linked_scale((1.0, 1.0), 1, -0.4);
    assert!(eq6(w, 0.4) && eq6(h, -0.4));
    // Both sides negative is a half turn, which the box reads as positive sides at 180°.
    let turned = s.with_signed_scale(rect(), 0.0, -0.4, -0.4);
    let parts = turned.parts(rect(), 0.0);
    assert!(eq6(parts.w, 0.4) && eq6(parts.h, 0.4) && eq6(parts.angle.abs(), 180.0), "{parts:?}");
    // A zero side collapses without dividing by zero.
    let (w, h) = BoxState::linked_scale((1.0, 1.0), 0, 0.0);
    assert!(w == 0.0 && h == 0.0);
    assert!(s.with_signed_scale(rect(), 0.0, w, h).quad.iter().all(|q| q.is_finite()));
}

/// PC `w_and_h_edits_on_a_distorted_box_match_upstream`: W and H typed (linked) on a perspective
/// box scale it through its homography and keep the perspective.
#[test]
fn w_and_h_edits_on_a_distorted_box_keep_its_perspective() {
    let r = Rect::new(0.0, 0.0, 100.0, 50.0);
    let s = BoxState { quad: [p(0.0, 0.0), p(100.0, 10.0), p(90.0, 60.0), p(-5.0, 50.0)], pivot: p(50.0, 25.0) };
    let now = s.signed_scale(r, 0.0);
    let edit = |axis: usize, typed: f64| {
        let (w, h) = BoxState::linked_scale(now, axis, typed);
        s.with_signed_scale(r, 0.0, w, h).quad
    };
    let want_w = [p(25.300, 12.650), p(74.570, 17.629), p(70.164, 42.643), p(22.139, 37.664)];
    let got = edit(0, 0.5);
    assert!(got.iter().zip(&want_w).all(|(g, w)| (g.x - w.x).abs() < 1e-3 && (g.y - w.y).abs() < 1e-3), "{got:?}");
    let got = edit(1, 1.5);
    assert!((got[0].x + 25.153).abs() < 1e-3 && (got[0].y + 12.577).abs() < 1e-3, "{got:?}");
}
