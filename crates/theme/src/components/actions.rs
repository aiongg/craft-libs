//! The Actions panel: row heights, the recording dot's radius and the six Button Mode fills.
//! Rows, selection fills and the footer are the list and panel footer families'. No theme
//! overrides it.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
use crate::values;

/// Actions tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ActionsTokens {
    /// A set header's height: `values::ACTIONS_SET_ROW_HEIGHT`.
    pub set_row_height: f32,
    /// An action row's height: `values::ACTIONS_ROW_HEIGHT`.
    pub row_height: f32,
    /// A step row's height: `values::ACTIONS_STEP_ROW_HEIGHT`.
    pub step_row_height: f32,
    /// The radius of the dot on the action being recorded:
    /// `values::ACTIONS_RECORDING_DOT_RADIUS`.
    pub recording_dot_radius: f32,
    /// The Button Mode fills, red to violet: `actions.color.red` to `actions.color.violet`
    /// (`values::ACTIONS_COLORS`). Data colours, the same in every theme (as the list's label
    /// colours); gray and no colour derive from `text.muted` and `surface.raised`.
    pub colors: [Rgba; 6],
    /// A label on a coloured fill: `actions.color.label` (`values::ACTIONS_LABEL_ON_COLOR`),
    /// near black over all six fills.
    pub label_on_color: Rgba,
}

/// Actions have no overridable field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ActionsField {}

impl Family for ActionsTokens {
    type Field = ActionsField;
    const FIELDS: &'static [ActionsField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { ActionsTokens;
        "set_row_height" 12.0..=64.0 => set_row_height;
        "row_height" 12.0..=64.0 => row_height;
        "step_row_height" 12.0..=64.0 => step_row_height;
        "recording_dot_radius" 1.0..=12.0 => recording_dot_radius;
    };

    fn defaults(_: &Semantic<'_>) -> ActionsTokens {
        ActionsTokens {
            set_row_height: values::ACTIONS_SET_ROW_HEIGHT,
            row_height: values::ACTIONS_ROW_HEIGHT,
            step_row_height: values::ACTIONS_STEP_ROW_HEIGHT,
            recording_dot_radius: values::ACTIONS_RECORDING_DOT_RADIUS,
            colors: values::ACTIONS_COLORS,
            label_on_color: values::ACTIONS_LABEL_ON_COLOR,
        }
    }

    fn path(f: ActionsField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<ActionsField> {
        None
    }

    fn kind(f: ActionsField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: ActionsField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: ActionsField, _: OverrideValue) {
        match f {}
    }

    // The fills are fixed data and the labels on them are fixed near black.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
