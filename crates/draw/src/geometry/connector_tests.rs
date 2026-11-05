//! Tests of connector routes (task SH-T2), ported from FigCraft's engine tests (`board_route.rs`).

use kurbo::{Point, Rect};

use super::connector::{ConnectionSide, ConnectorPath, ROOM, RouteEnd, Routing, attached_sides, path_handles, reshaped, route, route_along};

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn near(a: Point, b: Point) -> bool {
    (a - b).hypot() < 1e-9
}

const A: Rect = Rect::new(0.0, 0.0, 100.0, 100.0);
const B: Rect = Rect::new(300.0, 300.0, 400.0, 400.0);

// Straight, elbow and curved routes between two boxes and to a free point match FigCraft's cases:
// named sides and free points (`board_tools_tests.rs:94`), curves leave and arrive along the
// sides' normals (`board_connector_tests.rs:102`).
#[test]
fn connector_route_straight_elbow_curved_match_figcraft_cases() {
    use ConnectionSide::{Auto, Bottom, Left, Right, Top};
    let elbow = |from, to| route(from, to, Routing::Elbow);
    let side = |b, s| RouteEnd::Box(b, s);
    let p = |x, y| RouteEnd::Point(pt(x, y));
    // Side to side: out, across halfway, in.
    assert_eq!(elbow(side(A, Right), side(B, Left)), Some(vec![pt(100.0, 50.0), pt(200.0, 50.0), pt(200.0, 350.0), pt(300.0, 350.0)]));
    // A side and a top or bottom: one turn.
    assert_eq!(elbow(side(A, Right), side(B, Top)), Some(vec![pt(100.0, 50.0), pt(350.0, 50.0), pt(350.0, 300.0)]));
    assert_eq!(elbow(side(A, Bottom), side(B, Left)), Some(vec![pt(50.0, 100.0), pt(50.0, 350.0), pt(300.0, 350.0)]));
    // Two free points: the longer way first; in a line, no turns.
    assert_eq!(elbow(p(0.0, 0.0), p(100.0, 40.0)), Some(vec![pt(0.0, 0.0), pt(50.0, 0.0), pt(50.0, 40.0), pt(100.0, 40.0)]));
    assert_eq!(elbow(p(0.0, 0.0), p(100.0, 0.0)), Some(vec![pt(0.0, 0.0), pt(100.0, 0.0)]));
    // An automatic side faces the other end.
    assert_eq!(route(side(A, Auto), p(50.0, 500.0), Routing::Straight), Some(vec![pt(50.0, 100.0), pt(50.0, 500.0)]));
    // On a tie the sideways sides win.
    assert_eq!(route(side(A, Auto), side(B, Auto), Routing::Straight), Some(vec![pt(100.0, 50.0), pt(300.0, 350.0)]));
    // A non-finite point has no route.
    assert_eq!(route(p(f64::NAN, 0.0), p(1.0, 1.0), Routing::Straight), None);
    // Curves: side to side in a line, the control points lie on that line.
    let right_of = Rect::new(300.0, 0.0, 400.0, 100.0);
    assert_eq!(
        route(side(A, Right), side(right_of, Left), Routing::Curved),
        Some(vec![pt(100.0, 50.0), pt(200.0, 50.0), pt(200.0, 50.0), pt(300.0, 50.0)])
    );
    // Bottom to top: out downwards and in from above.
    let c = route(side(A, Bottom), side(B, Top), Routing::Curved).unwrap_or_default();
    assert_eq!(c.len(), 4);
    assert!(c[1].y > c[0].y && c[2].y < c[3].y, "{c:?}");
    // A free end pulls no way.
    let c = route(side(A, Right), p(300.0, 200.0), Routing::Curved).unwrap_or_default();
    assert_eq!(c[2], c[3]);
    // A point placed by hand on the object is the end; its side is the one named.
    let on = RouteEnd::On(A, Right, pt(25.0, 75.0));
    assert_eq!(route(on, p(200.0, 75.0), Routing::Straight), Some(vec![pt(25.0, 75.0), pt(200.0, 75.0)]));
}

// An elbow whose short way would cross a joined box goes around it, keeping `ROOM` from it
// (`board_connector_tests.rs:141`).
#[test]
fn connector_elbow_goes_around_joined_boxes_by_room() {
    use ConnectionSide::{Left, Right};
    let b = Rect::new(300.0, 0.0, 400.0, 100.0);
    // Out of A's left and B's right: the short way runs through both, so it goes over them, the
    // nearer way round (the tie between over and under goes to the top).
    let pts = route(RouteEnd::Box(A, Left), RouteEnd::Box(b, Right), Routing::Elbow);
    assert_eq!(pts, Some(vec![pt(0.0, 50.0), pt(-ROOM, 50.0), pt(-ROOM, -ROOM), pt(400.0 + ROOM, -ROOM), pt(400.0 + ROOM, 50.0), pt(400.0, 50.0)]));
    // Facing each other, the short way stays.
    assert_eq!(route(RouteEnd::Box(A, Right), RouteEnd::Box(b, Left), Routing::Elbow), Some(vec![pt(100.0, 50.0), pt(300.0, 50.0)]));
    // An end at the bottom of the second box leaving towards the first above it: no run goes
    // through either box.
    let pts = route(RouteEnd::Box(A, Left), RouteEnd::Box(b, ConnectionSide::Bottom), Routing::Elbow).unwrap_or_default();
    assert_eq!(pts.last().copied(), Some(pt(350.0, 100.0)));
    let inside = |r: Rect| {
        pts.windows(2).any(|w| {
            w[0].x.min(w[1].x) < r.x1 - 0.5 && w[0].x.max(w[1].x) > r.x0 + 0.5 && w[0].y.min(w[1].y) < r.y1 - 0.5 && w[0].y.max(w[1].y) > r.y0 + 0.5
        })
    };
    assert!(!inside(A) && !inside(b), "{pts:?}");
    assert!(pts.windows(2).all(|w| w[0].x == w[1].x || w[0].y == w[1].y), "right angles only");
}

// `reshaped` of a first or last run keeps it on its end through an extra run; a straight route
// has no path; `attached_sides` gives the sides it left by (`board_reshape_tests.rs:67`, `:123`,
// `:130`; `connector_anywhere_tests.rs:81`).
#[test]
fn connector_reshape_keeps_first_and_last_runs_on_their_ends() {
    use ConnectionSide::{Bottom, Left, Right};
    let a = Rect::new(0.0, 0.0, 100.0, 100.0);
    let b = Rect::new(300.0, 200.0, 400.0, 300.0);
    let (from, to) = (RouteEnd::Box(a, Right), RouteEnd::Box(b, Left));
    let (s, e) = (pt(100.0, 50.0), pt(300.0, 250.0));
    let plain = route(from, to, Routing::Elbow);
    assert_eq!(plain, Some(vec![s, pt(200.0, 50.0), pt(200.0, 250.0), e]));
    // The first run dragged: it gains a run and stays on its end.
    let path = reshaped(from, to, Routing::Elbow, None, 0, pt(150.0, 120.0));
    assert_eq!(
        route_along(from, to, Routing::Elbow, path.as_ref()),
        Some(vec![s, pt(124.0, 50.0), pt(124.0, 120.0), pt(200.0, 120.0), pt(200.0, 250.0), e])
    );
    // The last run of that path dragged, from the path as it is.
    let last = 4;
    let path = reshaped(from, to, Routing::Elbow, path.as_ref(), last, pt(250.0, 200.0));
    assert_eq!(
        route_along(from, to, Routing::Elbow, path.as_ref()),
        Some(vec![s, pt(124.0, 50.0), pt(124.0, 120.0), pt(200.0, 120.0), pt(200.0, 200.0), pt(276.0, 200.0), pt(276.0, 250.0), e])
    );
    // A middle run just moves.
    let mid = reshaped(from, to, Routing::Elbow, None, 1, pt(230.0, 150.0));
    assert_eq!(route_along(from, to, Routing::Elbow, mid.as_ref()), Some(vec![s, pt(230.0, 50.0), pt(230.0, 250.0), e]));
    // Two boxes level with each other: the lone run gains a run at each end.
    let level = RouteEnd::Box(Rect::new(300.0, 0.0, 400.0, 100.0), Left);
    let lone = reshaped(from, level, Routing::Elbow, None, 0, pt(200.0, -40.0));
    assert_eq!(
        route_along(from, level, Routing::Elbow, lone.as_ref()),
        Some(vec![s, pt(124.0, 50.0), pt(124.0, -40.0), pt(276.0, -40.0), pt(276.0, 50.0), pt(300.0, 50.0)])
    );
    // A straight route has no path; a portion it doesn't have is refused; a path is capped.
    assert_eq!(reshaped(from, to, Routing::Straight, None, 0, pt(1.0, 1.0)), None);
    assert_eq!(reshaped(from, to, Routing::Elbow, None, 3, pt(1.0, 1.0)), None);
    let mut path = None;
    let mut tries = 0;
    while let Some(next) = reshaped(from, level, Routing::Elbow, path.as_ref(), 0, pt(150.0, 60.0 + f64::from(tries) * 3.0)) {
        path = Some(next);
        tries += 1;
    }
    assert!((20..40).contains(&tries), "{tries}");
    assert!(route_along(from, level, Routing::Elbow, path.as_ref()).is_some_and(|r| r.len() <= 66));
    // A path for another routing is ignored.
    let bow = ConnectorPath::Bow { x: 5.0, y: 5.0 };
    assert_eq!(route_along(from, to, Routing::Elbow, Some(&bow)), plain);
    // The sides as routed now: named, facing the other end, or none for a free end.
    assert_eq!(attached_sides(RouteEnd::Box(a, Bottom), RouteEnd::Point(pt(0.0, 500.0))), (Some(Bottom), None));
    assert_eq!(attached_sides(RouteEnd::Box(a, ConnectionSide::Auto), RouteEnd::Box(b, ConnectionSide::Auto)), (Some(Right), Some(Left)));
    assert_eq!(attached_sides(RouteEnd::Point(pt(0.0, 0.0)), RouteEnd::On(b, Left, pt(310.0, 250.0))), (None, Some(Left)));
}

// `path_handles`: none for a straight route, one per run of an elbow at its middle, one for a
// curve at its middle.
#[test]
fn connector_path_handles_per_routing() {
    use ConnectionSide::{Left, Right};
    let b = Rect::new(300.0, 200.0, 400.0, 300.0);
    let (from, to) = (RouteEnd::Box(A, Right), RouteEnd::Box(b, Left));
    let straight = route(from, to, Routing::Straight).unwrap_or_default();
    assert!(path_handles(&straight, Routing::Straight).is_empty());
    let elbow = route(from, to, Routing::Elbow).unwrap_or_default();
    assert_eq!(path_handles(&elbow, Routing::Elbow), vec![(0, pt(150.0, 50.0)), (1, pt(200.0, 150.0)), (2, pt(250.0, 250.0))]);
    let curve = route(from, to, Routing::Curved).unwrap_or_default();
    let handles = path_handles(&curve, Routing::Curved);
    assert_eq!(handles.len(), 1);
    // Dragging the middle to a place puts it there, and the ends stay.
    let at = pt(200.0, 40.0);
    let bow = reshaped(from, to, Routing::Curved, None, 0, at);
    let bent = route_along(from, to, Routing::Curved, bow.as_ref()).unwrap_or_default();
    assert!(near(path_handles(&bent, Routing::Curved)[0].1, at), "{bent:?}");
    assert_eq!((bent[0], bent[3]), (curve[0], curve[3]));
}
