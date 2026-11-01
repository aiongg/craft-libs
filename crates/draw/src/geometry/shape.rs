//! Shape outlines and the drag rules that size them: [`ShapeGeometry`] (a live shape's outline as
//! a [`BezPath`]), the box drag ([`box_drag`]), the line drag ([`line_drag`]) and the radial drag
//! ([`RadialFrame::drag`]). No egui: an engine, importer or exporter builds the same outline the
//! Shape tools preview (`craft_draw::ui::shape_drag`).
//!
//! Document units throughout; y grows downwards. Corner radii run top-left, top-right,
//! bottom-right, bottom-left (PhotoCraft's engine order).

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::ops::RangeInclusive;

use kurbo::{BezPath, Point, Rect};

/// The sides a polygon or the points a star may have (PhotoCraft's Sides field, 3–100).
pub const SIDES: RangeInclusive<u32> = 3..=100;

/// The inner radius of a star over its outer radius; 1 is a plain polygon (PhotoCraft's Star
/// ratio field, 1–100 %, stored 0.01–1).
pub const STAR_RATIO: RangeInclusive<f64> = 0.01..=1.0;

/// The smallest side a box drag draws and the shortest line, in document units: anything smaller
/// draws nothing (PhotoCraft).
pub const MIN_SIDE: f64 = 1.0;

/// A shape's outline: what a Shape tool drag, the Create Shape dialog and the Properties panel
/// describe, and what the app's engine draws.
#[derive(Clone, Debug, PartialEq)]
pub enum ShapeGeometry {
    /// A rectangle; `radii` round the corners top-left, top-right, bottom-right, bottom-left (all
    /// 0 for square corners). Photoshop's Rectangle with a radius and VectorCraft's Rounded
    /// Rectangle are this with radii.
    Rect { rect: Rect, radii: [f64; 4] },
    /// An ellipse inscribed in `rect`.
    Ellipse { rect: Rect },
    /// A regular polygon or star fitted to `rect` (the Box frame: PhotoCraft's Triangle and
    /// Polygon). `sides` in [`SIDES`]; `star_ratio` in [`STAR_RATIO`], 1 for a plain polygon.
    /// The Triangle tool is 3 sides.
    Polygon { rect: Rect, sides: u32, star_ratio: f64 },
    /// A regular polygon or star about `center` (the Radial frame: VectorCraft's Polygon and
    /// Star). `radius` reaches the outer points; `rotation` turns the first point from straight
    /// up, in radians, clockwise on screen.
    RadialPolygon { center: Point, radius: f64, rotation: f64, sides: u32, star_ratio: f64 },
    /// A line from `from` to `to`, `weight` wide (Photoshop's Line tool: a filled bar).
    Line { from: Point, to: Point, weight: f64 },
    /// A custom shape's box: the app fits its preset path into `rect`, keeping the preset's
    /// proportions when `keep_proportions` (⇧ held, PhotoCraft's `finish_custom_shape`).
    Custom { rect: Rect, keep_proportions: bool },
}

impl ShapeGeometry {
    /// The outline as a closed path; `tolerance` (document units) bounds the error where a curve
    /// is approximated.
    ///
    /// - `Rect`: the four corners clockwise from the top left, each rounded by its radius (clamped
    ///   to half the shorter side, as kurbo's `RoundedRect` does); all radii 0 is the plain
    ///   rectangle.
    /// - `Ellipse`: kurbo's `Ellipse::from_rect`.
    /// - `Polygon`: the outline of [`RadialPolygon`](Self::RadialPolygon) with the first point
    ///   straight up, scaled into `rect` so its points touch the edges (a triangle's base lies on
    ///   the bottom edge); a star alternates outer and inner points.
    /// - `RadialPolygon`: `sides` outer points on the circle of `radius`, the first at `rotation`
    ///   from straight up; a star (`star_ratio` < 1) puts an inner point between each pair at
    ///   `radius * star_ratio`.
    /// - `Line`: the bar `weight` wide centred on the segment, square ends at `from` and `to`.
    /// - `Custom`: the box itself (the app draws the preset).
    pub fn to_path(&self, tolerance: f64) -> BezPath {
        // pc: crates/ui-egui/src/tools/vector_ui/shapes.rs:73-79 (the kinds: `roundedRect` with four
        // equal radii, ellipse, a polygon of 3 sides for the Triangle, `polygon_sides.max(3)`).
        // new: PhotoCraft's engine draws the outline; here it is kurbo's `RoundedRect`, `Ellipse`
        // and a point list, so engines without one share it.
        todo!()
    }

    /// The axis-aligned box the shape occupies: the rect of the box kinds, the circle's box of a
    /// radial polygon, the box of a line's ends. Space-drag snapping uses it (PhotoCraft's
    /// `shape_bounds`).
    pub fn bounds(&self) -> Rect {
        // pc: crates/ui-egui/src/tools/vector_ui/shapes.rs:85-96 `shape_bounds` (a line's ends' box).
        todo!()
    }
}

/// The box a drag from `start` to `end` draws, in whole pixels, or `None` when a side is under
/// [`MIN_SIDE`].
///
/// - `square` (⇧): both sides take the larger absolute extent, each keeping its direction.
/// - `from_centre` (⌥): `start` is the centre; the box is twice the extent.
/// - The top left and the size round to whole pixels, as PhotoCraft rounds `[x, y, w, h]`.
pub fn box_drag(start: Point, end: Point, square: bool, from_centre: bool) -> Option<Rect> {
    // pc: crates/ui-egui/src/tools/vector_ui/shapes.rs:63-72 (⇧: `m = w.abs().max(h.abs())` with the
    // signs kept; ⌥: origin `start - (w, h)`, size doubled; `[x0.min(x0 + w).round(), …,
    // w.abs().round(), …]`; `rect[2] < 1.0 || rect[3] < 1.0` draws nothing).
    todo!()
}

/// The end of a line dragged from `from` to `to`, or `None` when it is shorter than
/// [`MIN_SIDE`]. `snap` (⇧) turns it to the nearest 45° step and keeps its length.
pub fn line_drag(from: Point, to: Point, snap: bool) -> Option<Point> {
    // pc: crates/ui-egui/src/tools/vector_ui/shapes.rs:52-61 (angle `(dy.atan2(dx) / FRAC_PI_4).round()
    // * FRAC_PI_4`, length `dx.hypot(dy)`; shorter than 1 draws nothing).
    todo!()
}

/// A radial drag's frame: the press is the centre, the pointer sets the radius and the turn
/// (VectorCraft's Polygon and Star tools).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RadialFrame {
    pub center: Point,
    /// From the centre to the pointer, document units.
    pub radius: f64,
    /// The first point's turn from straight up, radians, clockwise on screen.
    pub rotation: f64,
}

impl RadialFrame {
    /// The frame of a drag from `center` to `at`, or `None` when the radius is under
    /// [`MIN_SIDE`]. The first point follows the pointer; `upright` (⇧) keeps the base
    /// orientation (rotation 0, the first point straight up).
    pub fn drag(center: Point, at: Point, upright: bool) -> Option<RadialFrame> {
        // inv-vectorcraft: crates/tools/src/shape.rs:131-137 (polygon and star: the press is the
        // centre, `r = v.hypot()`, rotation `(v.atan2().to_degrees() + 90.0) % 360.0`; Shift keeps the
        // base orientation). VectorCraft's radius floor is 0.01; the library's is `MIN_SIDE`.
        todo!()
    }

    /// The shape of this frame with `sides` points and `star_ratio`.
    pub fn geometry(self, sides: u32, star_ratio: f64) -> ShapeGeometry {
        ShapeGeometry::RadialPolygon { center: self.center, radius: self.radius, rotation: self.rotation, sides, star_ratio }
    }

    /// The star ratio that keeps the inner radius `inner` (held when Ctrl/Cmd went down) while
    /// the outer radius follows the pointer to [`radius`](Self::radius); clamped to
    /// [`STAR_RATIO`].
    pub fn ratio_holding(self, inner: f64) -> f64 {
        // inv-vectorcraft: crates/tools/src/shape.rs:5-7 (Ctrl/Cmd keeps the inner radius while the
        // outer one follows), :32-33 (`inner` held while Ctrl/Cmd is down).
        todo!()
    }

    /// The star ratio whose shoulders are straight (⌥, five points or more): the line between
    /// two outer points two apart passes through the inner points between them. `None` under five
    /// points.
    pub fn straight_shoulders(sides: u32) -> Option<f64> {
        // inv-vectorcraft: crates/tools/src/shape.rs:42-48 `straight_ratio`: `(points >= 5).then(||
        // (2π / n).cos() / (π / n).cos())`.
        todo!()
    }
}
