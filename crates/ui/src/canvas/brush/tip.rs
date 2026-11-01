//! The brush tip as the cursor draws it.
#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::sync::Arc;

use kurbo::{Rect, Vec2};

/// A brush tip: its diameter and the outline the cursor draws. The app builds it from its engine
/// when the brush changes (an aliased tip's cell also when the pointer moves); every length is in
/// document units.
#[derive(Clone, Debug, PartialEq)]
pub struct BrushTip {
    /// The brush's diameter (its Size).
    pub diameter: f64,
    pub outline: BrushTipOutline,
}

impl BrushTip {
    /// A plain round tip of `diameter` whose 50 % contour is at `normal_radius` (the engine's
    /// half-coverage radius for its hardness) and whose full reach is `diameter / 2`.
    pub fn round(diameter: f64, normal_radius: f64) -> BrushTip {
        BrushTip { diameter, outline: BrushTipOutline::Round { normal_radius, full_radius: diameter / 2.0 } }
    }
}

/// The outline of a tip, for Preferences › Cursors' Normal Brush Tip (where a dab reaches 50 %)
/// and Full Size Brush Tip (where it reaches 0 %).
#[derive(Clone, Debug, PartialEq)]
pub enum BrushTipOutline {
    /// A plain round tip: two circles, radii in document units. Under a skewed pose or a view
    /// with a pixel aspect the circle becomes an ellipse.
    Round { normal_radius: f64, full_radius: f64 },
    /// A shaped tip (a sampled bitmap, a turned or flattened round tip) traced by the engine:
    /// rings in radius units (1 is `diameter / 2`) in document orientation, for each preference.
    Traced { normal: TipRings, full: TipRings },
    /// An aliased tip (the Pencil, the Eraser in Pencil or Block mode): the whole-pixel `cell` its
    /// dab fills at the pointer (PhotoCraft's `paint::grid_square`), drawn on the pixel grid as a
    /// disc when `round`, else as the square. The OS pointer hides; see
    /// [`BrushCursor::paint_pixels`](super::BrushCursor::paint_pixels).
    Pixels { cell: Rect, round: bool },
}

/// Closed rings of a traced tip outline, points in radius units `[x, y]` (document orientation,
/// y down), as the engine's tip tracer returns them. Shared: the app keeps one value per brush
/// and the cursor maps it each pass without retracing, whatever the pose or view.
#[derive(Clone, Debug, PartialEq)]
pub struct TipRings(Arc<[Vec<[f32; 2]>]>);

impl TipRings {
    /// Rings from the engine's tracer.
    pub fn new(rings: Vec<Vec<[f32; 2]>>) -> TipRings {
        TipRings(rings.into())
    }

    /// The rings.
    pub fn rings(&self) -> &[Vec<[f32; 2]>] {
        &self.0
    }

    /// Whether `self` and `other` are the same shared rings (not merely equal): the app's cache
    /// check.
    pub fn same(&self, other: &TipRings) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

/// The tip's own axes in document space for one radius: where the tip's x and y directions point
/// after the brush angle and the pen's pose, before the view. Upright is `(1, 0)` and `(0, 1)`.
/// The cursor maps both through the view ([`CanvasView::map_vec`](super::super::CanvasView::map_vec)),
/// so the pose comes before zoom, rotation and flip, and a cached outline is only re-mapped.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TipPose {
    pub x_axis: Vec2,
    pub y_axis: Vec2,
}

impl Default for TipPose {
    fn default() -> Self {
        TipPose::upright()
    }
}

impl TipPose {
    /// No pose: the document's axes.
    pub const fn upright() -> TipPose {
        TipPose { x_axis: Vec2::new(1.0, 0.0), y_axis: Vec2::new(0.0, 1.0) } // design: allow(unit axes, not sizes)
    }

    /// The pose of a pen: the tip turned clockwise by `degrees` (the app's angle from its shape
    /// dynamics: pen tilt or barrel rotation, plus the barrel rotation with Brush Projection), and
    /// with `tilt` (Brush Projection on; the pen's tilt x and y in degrees) shortened along the
    /// tilt direction by `cos(|tilt|)`, at least 0.05, as a tilted pen's footprint. A zero tilt
    /// or a non-finite value leaves that step out.
    pub fn pen(degrees: f64, tilt: Option<[f32; 2]>) -> TipPose {
        // pc: crates/ui-egui/src/tools/brush_cursor.rs:55 `pen_axes` (x = (cos, −sin), y = (sin, cos) of the
        // angle; `project`: v + dir · (v · dir) · (max(cos tilt, 0.05) − 1) with tilt clamped to 0..90).
        todo!()
    }
}
