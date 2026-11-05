//! Content-Aware Fill's fill-area edits.

use serde::{Deserialize, Serialize};

use craft_ui::canvas::SelectionOperation;

/// One edit of the fill area, in order: a Lasso polygon with its operation, or Expand and
/// Contract by 1..=100 pixels. Serialised as PhotoCraft's `fillEdits` entries
/// (`{"op": "add", "points": […]}`, `{"op": "expand", "by": 4}`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum FillAreaEdit {
    /// Replaces the fill area with the polygon `points`.
    New {
        #[allow(missing_docs)]
        points: Vec<[f64; 2]>,
    },
    /// Adds the polygon `points` to the fill area.
    Add {
        #[allow(missing_docs)]
        points: Vec<[f64; 2]>,
    },
    /// Subtracts the polygon `points` from the fill area.
    Subtract {
        #[allow(missing_docs)]
        points: Vec<[f64; 2]>,
    },
    /// Intersects the fill area with the polygon `points`.
    Intersect {
        #[allow(missing_docs)]
        points: Vec<[f64; 2]>,
    },
    /// Grows the fill area by `by` pixels.
    Expand {
        #[allow(missing_docs)]
        by: f64,
    },
    /// Shrinks the fill area by `by` pixels.
    Contract {
        #[allow(missing_docs)]
        by: f64,
    },
}

impl FillAreaEdit {
    /// A polygon needs this many points.
    const LASSO_MIN_POINTS: usize = 3;
    /// Expand and Contract's range in pixels.
    pub const GROW_RANGE: (f64, f64) = (1.0, 100.0);

    /// The Lasso's edit for a closed polygon with `operation`; `None` under 3 points.
    pub fn lasso(operation: SelectionOperation, points: Vec<[f64; 2]>) -> Option<FillAreaEdit> {
        // pc: crates/ui-egui/src/dialogs/caf_ui/controls.rs:118-141 (3 points or more; the ops
        // are caf_ui.rs:96 `LASSO_OPS`).
        if points.len() < Self::LASSO_MIN_POINTS {
            return None;
        }
        Some(match operation {
            SelectionOperation::New => FillAreaEdit::New { points },
            SelectionOperation::Add => FillAreaEdit::Add { points },
            SelectionOperation::Subtract => FillAreaEdit::Subtract { points },
            SelectionOperation::Intersect => FillAreaEdit::Intersect { points },
        })
    }

    /// Expand or Contract by `by`, clamped to 1..=100.
    pub fn grow(expand: bool, by: f64) -> FillAreaEdit {
        // pc: crates/ui-egui/src/dialogs/caf_ui/view.rs:62-69 (the buttons and the field,
        // range 1..=100), controls.rs:72-75 (the control channel's clamp). A non-finite amount
        // falls to the least.
        let by = if by.is_finite() { by.clamp(Self::GROW_RANGE.0, Self::GROW_RANGE.1) } else { Self::GROW_RANGE.0 };
        if expand { FillAreaEdit::Expand { by } } else { FillAreaEdit::Contract { by } }
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
