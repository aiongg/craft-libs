//! Panel headers: the title row of a flyout or floating panel. Sizes only (no colours): no theme overrides them.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::values;

/// Panel headers tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PanelHeaderTokens {
    /// Height of the header, from `size.panel_header`: `panel_header.height`.
    pub height: f32,
    /// Between the header edge and its content: `panel_header.pad`.
    pub pad: f32,
    /// Between the buttons: `panel_header.gap`.
    pub gap: f32,
    /// Side of the float, close and menu buttons: `panel_header.button`.
    pub button: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PanelHeaderField {}

impl Family for PanelHeaderTokens {
    type Field = PanelHeaderField;
    const FIELDS: &'static [PanelHeaderField] = &[];

    /// The height is the size preset's `panel_header`; the buttons scale from their Compact side with the control height.
    fn defaults(s: &Semantic<'_>) -> PanelHeaderTokens {
        let k = s.size.control.md / values::SECTION_BASE_CONTROL;
        PanelHeaderTokens {
            height: s.size.panel_header,
            pad: s.space.pad.control,
            gap: s.space.gap.inline,
            button: (values::PANEL_HEADER_BUTTON * k).round(),
        }
    }

    fn path(f: PanelHeaderField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<PanelHeaderField> {
        None
    }

    fn kind(f: PanelHeaderField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: PanelHeaderField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: PanelHeaderField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
