//! The shape layer's Properties sections ([`ShapePanel`]) and the Shape tools' options bar
//! ([`ShapeBar`]), each field a gesture-aware [`EditEvent<ShapeChange>`](craft_ui::edit::EditEvent).
//!
//! ```no_run
//! use craft_draw::ui::shape::{LiveShape, ShapeAppearance, ShapeChange, ShapePanel};
//! use craft_ui::edit::EditEvent;
//! # fn demo(ui: &mut egui::Ui, shape: &LiveShape, look: &ShapeAppearance, stroke_open: &mut bool) {
//! let out = ShapePanel::new("shape-panel", shape, look).stroke_open(stroke_open).show(ui);
//! for event in out.events {
//!     if let EditEvent::Commit { value: ShapeChange::Size { width, height }, changed: true, .. } = event {
//!         // one `shape.edit` command with the new size
//!         let _ = (width, height);
//!     }
//! }
//! # }
//! ```

use craft_color::ui::Paint;
use serde::{Deserialize, Serialize};

use crate::stroke::StrokeStyle;

use super::shape_drag::{ShapeDragOptions, ShapeKind};

mod bar;
#[cfg(test)]
mod bar_tests;
mod labels;
mod panel;
#[cfg(test)]
mod panel_tests;
pub(crate) mod stories;

pub use bar::{ShapeBar, ShapeBarOutput};
pub use labels::ShapeLabels;
pub use panel::{ShapePaint, ShapePanel, ShapePanelOutput};

/// The selected shape layer's live shape, as the Shape section shows it: plain data the app
/// converts to and from its document. Sizes in document pixels.
#[derive(Clone, Debug, PartialEq)]
pub enum LiveShape {
    /// A rectangle: radii top-left, top-right, bottom-right, bottom-left.
    Rect {
        /// Width in document pixels.
        width: f64,
        /// Height in document pixels.
        height: f64,
        /// The four corner radii in document pixels.
        radii: [f64; 4],
    },
    /// An ellipse.
    Ellipse {
        /// Width in document pixels.
        width: f64,
        /// Height in document pixels.
        height: f64,
    },
    /// A polygon or star: one corner radius for every corner.
    Polygon {
        /// Width in document pixels.
        width: f64,
        /// Height in document pixels.
        height: f64,
        /// The corner radius in document pixels.
        corner_radius: f64,
        /// Number of sides, or points of a star.
        sides: u32,
        /// Inner radius over outer radius; 1 for a plain polygon.
        star_ratio: f64,
    },
    /// A line.
    Line {
        /// The line's weight in document pixels.
        weight: f64,
    },
}

/// A shape's fill and stroke as the Appearance section and the bar show them: the selected
/// layer's, or the tool defaults when no shape layer is selected (the app passes the right one).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ShapeAppearance {
    /// The fill paint.
    pub fill: Paint,
    /// The stroke paint.
    pub stroke: Paint,
    /// Stroke width in pixels; 0 is no stroke.
    pub stroke_width: f64,
    /// The stroke's style (alignment, caps, corners, dashes): the value the Stroke style button
    /// shows and edits. Its `width` is ignored here; [`stroke_width`](Self::stroke_width) is the
    /// width.
    pub stroke_style: StrokeStyle,
    /// The layer is locked (`locks.all` or `.pixels`): every field is disabled.
    pub locked: bool,
}

impl ShapeAppearance {
    /// Fill and stroke with the default stroke style, unlocked.
    pub fn new(fill: Paint, stroke: Paint, stroke_width: f64) -> Self {
        ShapeAppearance { fill, stroke, stroke_width, stroke_style: StrokeStyle::default(), locked: false }
    }

    /// The stroke's style.
    pub fn stroke_style(mut self, style: StrokeStyle) -> Self {
        self.stroke_style = style;
        self
    }

    /// Disables every field.
    pub fn locked(mut self, locked: bool) -> Self {
        self.locked = locked;
        self
    }
}

/// One field's change. The app turns it into one document command (and, from the options bar with
/// a shape layer selected, into the tool defaults too: storytold/photocraft#3347).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ShapeChange {
    /// The fill paint. The app also sets its foreground colour from a solid fill (PhotoCraft).
    Fill(Paint),
    /// The options bar's Fill check box (tool defaults only).
    FillOn(bool),
    /// The stroke paint. The app also sets its background colour from a solid stroke. `width` is
    /// `Some(`[`ENABLED_STROKE_WIDTH`](Self::ENABLED_STROKE_WIDTH)`)` when the edit turns the
    /// stroke on from width 0 (the app sets the width with the paint, in one command), else `None`.
    Stroke {
        /// The new stroke paint.
        paint: Paint,
        /// The width to set along with the paint.
        width: Option<f64>,
    },
    /// The stroke width in pixels, 0–288. A step that clamps at 0 is no change.
    StrokeWidth(f64),
    /// The Stroke style button's edit (its `EditEvent<StrokeStyle>`, mapped); `width` as in
    /// [`Stroke`](Self::Stroke).
    StrokeStyle {
        /// The new stroke style.
        style: StrokeStyle,
        /// The width to set along with the style.
        width: Option<f64>,
    },
    /// Width and height of a rect, ellipse or polygon.
    Size {
        /// New width in document pixels.
        width: f64,
        /// New height in document pixels.
        height: f64,
    },
    /// A rectangle's four radii (the "Corner radius" field sets all four).
    Radii([f64; 4]),
    /// A polygon's corner radius, or the options bar's Radius.
    CornerRadius(f64),
    /// A polygon's number of sides.
    Sides(u32),
    /// A star's inner over outer radius.
    StarRatio(f64),
    /// A line's weight.
    Weight(f64),
}

impl ShapeChange {
    /// The stroke width a stroke gets when an edit turns it on at width 0 (PhotoCraft's
    /// `enabled_stroke_width`).
    pub const ENABLED_STROKE_WIDTH: f64 = 3.0;
}

/// The Shape tools' persisted options-bar values (Fill on, radius, sides, star ratio, weight,
/// stroke width); the paints are the app's foreground and background.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[non_exhaustive]
pub struct ShapeToolSettings {
    /// Whether new shapes get a fill.
    pub fill_on: bool,
    /// The Rectangle's corner radius.
    pub corner_radius: f64,
    /// A polygon's sides or a star's points.
    pub sides: u32,
    /// A star's inner over outer radius; 1 for a plain polygon.
    pub star_ratio: f64,
    /// The Line's weight.
    pub line_weight: f64,
    /// Stroke width of new shapes, in pixels; 0 is no stroke.
    pub stroke_width: f64,
}

impl Default for ShapeToolSettings {
    /// Fill on, square corners, 5 sides, a plain polygon, weight 3, no stroke (PhotoCraft's
    /// `ToolOptions::default`, `state/tool_options.rs:405-412`).
    fn default() -> Self {
        ShapeToolSettings { fill_on: true, corner_radius: 0.0, sides: 5, star_ratio: 1.0, line_weight: 3.0, stroke_width: 0.0 }
    }
}

impl ShapeToolSettings {
    /// The drag options of `kind` with these values.
    pub fn drag_options(&self, kind: ShapeKind) -> ShapeDragOptions {
        ShapeDragOptions { kind, corner_radius: self.corner_radius, sides: self.sides, star_ratio: self.star_ratio, line_weight: self.line_weight }
    }
}
