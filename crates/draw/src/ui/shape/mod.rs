//! The shape layer's Properties sections ([`ShapePanel`]) and the Shape tools' options bar
//! ([`ShapeOptionsBar`]), each field a gesture-aware [`EditEvent<ShapeChange>`](craft_ui::edit::EditEvent).
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

use super::shape_drag::{ShapeDragOptions, ShapeKind};

mod bar;
#[cfg(test)]
mod bar_tests;
mod labels;
mod panel;
#[cfg(test)]
mod panel_tests;
pub(crate) mod stories;

pub use bar::{ShapeOptionsBar, ShapeOptionsOutput};
pub use labels::ShapeLabels;
pub use panel::{ShapePaint, ShapePanel, ShapePanelOutput};

/// The selected shape layer's live shape, as the Shape section shows it: plain data the app
/// converts to and from its document. Sizes in document pixels.
#[derive(Clone, Debug, PartialEq)]
pub enum LiveShape {
    /// A rectangle: radii top-left, top-right, bottom-right, bottom-left.
    Rect {
        width: f64,
        height: f64,
        radii: [f64; 4],
    },
    Ellipse {
        width: f64,
        height: f64,
    },
    /// A polygon or star: one corner radius for every corner.
    Polygon {
        width: f64,
        height: f64,
        corner_radius: f64,
        sides: u32,
        star_ratio: f64,
    },
    Line {
        weight: f64,
    },
}

/// A shape's fill and stroke as the Appearance section and the bar show them: the selected
/// layer's, or the tool defaults when no shape layer is selected (the app passes the right one).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ShapeAppearance {
    pub fill: Paint,
    pub stroke: Paint,
    /// Stroke width in pixels; 0 is no stroke.
    pub stroke_width: f64,
    // needs: PE craft_draw::stroke::StrokeStyle: `pub stroke_style: StrokeStyle,` (the value the
    // Stroke style button shows and edits).
    /// The layer is locked (`locks.all` or `.pixels`): every field is disabled.
    pub locked: bool,
}

impl ShapeAppearance {
    /// Fill and stroke, unlocked.
    pub fn new(fill: Paint, stroke: Paint, stroke_width: f64) -> Self {
        ShapeAppearance { fill, stroke, stroke_width, locked: false }
    }

    /// Disables every field.
    pub fn locked(mut self, locked: bool) -> Self {
        self.locked = locked;
        self
    }
}

/// One field's change. The app turns it into one document command (and, from the options bar with
/// a shape layer selected, into the tool defaults too: PhotoCraft #3347).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ShapeChange {
    /// The fill paint. The app also sets its foreground colour from a solid fill (PhotoCraft).
    Fill(Paint),
    /// The options bar's Fill check box (tool defaults only).
    FillOn(bool),
    /// The stroke paint. The app also sets its background colour from a solid stroke.
    Stroke(Paint),
    /// The stroke width in pixels, 0–288. A stroke turned on without a width carries
    /// [`ShapeChange::ENABLED_STROKE_WIDTH`]; a step that clamps at 0 is no change.
    StrokeWidth(f64),
    // needs: PE craft_draw::stroke::StrokeStyle: `StrokeStyle(StrokeStyle),` (the Stroke style
    // button's `EditEvent<StrokeStyle>`, mapped).
    /// Width and height of a rect, ellipse or polygon.
    Size {
        width: f64,
        height: f64,
    },
    /// A rectangle's four radii (the "Corner radius" field sets all four).
    Radii([f64; 4]),
    /// A polygon's corner radius, or the options bar's Radius.
    CornerRadius(f64),
    Sides(u32),
    StarRatio(f64),
    /// A line's weight.
    Weight(f64),
}

impl ShapeChange {
    /// The stroke width a stroke gets when it is turned on at 0 (PhotoCraft's
    /// `enabled_stroke_width`).
    pub const ENABLED_STROKE_WIDTH: f64 = 3.0;
}

/// The Shape tools' persisted options-bar values (Fill on, radius, sides, star ratio, weight,
/// stroke width); the paints are the app's foreground and background.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
#[non_exhaustive]
pub struct ShapeToolSettings {
    pub fill_on: bool,
    pub corner_radius: f64,
    pub sides: u32,
    pub star_ratio: f64,
    pub line_weight: f64,
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
