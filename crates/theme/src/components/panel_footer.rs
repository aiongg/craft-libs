//! Panel footers: the bar of buttons under a panel body. Sizes only (no colours): no theme overrides them.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic};

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

    /// The buttons are 2 larger than a medium control (26 at Compact) and 2 apart (the inline gap less 2); the bar has 2 above and below them.
    fn defaults(s: &Semantic<'_>) -> PanelFooterTokens {
        let button = s.size.control.md + 2.0;
        PanelFooterTokens { height: button + 4.0, gap: s.space.gap.inline - 2.0, button }
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
