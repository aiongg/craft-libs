//! Annotation geometry.

use kurbo::{Point, Rect, Vec2};

use crate::annotate::{
    Annotation, AnnotationGroup, AnnotationHandle, AnnotationHit, AnnotationId, AnnotationKind, AnnotationTool, handles, hit, reshaped, resized,
};

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
}

// resized keeps the ratio and never inverts.
#[test]
fn resize_keeps_the_aspect_with_shift() {
    let rect = Rect::new(0.0, 0.0, 40.0, 20.0);
    let grown = resized(rect, (1, 1), Vec2::new(40.0, 0.0), Some(2.0));
    assert_eq!((grown.x0, grown.y0, grown.width(), grown.height()), (0.0, 0.0, 80.0, 40.0));
    let free = resized(rect, (1, 1), Vec2::new(40.0, 0.0), None);
    assert_eq!((free.width(), free.height()), (80.0, 20.0));
    // A plain resize adds the delta and normalises: dragging past the other edge flips the box.
    let flipped = resized(rect, (1, 0), Vec2::new(-100.0, 0.0), None);
    assert_eq!((flipped.x0, flipped.x1, flipped.height()), (-60.0, 0.0, 20.0));
    let top_left = resized(rect, (-1, -1), Vec2::new(-20.0, 0.0), Some(2.0));
    assert_eq!((top_left.x1, top_left.y1, top_left.width()), (40.0, 20.0, 60.0));
}

// reshaped with shift snaps the segment.

// AnnotationGroup::tools order and Eraser in Draw.

// Note, Attach and Caret keep their icon size on screen: a hit tests the icon side given for this
// zoom, not the stored rect.
#[test]
fn icon_kinds_hit_the_icon_side_given() {
    let note = annotation(5, AnnotationKind::Note, Rect::new(100.0, 100.0, 100.0, 100.0));
    assert_eq!(hit(std::slice::from_ref(&note), &[], Point::new(108.0, 108.0), 0.0, ICON), Some(AnnotationHit::Body(AnnotationId(5))));
    assert_eq!(hit(std::slice::from_ref(&note), &[], Point::new(108.0, 108.0), 0.0, 4.0), None, "zoomed in, the icon covers fewer page units");
    assert_eq!(hit(std::slice::from_ref(&note), &[], Point::new(108.0, 108.0), 0.0, 40.0), Some(AnnotationHit::Body(AnnotationId(5))));
}
