//! Panel footers: the bar of buttons under a panel body. Sizes only (no colours): no theme overrides them.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::values;

/// Panel footer tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PanelFooterTokens {
    /// Height of the bar under the rule: `panel_footer.height`.
    pub height: f32,
    /// Between the buttons: `panel_footer.gap`.
    pub gap: f32,
    /// Side of a footer button: `panel_footer.button`.
    pub button: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PanelFooterField {}

impl Family for PanelFooterTokens {
    type Field = PanelFooterField;
    const FIELDS: &'static [PanelFooterField] = &[];

    /// The Compact sizes (PhotoCraft's footer: 26 point buttons 2 apart in a 30 point bar),
    /// scaled by the control height of the size and rounded to whole points.
    fn defaults(s: &Semantic<'_>) -> PanelFooterTokens {
        let k = s.size.control.md / values::PANEL_FOOTER_BASE_CONTROL;
        let scaled = |compact: f32| (compact * k).round();
        PanelFooterTokens {
            height: scaled(values::PANEL_FOOTER_HEIGHT),
            gap: scaled(values::PANEL_FOOTER_GAP),
            button: scaled(values::PANEL_FOOTER_BUTTON),
        }
    }

    fn path(f: PanelFooterField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<PanelFooterField> {
        None
    }

    fn kind(f: PanelFooterField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: PanelFooterField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: PanelFooterField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
