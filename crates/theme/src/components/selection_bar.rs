//! Selection bars: the floating bar of actions beside a selection. The family holds sizes only; the colours are semantic tokens.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// SelectionBar tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SelectionBarTokens {
    /// The bar's height: `selection_bar.height`.
    pub height: f32,
    /// Between the selection and the bar when it sits below: `selection_bar.gap_below`.
    pub gap_below: f32,
    /// Between the selection and the bar when it sits above: `selection_bar.gap_above`.
    pub gap_above: f32,
    /// Kept between the bar and the canvas edge: `selection_bar.inset`.
    pub inset: f32,
    /// The width assumed before the bar has been measured: `selection_bar.first_width`.
    pub first_width: f32,
    /// Width of the grip box: `selection_bar.grip_width`.
    pub grip_width: f32,
    /// Height of the grip box: `selection_bar.grip_height`.
    pub grip_height: f32,
    /// Radius of a grip dot: `selection_bar.grip_dot`.
    pub grip_dot: f32,
    /// Distance between grip dots: `selection_bar.grip_pitch`.
    pub grip_pitch: f32,
}

/// SelectionBar tokens have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SelectionBarField {}

impl Family for SelectionBarTokens {
    type Field = SelectionBarField;
    const FIELDS: &'static [SelectionBarField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { SelectionBarTokens;
        "height" 0.0..=1024.0 => height;
        "gap_below" 0.0..=1024.0 => gap_below;
        "gap_above" 0.0..=1024.0 => gap_above;
        "inset" 0.0..=1024.0 => inset;
        "first_width" 0.0..=1024.0 => first_width;
        "grip_width" 0.0..=1024.0 => grip_width;
        "grip_height" 0.0..=1024.0 => grip_height;
        "grip_dot" 0.0..=1024.0 => grip_dot;
        "grip_pitch" 0.0..=1024.0 => grip_pitch;
    };

    fn defaults(_: &Semantic<'_>) -> SelectionBarTokens {
        SelectionBarTokens {
            height: values::SELECTION_BAR_HEIGHT,
            gap_below: values::SELECTION_BAR_GAP_BELOW,
            gap_above: values::SELECTION_BAR_GAP_ABOVE,
            inset: values::SELECTION_BAR_INSET,
            first_width: values::SELECTION_BAR_FIRST_WIDTH,
            grip_width: values::SELECTION_BAR_GRIP_WIDTH,
            grip_height: values::SELECTION_BAR_GRIP_HEIGHT,
            grip_dot: values::SELECTION_BAR_GRIP_DOT,
            grip_pitch: values::SELECTION_BAR_GRIP_PITCH,
        }
    }

    fn path(f: SelectionBarField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<SelectionBarField> {
        None
    }

    fn kind(f: SelectionBarField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: SelectionBarField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: SelectionBarField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
