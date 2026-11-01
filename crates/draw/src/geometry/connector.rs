//! Connector routes between objects: [`route`], [`route_along`], [`attached_sides`],
//! [`path_handles`] and [`reshaped`], ported from FigCraft's `board_route` (no egui). The
//! Connector tool previews with them (`craft_draw::ui::connector`), and an app's engine routes the
//! connectors it stores with the same functions, so the two never disagree.
//!
//! A connector's route is document geometry: [`ROOM`] is a constant, not a theme token, so a
//! theme never moves a connector.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use kurbo::{Point, Rect, Vec2};
use serde::{Deserialize, Serialize};

/// How far an elbow route going around the objects it joins keeps from them, document units
/// (FigCraft's `ROOM`).
pub const ROOM: f64 = 24.0;

/// How a connector runs between its ends.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Routing {
    #[default]
    Straight,
    /// Right angles only: out of each end along its side's normal, turning once or twice, around
    /// the objects it joins when the short way would cross them.
    Elbow,
    /// One smooth curve, leaving and arriving along the sides' normals.
    Curved,
}

/// The side of an object a connector end leaves by.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionSide {
    /// The side facing the other end.
    #[default]
    Auto,
    Top,
    Right,
    Bottom,
    Left,
}

impl ConnectionSide {
    /// The four sides clockwise from the top, the default connection sites' order.
    pub const ALL: [ConnectionSide; 4] = [ConnectionSide::Top, ConnectionSide::Right, ConnectionSide::Bottom, ConnectionSide::Left];

    /// The side's middle on `bounds`; the centre for [`Auto`](Self::Auto).
    pub fn point(self, bounds: Rect) -> Point {
        let c = bounds.center();
        match self {
            ConnectionSide::Top => Point::new(c.x, bounds.y0),
            ConnectionSide::Right => Point::new(bounds.x1, c.y),
            ConnectionSide::Bottom => Point::new(c.x, bounds.y1),
            ConnectionSide::Left => Point::new(bounds.x0, c.y),
            ConnectionSide::Auto => c,
        }
    }

    /// Left or right.
    pub fn horizontal(self) -> bool {
        matches!(self, ConnectionSide::Left | ConnectionSide::Right)
    }

    /// The way out of the side (y down); zero for [`Auto`](Self::Auto).
    pub fn normal(self) -> Vec2 {
        match self {
            ConnectionSide::Top => Vec2::new(0.0, -1.0),
            ConnectionSide::Right => Vec2::new(1.0, 0.0),
            ConnectionSide::Bottom => Vec2::new(0.0, 1.0),
            ConnectionSide::Left => Vec2::new(-1.0, 0.0),
            ConnectionSide::Auto => Vec2::ZERO,
        }
    }
}

/// One end of a route.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RouteEnd {
    /// A free end at a point.
    Point(Point),
    /// Glued to the middle of a side of the box (or the side facing the other end).
    Box(Rect, ConnectionSide),
    /// Glued to a point on the object in the box, placed by hand or a numbered site, leaving by the
    /// side given.
    On(Rect, ConnectionSide, Point),
}

/// A connector's path as reshaped by hand, measured from the middle of its two ends so the shape
/// travels with what it joins.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConnectorPath {
    /// An elbow's straight runs, each at right angles to the one before: whether the first runs
    /// across (else up or down), and where the runs between the first and last lie (x of one that
    /// runs up or down, y of one that runs across). The first and last lie on the ends.
    Runs { across: bool, at: Vec<f64> },
    /// How far a curve's middle was moved.
    Bow { x: f64, y: f64 },
}

/// The points a connector runs through from `from` to `to`: from the middle of the side of each
/// box facing the other end (or the side it names), straight, in right angles keeping [`ROOM`]
/// from the joined boxes, or (curved) its two ends with the curve's two control points between
/// them. `None` when a point isn't finite.
pub fn route(from: RouteEnd, to: RouteEnd, routing: Routing) -> Option<Vec<Point>> {
    // inv-figcraft: crates/doc/src/board_route.rs:34-44 `route_points` (with its private `attach`,
    // `tidy`, `elbow`, `curve`; port them with their tests).
    todo!()
}

/// [`route`] for a connector whose `path` may have been reshaped by hand; a path that isn't for
/// its `routing` is ignored.
pub fn route_along(from: RouteEnd, to: RouteEnd, routing: Routing, path: Option<&ConnectorPath>) -> Option<Vec<Point>> {
    // inv-figcraft: crates/doc/src/board_route.rs:48-62 `route_along` (`along_runs`; a bow moves both
    // control points by 4/3 of it).
    todo!()
}

/// The sides a connector's ends leave by as routed now; `None` for a free end.
pub fn attached_sides(from: RouteEnd, to: RouteEnd) -> (Option<ConnectionSide>, Option<ConnectionSide>) {
    // inv-figcraft: crates/doc/src/board_route.rs:65-67 `attached_sides`.
    todo!()
}

/// The route's path handles: one per run of an elbow and one for a curve, each with the portion
/// of the path it moves and where it sits. None for a straight route.
pub fn path_handles(points: &[Point], routing: Routing) -> Vec<(usize, Point)> {
    // inv-figcraft: crates/doc/src/board_route.rs:71-77 `path_handles`.
    todo!()
}

/// The path after dragging `portion`'s handle to `at`: an elbow's run moves there, and a first or
/// last run stays on its end through an extra run; a curve's middle moves there. `None` for a
/// straight connector, a portion it doesn't have, or a path of too many runs (64).
pub fn reshaped(from: RouteEnd, to: RouteEnd, routing: Routing, path: Option<&ConnectorPath>, portion: usize, at: Point) -> Option<ConnectorPath> {
    // inv-figcraft: crates/doc/src/board_route.rs:82-91 `reshaped`, :94 `MAX_RUNS`, :97 `reshaped_runs`.
    todo!()
}
