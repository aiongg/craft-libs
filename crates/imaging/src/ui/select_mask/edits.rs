//! Select and Mask's edits and Refine Edge Brush strokes as typed data (task WS-SM1).

use serde::{Deserialize, Serialize};

/// Whether an edit adds to the selection or subtracts from it: the options bar's + and −; Alt at
/// the press gives the other.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EditMode {
    #[default]
    Add,
    Subtract,
}

/// What Object Selection picks: the object under a click (a drag shorter than 4 screen points), or
/// the one inside a dragged rect `[x, y, width, height]`. Serialised as PhotoCraft's `point` or
/// `rect` key.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ObjectTarget {
    Point([f64; 2]),
    Rect([f64; 4]),
}

/// One tool edit, in the order the engine applies them (`select.refineEdge`'s `edits`). Points are
/// document pixels rounded to a tenth. Serialised as PhotoCraft's JSON edits
/// (`{"tool": "quick", "points": …, "size": …, "mode": "add", "sampleAllLayers": false}`), so the
/// app passes [`SelectMaskCommand`](super::SelectMaskCommand) to its engine as it is.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "tool", rename_all = "camelCase")]
pub enum SelectMaskEdit {
    /// A Quick Selection stroke.
    Quick {
        points: Vec<[f64; 2]>,
        size: f64,
        mode: EditMode,
        #[serde(rename = "sampleAllLayers")]
        sample_all_layers: bool,
    },
    /// A Brush stroke.
    Brush { points: Vec<[f64; 2]>, size: f64, hardness: f64, mode: EditMode },
    /// A Lasso or Polygonal Lasso polygon (3 points or more).
    Lasso { points: Vec<[f64; 2]>, mode: EditMode },
    /// An Object Selection click or rect.
    Object {
        #[serde(flatten)]
        target: ObjectTarget,
        mode: EditMode,
        #[serde(rename = "sampleAllLayers")]
        sample_all_layers: bool,
    },
    /// Select Subject.
    Subject {
        #[serde(rename = "sampleAllLayers")]
        sample_all_layers: bool,
    },
    /// Clear Selection.
    Clear,
    /// Invert.
    Invert,
}

impl SelectMaskEdit {
    /// The points the edit carries, against `SelectMaskLimits::edit_points` (an Object edit and
    /// the buttons carry none).
    #[expect(clippy::todo, reason = "skeleton: task WS-SM1")]
    pub fn points_len(&self) -> usize {
        // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:510 `points_in`.
        todo!()
    }
}

/// A Refine Edge Brush stroke: kept apart from the edits (`select.refineEdge`'s `refineStrokes`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefineStroke {
    pub points: Vec<[f64; 2]>,
    pub size: f64,
    pub mode: EditMode,
}

/// A point rounded to a tenth of a pixel, as edits store them.
pub fn round_point(at: [f64; 2]) -> [f64; 2] {
    // pc: crates/ui-egui/src/dialogs/select_mask_ui.rs:555 `round`.
    [(at[0] * 10.0).round() / 10.0, (at[1] * 10.0).round() / 10.0]
}
