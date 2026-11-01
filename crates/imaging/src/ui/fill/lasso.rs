//! Content-Aware Fill's fill-area edits (task WS-CA1).

use serde::{Deserialize, Serialize};

use craft_ui::canvas::SelectionOperation;

/// One edit of the fill area, in order: a Lasso polygon with its operation, or Expand and
/// Contract by 1..=100 pixels. Serialised as PhotoCraft's `fillEdits` entries
/// (`{"op": "add", "points": […]}`, `{"op": "expand", "by": 4}`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum FillAreaEdit {
    New { points: Vec<[f64; 2]> },
    Add { points: Vec<[f64; 2]> },
    Subtract { points: Vec<[f64; 2]> },
    Intersect { points: Vec<[f64; 2]> },
    Expand { by: f64 },
    Contract { by: f64 },
}

impl FillAreaEdit {
    /// The Lasso's edit for a closed polygon with `operation`; `None` under 3 points.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-CA1")]
    pub fn lasso(operation: SelectionOperation, points: Vec<[f64; 2]>) -> Option<FillAreaEdit> {
        // pc: crates/ui-egui/src/dialogs/caf_ui/controls.rs:135-140 (3 points or more).
        todo!()
    }

    /// Expand or Contract by `by`, clamped to 1..=100.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-CA1")]
    pub fn grow(expand: bool, by: f64) -> FillAreaEdit {
        // pc: crates/ui-egui/src/dialogs/caf_ui/view.rs:62-68, controls.rs:74 (the clamp).
        todo!()
    }

    /// The points the edit carries, against `FillLimits::points`.
    pub fn points_len(&self) -> usize {
        match self {
            FillAreaEdit::New { points } | FillAreaEdit::Add { points } | FillAreaEdit::Subtract { points } | FillAreaEdit::Intersect { points } => {
                points.len()
            }
            FillAreaEdit::Expand { .. } | FillAreaEdit::Contract { .. } => 0,
        }
    }
}
