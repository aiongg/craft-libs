//! Status bars: the bar at the bottom of the window. Sizes only (no colours): the bar draws in
//! `surface.chrome`, its info text in `text.muted`, its message in `text.faint` and an error in
//! `status.warn.fg`. The height is `size.bar.status`.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::values;

/// Status bar tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct StatusBarTokens {
    /// Height of the bar: `status_bar.height` (`size.bar.status`).
    pub height: f32,
    /// Space at both ends of the bar: `status_bar.pad`.
    pub pad: f32,
    /// Between the items of a group: `status_bar.gap`.
    pub gap: f32,
    /// Added after the zoom field, before the info text: `status_bar.group_gap`.
    pub group_gap: f32,
    /// Length of a divider: `status_bar.divider`.
    pub divider: f32,
    /// Width of the zoom field's cell: `status_bar.field_width`.
    pub field_width: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum StatusBarField {}

impl Family for StatusBarTokens {
    type Field = StatusBarField;
    const FIELDS: &'static [StatusBarField] = &[];

    /// The Compact sizes (PhotoCraft's Pro status bar), scaled by the control height of the size
    /// and rounded to whole points (halves to even). The ends keep their width.
    fn defaults(s: &Semantic<'_>) -> StatusBarTokens {
        let k = s.size.control.md / values::OPTIONS_BAR_BASE_CONTROL;
        let scale = |w: f32| (w * k).round_ties_even();
        StatusBarTokens {
            height: s.size.bar.status,
            pad: values::STATUS_BAR_PAD,
            gap: scale(values::STATUS_BAR_GAP),
            group_gap: scale(values::STATUS_BAR_GROUP_GAP),
            divider: scale(values::STATUS_BAR_DIVIDER),
            field_width: values::status_bar_field_width(s.preset),
        }
    }

    fn path(f: StatusBarField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<StatusBarField> {
        None
    }

    fn kind(f: StatusBarField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: StatusBarField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: StatusBarField, _: OverrideValue) {
        match f {}
    }

    // The text is a semantic text colour, which the text rules hold on `surface.chrome`.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
