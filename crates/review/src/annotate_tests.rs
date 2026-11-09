//! Annotation geometry.

use kurbo::{Point, Rect, Vec2};

use crate::annotate::{Annotation, AnnotationHandle, AnnotationHit, AnnotationId, AnnotationKind, handles, hit, reshaped, resized};

/// The icon side, in page units, the hit tests are given.
const ICON: f64 = 16.0;

fn annotation(id: u64, kind: AnnotationKind, rect: Rect) -> Annotation {
    Annotation {
        id: AnnotationId(id),
        page: 0,
        kind,
        rect,
        points: Vec::new(),
        strokes: Vec::new(),
        color: [255, 0, 0],
        opacity: 1.0,
        stroke_width: 2.0,
        text: String::new(),
        locked: false,
    }
}

fn square(id: u64, kind: AnnotationKind) -> Annotation {
    annotation(id, kind, Rect::new(10.0, 10.0, 50.0, 30.0))
}

// handles by kind; none for markups and locked ones.
#[test]
fn box_kinds_have_eight_handles_and_pointwise_their_points() {
    let rectangle = square(1, AnnotationKind::Rectangle);
    let all = handles(&rectangle);
    assert_eq!(all.len(), 8);
    assert!(all.iter().all(|(handle, _)| matches!(handle, AnnotationHandle::Box(..))));

    let mut line = square(2, AnnotationKind::Line);
    line.points = vec![Point::new(0.0, 0.0), Point::new(5.0, 5.0)];
    let ends = handles(&line);
    assert_eq!(ends.len(), line.points.len());
    assert!(ends.iter().all(|(handle, _)| matches!(handle, AnnotationHandle::Point(..))));

    assert!(handles(&square(3, AnnotationKind::Highlight)).is_empty());
    let mut locked = square(4, AnnotationKind::Rectangle);
    locked.locked = true;
    assert!(handles(&locked).is_empty());
}

// hit order: selected handles, then last-drawn bodies.
#[test]
fn hit_prefers_handles_then_the_topmost_body() {
    let (below, above) = (square(1, AnnotationKind::Rectangle), square(2, AnnotationKind::Rectangle));
    let all = [below, above];
    let inside = Point::new(30.0, 20.0);
    assert_eq!(hit(&all, &[], inside, 2.0, ICON), Some(AnnotationHit::Body(AnnotationId(2))));
    let corner = Point::new(11.0, 11.0);
    assert_eq!(hit(&all, &[AnnotationId(1)], corner, 2.0, ICON), Some(AnnotationHit::Handle(AnnotationId(1), AnnotationHandle::Box(-1, -1))));
    assert_eq!(hit(&all, &[], Point::new(200.0, 200.0), 2.0, ICON), None);

    let mut line = square(3, AnnotationKind::Line);
    line.points = vec![Point::new(100.0, 100.0), Point::new(200.0, 100.0)];
    assert_eq!(hit(&[line.clone()], &[], Point::new(150.0, 101.0), 2.0, ICON), Some(AnnotationHit::Body(AnnotationId(3))));
    assert_eq!(hit(&[line], &[], Point::new(150.0, 110.0), 2.0, ICON), None);

    // Note, Attach and Caret keep their icon size on screen: a hit tests the icon side given for
    // this zoom, not the stored rect.
    let note = annotation(5, AnnotationKind::Note, Rect::new(100.0, 100.0, 100.0, 100.0));
    let probe = Point::new(108.0, 108.0);
    assert_eq!(hit(std::slice::from_ref(&note), &[], probe, 0.0, ICON), Some(AnnotationHit::Body(AnnotationId(5))));
    assert_eq!(hit(std::slice::from_ref(&note), &[], probe, 0.0, 4.0), None, "zoomed in, the icon covers fewer page units");
    assert_eq!(hit(std::slice::from_ref(&note), &[], probe, 0.0, 40.0), Some(AnnotationHit::Body(AnnotationId(5))));
}

// resized keeps the ratio with an aspect and never inverts.
#[test]
fn resize_keeps_the_aspect_with_shift() {
    let rect = Rect::new(0.0, 0.0, 40.0, 20.0);
    let ratio = rect.width() / rect.height();
    let delta = Vec2::new(40.0, 0.0);
    let grown = resized(rect, (1, 1), delta, Some(ratio));
    assert!((grown.width() / grown.height() - ratio).abs() < 1e-9, "the ratio is kept");
    assert!(grown.width() > rect.width());
    assert_eq!((grown.x0, grown.y0), (rect.x0, rect.y0), "the opposite corner stays");
    let free = resized(rect, (1, 1), delta, None);
    assert!(free.width() > rect.width());
    assert_eq!(free.height(), rect.height(), "without an aspect only the dragged axis changes");

    // Dragging past the opposite edge flips the box instead of inverting it.
    let flipped = resized(rect, (1, 0), Vec2::new(-100.0, 0.0), None);
    assert!(flipped.x0 < flipped.x1);
    assert_eq!(flipped.x1, rect.x0, "the fixed edge stays");
    assert_eq!(flipped.height(), rect.height());
    let top_left = resized(rect, (-1, -1), Vec2::new(-20.0, 0.0), Some(ratio));
    assert_eq!((top_left.x1, top_left.y1), (rect.x1, rect.y1), "the opposite corner stays");
    assert!((top_left.width() / top_left.height() - ratio).abs() < 1e-9);
}

// reshaped with shift snaps the segment.
#[test]
fn reshape_snaps_to_45_degrees() {
    let points = [Point::new(0.0, 0.0), Point::new(10.0, 0.0)];
    let free = reshaped(&points, 1, Point::new(10.0, 3.0), false);
    assert_eq!(free[1], Point::new(10.0, 3.0));
    let snapped = reshaped(&points, 1, Point::new(10.0, 3.0), true);
    assert!((snapped[1].x - 10.0_f64.hypot(3.0)).abs() < 1e-9 && snapped[1].y.abs() < 1e-9, "{snapped:?}");
    let diagonal = reshaped(&points, 1, Point::new(8.0, 7.0), true);
    assert!((diagonal[1].x - diagonal[1].y).abs() < 1e-9);
    assert_eq!(snapped[0], points[0]);
}
