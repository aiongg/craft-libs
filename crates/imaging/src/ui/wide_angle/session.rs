//! Adaptive Wide Angle's session: the correction and the constraints (task WS-WA1).

use serde::{Deserialize, Serialize};

use craft_ui::canvas::kurbo::Point;

/// Correction: the lens model.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WideAngleModel {
    #[default]
    Auto,
    Fisheye,
    Perspective,
    FullSpherical,
}

impl WideAngleModel {
    pub const ALL: [WideAngleModel; 4] = [WideAngleModel::Auto, WideAngleModel::Fisheye, WideAngleModel::Perspective, WideAngleModel::FullSpherical];
}

/// A constraint's orientation: Free (a straight line), Horizontal or Vertical.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConstraintOrientation {
    #[default]
    Free,
    Horizontal,
    Vertical,
}

impl ConstraintOrientation {
    pub const ALL: [ConstraintOrientation; 3] = [ConstraintOrientation::Free, ConstraintOrientation::Horizontal, ConstraintOrientation::Vertical];
}

/// A constraint: the end points of a line in the scene, in document pixels. Serialised as
/// PhotoCraft's `Constraint` (`a`, `b`, `orientation`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WideAngleConstraint {
    pub a: [f64; 2],
    pub b: [f64; 2],
    #[serde(default)]
    pub orientation: ConstraintOrientation,
}

impl WideAngleConstraint {
    /// A drag from `a` to `b` with Shift held at the release or not: Shift makes it Horizontal or
    /// Vertical by the larger of the two deltas (Horizontal on a tie), else Free.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-WA1")]
    pub fn from_drag(a: Point, b: Point, shift: bool) -> WideAngleConstraint {
        // pc: crates/ui-egui/src/dialogs/wide_angle_ui.rs:300-305.
        todo!()
    }
}

/// A change of the constraints.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum WideAngleConstraintEdit {
    Add(WideAngleConstraint),
    Orient {
        index: usize,
        orientation: ConstraintOrientation,
    },
    Delete(usize),
    /// Arrow keys moved the focused constraint by `by` document pixels.
    Move {
        index: usize,
        by: [f64; 2],
    },
}

/// What OK applies (`filter.adaptiveWideAngle`'s parameters, serialised as PhotoCraft's
/// `WideAngle`) and the history snapshots. Defaults: Auto, focal length 0 (from the file's
/// metadata), crop factor 1, scale 100, no constraints. The Preview check is view state outside
/// it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WideAngleSession {
    pub model: WideAngleModel,
    /// 0..=200 mm (35 mm frame after the crop factor).
    pub focal_length: f64,
    /// 0.1..=10.
    pub crop_factor: f64,
    /// 50..=150 %.
    pub scale: f64,
    pub constraints: Vec<WideAngleConstraint>,
}

impl Default for WideAngleSession {
    fn default() -> Self {
        // pc: crates/algo/src/wideangle.rs:83-87.
        Self { model: WideAngleModel::Auto, focal_length: 0.0, crop_factor: 1.0, scale: 100.0, constraints: Vec::new() }
    }
}

impl WideAngleSession {
    /// Applies `edit`; an index out of range changes nothing. Returns whether it changed.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-WA1")]
    pub fn apply(&mut self, edit: WideAngleConstraintEdit) -> bool {
        // pc: crates/ui-egui/src/dialogs/wide_angle_ui.rs:232 `constraint_menu`, :402-425.
        todo!()
    }
}

/// The app's lens geometry (lead call 44): the solver stays the app's (`wideangle::solve`).
pub trait WideAngleGeometry {
    /// The curve the scene line from `a` to `b` makes on the source image under `session`'s lens,
    /// as `segments + 1` document points (PhotoCraft: `Camera::arc(a, b, 24)`).
    fn arc(&self, session: &WideAngleSession, a: [f64; 2], b: [f64; 2], segments: usize) -> Vec<Point>;

    /// Where a source point lands in the corrected preview (the solved mesh's map), or `None`
    /// while no solve is ready.
    fn corrected(&self, session: &WideAngleSession, at: Point) -> Option<Point>;
}
