//! [`Symmetry`]: a painting symmetry's straight axes and their geometry. No egui.

#![expect(clippy::todo, unused_variables, reason = "skeleton: removed when the bodies land (task TO-S1)")]

use kurbo::{Line, Point, Size, Vec2};

/// The largest centre coordinate a symmetry keeps, in document units (PhotoCraft's).
pub const MAX_COORDINATE: f64 = 1_000_000.0;
/// ⇧ turns the axes in steps of this many degrees, absolute from 0.
pub const TURN_STEP_DEGREES: f64 = 15.0;

/// The straight symmetry modes PhotoCraft has (lead call 23). Photoshop's Wavy, Circle, Spiral,
/// Parallel Lines, Radial and Mandala modes need engine work and are not built.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SymmetryMode {
    /// One vertical axis (90° before the rotation).
    Vertical,
    /// One horizontal axis (0°).
    Horizontal,
    /// A vertical and a horizontal axis: three mirrored copies.
    Dual,
    /// One diagonal axis (45°).
    Diagonal,
}

impl SymmetryMode {
    /// Every mode, in menu order.
    pub const ALL: [SymmetryMode; 4] = [SymmetryMode::Vertical, SymmetryMode::Horizontal, SymmetryMode::Dual, SymmetryMode::Diagonal];
}

/// A painting symmetry: its mode, the centre the axes run through (document units) and the
/// rotation in degrees clockwise (document y points down), 0 to 360.
// needs: lead, kurbo's `serde` feature for craft-draw (root change), then `serde::Serialize` and
// `serde::Deserialize` derives here (task TO-S1): brush presets store the symmetry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Symmetry {
    pub mode: SymmetryMode,
    pub center: Point,
    pub rotation: f64,
}

impl Symmetry {
    /// The symmetry, or `None` for a rotation or centre that is not finite or a centre coordinate
    /// beyond ±[`MAX_COORDINATE`]. The rotation wraps into 0 to 360.
    pub fn new(mode: SymmetryMode, center: Point, rotation: f64) -> Option<Symmetry> {
        // pc: crates/paint/src/symmetry.rs:41 `PresetSymmetry::new`.
        todo!()
    }

    /// The axes' unit directions: the first at 90° (Vertical, Dual), 0° (Horizontal) or 45°
    /// (Diagonal) plus the rotation, clockwise on screen; Dual adds the perpendicular.
    pub fn axes(&self) -> Vec<Vec2> {
        // pc: crates/paint/src/symmetry.rs:47 `directions`.
        todo!()
    }

    /// Each axis clipped to the document `size` (`(0, 0)` to `size`), whether or not the centre is
    /// inside it; an axis that misses the document gives no segment.
    pub fn segments(&self, size: Size) -> Vec<Line> {
        // pc: crates/paint/src/symmetry.rs:73 `segments`.
        todo!()
    }

    /// The rotation handle: along the first axis from the centre, at `share` of the document's
    /// shorter side (`symmetry.handle_share`, 0.35), at least 1 unit.
    pub fn rotation_handle(&self, size: Size, share: f64) -> Point {
        // pc: crates/ui-egui/src/tools/symmetry_ui.rs:227 `rotation_handle`.
        todo!()
    }
}
