//! Tests of the distances between boxes (task TO-R1).

use kurbo::{Point, Rect};

use craft_ui::canvas::Orientation;

use super::distances::box_distances;

type Run = (Vec<(Point, Point, String)>, Vec<(Point, Point)>);

fn run(a: Rect, b: Rect) -> Run {
    let (lines, measures) = box_distances(a, b, |len, axis| format!("{len}{}", if axis == Orientation::Horizontal { "x" } else { "y" }));
    assert!(lines.iter().all(|l| l.extension));
    (measures.into_iter().map(|m| (m.a, m.b, m.label)).collect(), lines.into_iter().map(|l| (l.a, l.b)).collect())
}

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

// PhotoCraft's `move_distance` cases: apart on one axis (one gap), overlapping (like edges), diagonal (a line through a's centre and dashed extensions of b's edges), a inside b (four distances to b's edges); zero and non-finite distances are dropped; labels come from the closure with the axis.
#[test]
fn apart_overlapping_diagonal_and_inside() {
    // Apart horizontally, overlapping vertically from 20 to 40; the other axis has like edges.
    let (m, e) = run(Rect::new(0.0, 0.0, 40.0, 40.0), Rect::new(100.0, 20.0, 150.0, 80.0));
    assert_eq!(
        m,
        vec![
            (p(40.0, 30.0), p(100.0, 30.0), "60x".into()),
            (p(20.0, 0.0), p(20.0, 20.0), "20y".into()),
            (p(20.0, 40.0), p(20.0, 80.0), "40y".into())
        ]
    );
    assert_eq!(e, vec![(p(100.0, 20.0), p(20.0, 20.0)), (p(100.0, 80.0), p(20.0, 80.0))]);

    // Diagonal: both lines end beside b, with extensions of its edges.
    let (m, e) = run(Rect::new(100.0, 0.0, 140.0, 20.0), Rect::new(20.0, 70.0, 70.0, 90.0));
    assert_eq!(m, vec![(p(70.0, 10.0), p(100.0, 10.0), "30x".into()), (p(120.0, 20.0), p(120.0, 70.0), "50y".into())]);
    assert_eq!(e, vec![(p(70.0, 70.0), p(70.0, 10.0)), (p(70.0, 70.0), p(120.0, 70.0))]);

    // Inside the canvas: four distances, no extensions.
    let (m, e) = run(Rect::new(40.0, 40.0, 90.0, 80.0), Rect::new(0.0, 0.0, 400.0, 300.0));
    assert_eq!(m.len(), 4);
    assert_eq!((m[0].0, m[0].1), (p(0.0, 60.0), p(40.0, 60.0)));
    assert_eq!((m[3].0, m[3].1), (p(65.0, 80.0), p(65.0, 300.0)));
    assert!(e.is_empty());

    // Touching edges and non-finite boxes give nothing.
    let canvas = Rect::new(0.0, 0.0, 400.0, 300.0);
    assert_eq!(run(canvas, canvas).0, vec![]);
    assert_eq!(run(Rect::new(f64::NAN, 0.0, 1.0, 1.0), canvas).0, vec![]);
}
