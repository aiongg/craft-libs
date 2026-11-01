//! The Shape tools on the canvas ([`ShapeDragTool`]: Rectangle, Rounded Rectangle, Ellipse,
//! Triangle, Polygon, Star, Line and Custom Shape) and the Create Shape dialog a click opens
//! ([`CreateShapeDialog`]).
//!
//! ```no_run
//! use craft_draw::ui::shape_drag::{CreateShapeDialog, CreateShapeOptions, ShapeDragEvent, ShapeDragOptions, ShapeDragTool, ShapeKind};
//! use craft_ui::canvas::{CanvasId, CanvasView, ToolHost};
//! use craft_ui::dialog::{DialogHost, GuideUnits};
//! use craft_ui::edit::EditEvent;
//! # fn demo(ui: &mut egui::Ui, view: &CanvasView, canvas: &egui::Response, id: CanvasId, tool: &mut ShapeDragTool, dialogs: &mut DialogHost, units: GuideUnits) {
//! tool.options(ShapeDragOptions { kind: ShapeKind::Rectangle, ..ShapeDragOptions::default() });
//! let _ = ToolHost::new(id).show(ui, view, canvas, tool);
//! for event in tool.take_events() {
//!     match event {
//!         ShapeDragEvent::Gesture(EditEvent::Commit { value, changed: true, .. }) => { /* create the shape `value` */ }
//!         ShapeDragEvent::Click { at, kind, .. } if CreateShapeDialog::supports(kind) => {
//!             dialogs.open(CreateShapeDialog::new(CreateShapeOptions::initial(kind, at).units(units)));
//!         }
//!         _ => {}
//!     }
//! }
//! # }
//! ```

use egui::Modifiers;
use kurbo::Point;

use crate::geometry::shape::ShapeGeometry;
use craft_ui::edit::EditEvent;

mod dialog;
#[cfg(test)]
mod dialog_tests;
mod labels;
pub(crate) mod stories;
mod style;
mod tool;
#[cfg(test)]
mod tool_tests;

pub use dialog::{CreateShapeDialog, CreateShapeOptions, CreatedShape};
pub use labels::{CreateShapeLabels, ShapeDragNames};
pub use style::ShapeDragStyle;
pub use tool::ShapeDragTool;

/// The Shape tools. Photoshop's Rectangle draws a rounded rectangle when the corner radius is over
/// 0; VectorCraft (Illustrator) has its own Rounded Rectangle and Star.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ShapeKind {
    #[default]
    Rectangle,
    RoundedRectangle,
    Ellipse,
    Triangle,
    Polygon,
    Star,
    Line,
    /// The selected custom shape preset: the tool gives its box and the app fits the preset in it.
    Custom,
}

/// How a drag sizes the shape.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ShapeFrame {
    /// The press is one corner of the shape's box, the pointer the other (Photoshop: every kind).
    #[default]
    Box,
    /// The press is the centre, the pointer sets the radius and the turn (Illustrator's Polygon
    /// and Star, [`RadialFrame`](crate::geometry::shape::RadialFrame)).
    Radial,
}

/// The tool options a drag reads each pass: the kind from the selected tool, the rest from the
/// options bar (PhotoCraft's `ToolOptions`). [`ShapeToolSettings::drag_options`](super::shape::ShapeToolSettings::drag_options)
/// builds it from the stored settings.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShapeDragOptions {
    pub kind: ShapeKind,
    /// The Rectangle's corner radius; over 0 draws four equal radii.
    pub corner_radius: f64,
    /// The polygon's sides or the star's points, in [`SIDES`](crate::geometry::shape::SIDES).
    pub sides: u32,
    /// The star's inner over outer radius, in [`STAR_RATIO`](crate::geometry::shape::STAR_RATIO).
    pub star_ratio: f64,
    /// The Line's weight, at least 1.
    pub line_weight: f64,
}

impl Default for ShapeDragOptions {
    /// Rectangle, square corners, 5 sides, a plain polygon, weight 3 (PhotoCraft's defaults).
    fn default() -> Self {
        ShapeDragOptions { kind: ShapeKind::Rectangle, corner_radius: 0.0, sides: 5, star_ratio: 1.0, line_weight: 3.0 }
    }
}

/// Which product's drag rules the tool follows. Photoshop's are the default; VectorCraft uses
/// [`ILLUSTRATOR`](Self::ILLUSTRATOR).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ShapeDragRules {
    /// Polygon and Star use the [`Radial`](ShapeFrame::Radial) frame.
    pub radial_polygons: bool,
    /// ↑ and ↓ during a Polygon or Star drag add or remove a side or point (3 to 100).
    pub arrow_sides: bool,
    /// Ctrl/Cmd held during a Star drag keeps the inner radius while the outer follows.
    pub hold_inner: bool,
    /// ⌥ during a Star drag of five points or more makes the shoulders straight (and no longer
    /// draws from the centre: the radial frame already does).
    pub straight_shoulders: bool,
}

impl ShapeDragRules {
    /// Photoshop's (and PhotoCraft's): every kind in the Box frame, no arrow or hold rules.
    pub const PHOTOSHOP: ShapeDragRules = ShapeDragRules { radial_polygons: false, arrow_sides: false, hold_inner: false, straight_shoulders: false };
    /// Illustrator's (VectorCraft's): Polygon and Star radial, ↑/↓ change the count, Ctrl/Cmd holds
    /// the inner radius, ⌥ straightens the shoulders.
    pub const ILLUSTRATOR: ShapeDragRules = ShapeDragRules { radial_polygons: true, arrow_sides: true, hold_inner: true, straight_shoulders: true };

    /// The frame `kind` is drawn in under these rules.
    pub fn frame(self, kind: ShapeKind) -> ShapeFrame {
        match kind {
            ShapeKind::Polygon | ShapeKind::Star if self.radial_polygons => ShapeFrame::Radial,
            _ => ShapeFrame::Box,
        }
    }
}

impl Default for ShapeDragRules {
    fn default() -> Self {
        ShapeDragRules::PHOTOSHOP
    }
}

/// What [`ShapeDragTool`] reports, drained with [`ShapeDragTool::take_events`] after
/// `ToolHost::show`.
#[derive(Clone, Debug, PartialEq)]
pub enum ShapeDragEvent {
    /// The drag: `Begin` once the pointer is past the click slop, `Update` each pass with the
    /// shape it would draw, `Commit` on release (`changed: false` when a side is under 1 px, so the
    /// app makes nothing), `Cancel` on Esc or a tool switch.
    Gesture(EditEvent<ShapeGeometry>),
    /// A press and release within the click slop, or Enter with the canvas focused (at the view
    /// centre): the app opens [`CreateShapeDialog`] for kinds it [supports](CreateShapeDialog::supports).
    Click { at: Point, kind: ShapeKind, modifiers: Modifiers },
}
