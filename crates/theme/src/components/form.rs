//! Form grids: the two-column grid of a dialog or a preferences page, labels in the first column
//! and controls in the second. Sizes only (no colours): they scale with the control height, and
//! no theme overrides them. The label's look is the `field` family's.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// The gaps and label column of one density of form grid, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct FormSpacing {
    /// Between the label column and the control column: `form.column_gap`.
    pub column_gap: f32,
    /// Between two rows: `form.row_gap`.
    pub row_gap: f32,
    /// The least width of the label column; a wider label widens it: `form.label_min`.
    pub label_min: f32,
}

/// Form grid tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct FormTokens {
    /// A dialog's form: PhotoCraft's `Grid::spacing([12.0, 6.0])`.
    pub normal: FormSpacing,
    /// A preferences page's form: PhotoCraft's `Grid::spacing([14.0, 7.0])` with a label column
    /// of at least 150 points. Paths `form.roomy.*`.
    pub roomy: FormSpacing,
}

/// Form grids have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FormField {}

impl Family for FormTokens {
    type Field = FormField;
    const FIELDS: &'static [FormField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { FormTokens;
        "normal.column_gap" 0.0..=256.0 => normal.column_gap;
        "normal.row_gap" 0.0..=256.0 => normal.row_gap;
        "normal.label_min" 0.0..=256.0 => normal.label_min;
        "roomy.column_gap" 0.0..=256.0 => roomy.column_gap;
        "roomy.row_gap" 0.0..=256.0 => roomy.row_gap;
        "roomy.label_min" 0.0..=256.0 => roomy.label_min;
    };

    /// The Compact values (PhotoCraft's), scaled by the control height of the size and rounded
    /// to whole points.
    fn defaults(s: &Semantic<'_>) -> FormTokens {
        let k = s.size.control.md / values::FORM_BASE_CONTROL;
        let scale = |w: f32| (w * k).round();
        FormTokens {
            normal: FormSpacing {
                column_gap: scale(values::FORM_COLUMN_GAP),
                row_gap: scale(values::FORM_ROW_GAP),
                label_min: scale(values::FORM_LABEL_MIN),
            },
            roomy: FormSpacing {
                column_gap: scale(values::FORM_ROOMY_COLUMN_GAP),
                row_gap: scale(values::FORM_ROOMY_ROW_GAP),
                label_min: scale(values::FORM_ROOMY_LABEL_MIN),
            },
        }
    }

    fn path(f: FormField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<FormField> {
        None
    }

    fn kind(f: FormField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: FormField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: FormField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
