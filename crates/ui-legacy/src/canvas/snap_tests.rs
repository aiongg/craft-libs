//! Snapping.

use kurbo::{Point, Rect, Vec2};

use super::{SnapKind, SnapOptions, SnapTarget, SnapTargets};

fn guide(vertical: bool, position: f64) -> SnapTarget {
    SnapTarget { vertical, position, kind: SnapKind::Guide, span: None }
}

// A point 3 units from a guide at tolerance 4 snaps onto it; at 5 it does not.
#[test]
fn a_point_snaps_to_a_target_within_the_tolerance() {
    let mut targets = SnapTargets::new();
    targets.add(guide(true, 120.0));
    let (snapped, lines) = targets.snap_point(Point::new(123.0, 50.0), 4.0);
    assert_eq!(snapped, Point::new(120.0, 50.0));
    assert_eq!(lines.len(), 1);
    let (unsnapped, lines) = targets.snap_point(Point::new(125.5, 50.0), 5.0);
    assert_eq!(unsnapped, Point::new(125.5, 50.0));
    assert!(lines.is_empty());
}

// The rect moves so its nearest edge or centre meets the target.
#[test]
fn a_moving_rect_snaps_its_nearest_edge_or_centre() {
    let mut targets = SnapTargets::new();
    targets.add(guide(true, 100.0));
    targets.add(guide(false, 50.0));
    // Centre x is 98 (2 from the guide), right edge 120; the top edge is 3 below the guide.
    let (offset, _) = targets.snap_rect(Rect::new(76.0, 53.0, 120.0, 90.0), 4.0);
    assert_eq!(offset, Vec2::new(2.0, -3.0));
    // Out of range on both axes: no move.
    let (offset, lines) = targets.snap_rect(Rect::new(10.0, 10.0, 20.0, 20.0), 4.0);
    assert_eq!(offset, Vec2::ZERO);
    assert!(lines.is_empty());
}

// The result carries the SnapLine of the target.
#[test]
fn snap_lines_show_what_was_snapped_to() {
    let mut targets = SnapTargets::new();
    targets.add_rect(Rect::new(0.0, 0.0, 200.0, 100.0), SnapKind::LayerEdge, SnapKind::LayerCenter);
    let (offset, lines) = targets.snap_rect(Rect::new(52.0, 160.0, 148.0, 190.0), 4.0);
    // The centre x (100) meets the layer's centre line; the top edge (160) is far from every y.
    assert_eq!(offset, Vec2::new(0.0, 0.0));
    let line = lines.iter().find(|l| l.vertical).expect("a vertical line");
    assert_eq!((line.position, line.kind), (100.0, SnapKind::LayerCenter));
    // From the layer's top (0) down to the moving rect's bottom (190).
    assert_eq!(line.span, Some((0.0, 190.0)));
    assert!(line.kind.is_smart());

    let all = SnapTargets::new().with_document(&SnapOptions::default(), Rect::new(0.0, 0.0, 10.0, 10.0), [(true, 4.0)], None);
    let (_, line) = all.snap_axis(true, 4.2, 0.5);
    assert_eq!(line.map(|l| (l.kind, l.span)), Some((SnapKind::Guide, None)));
    let (value, line) = all.filtered(|k| k != SnapKind::Guide).snap_axis(true, 4.2, 0.5);
    assert_eq!((value, line), (4.2, None));
}
