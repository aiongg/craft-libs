//! Snapping to targets within a tolerance.

use craft_behavior::geometry::{SnapKind, SnapTarget, SnapTargets};
use kurbo::{Point, Rect, Vec2};

fn guide(vertical: bool, position: f64) -> SnapTarget {
    SnapTarget::new(vertical, position, SnapKind::Guide)
}

#[test]
fn a_point_snaps_within_the_tolerance_and_not_outside_it() {
    let mut targets = SnapTargets::new();
    targets.add(guide(true, 120.0));
    let (snapped, lines) = targets.snap_point(Point::new(123.0, 50.0), 4.0);
    assert_eq!(snapped, Point::new(120.0, 50.0));
    assert_eq!(lines.len(), 1);
    let (unsnapped, lines) = targets.snap_point(Point::new(125.5, 50.0), 5.0);
    assert_eq!(unsnapped, Point::new(125.5, 50.0));
    assert!(lines.is_empty());
}

#[test]
fn the_nearest_target_wins() {
    let mut targets = SnapTargets::new();
    targets.add(guide(true, 100.0));
    targets.add(guide(true, 103.0));
    let (snapped, _) = targets.snap_point(Point::new(102.0, 0.0), 4.0);
    assert_eq!(snapped.x, 103.0);
}

#[test]
fn a_moving_rect_snaps_its_nearest_edge_or_centre() {
    let mut targets = SnapTargets::new();
    targets.add(guide(true, 100.0));
    targets.add(guide(false, 50.0));
    let (offset, _) = targets.snap_rect(Rect::new(76.0, 53.0, 120.0, 90.0), 4.0);
    assert_eq!(offset, Vec2::new(2.0, -3.0));
    let (offset, lines) = targets.snap_rect(Rect::new(10.0, 10.0, 20.0, 20.0), 4.0);
    assert_eq!(offset, Vec2::ZERO);
    assert!(lines.is_empty());
}

#[test]
fn filtered_kinds_are_skipped() {
    let all = SnapTargets::new().with_document(&Default::default(), Rect::new(0.0, 0.0, 10.0, 10.0), [(true, 4.0)], None);
    let (_, line) = all.snap_axis(true, 4.2, 0.5);
    assert_eq!(line.map(|l| (l.kind, l.span)), Some((SnapKind::Guide, None)));
    let (value, line) = all.filtered(|kind| kind != SnapKind::Guide).snap_axis(true, 4.2, 0.5);
    assert_eq!((value, line), (4.2, None));
}
