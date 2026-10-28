//! Tooltips. A tooltip is drawn on the popup surface in the body type role, by egui's own tooltip
//! machinery, which takes its frame, shadow, border, margin and text from the generated egui
//! `Style` shared with popups and menus. The family holds only what the style has a field for
//! beyond that: the widest a tooltip's text grows. No theme overrides it.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::values;

/// Tooltip tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TooltipTokens {
    /// Widest the text grows before it wraps.
    pub max_width: f32,
}

/// Tooltips have no overridable field: stock egui tooltips share the popup style, so a tooltip
/// colour or radius of its own could not reach them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TooltipField {}

impl Family for TooltipTokens {
    type Field = TooltipField;
    const FIELDS: &'static [TooltipField] = &[];

    fn defaults(_: &Semantic<'_>) -> TooltipTokens {
        TooltipTokens { max_width: values::TOOLTIP_WIDTH }
    }

    fn path(f: TooltipField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<TooltipField> {
        None
    }

    fn kind(f: TooltipField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: TooltipField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: TooltipField, _: OverrideValue) {
        match f {}
    }

    // The text is `text.normal` on `surface.raised`, which the semantic pass already requires.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
