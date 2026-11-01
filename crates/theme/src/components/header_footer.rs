//! The header and footer zones. No theme overrides it.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// HeaderFooter tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct HeaderFooterTokens {
    /// A section's inset from its band, top and bottom: [`values::HEADER_FOOTER_INSET`].
    pub inset: f32,
    /// The dashed outline of a hovered empty section: [`values::HEADER_FOOTER_HINT_OUTLINE`].
    pub hint_outline: f32,
    /// The edited section's outline and its neighbours': [`values::HEADER_FOOTER_EDIT_OUTLINE`].
    pub edit_outline: f32,
}

/// HeaderFooter tokens have no overridable field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HeaderFooterField {}

impl Family for HeaderFooterTokens {
    type Field = HeaderFooterField;
    const FIELDS: &'static [HeaderFooterField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { HeaderFooterTokens;
        "inset" 0.0..=32.0 => inset;
        "hint_outline" 0.0..=8.0 => hint_outline;
        "edit_outline" 0.0..=8.0 => edit_outline;
    };

    fn defaults(_: &Semantic<'_>) -> HeaderFooterTokens {
        HeaderFooterTokens {
            inset: values::HEADER_FOOTER_INSET,
            hint_outline: values::HEADER_FOOTER_HINT_OUTLINE,
            edit_outline: values::HEADER_FOOTER_EDIT_OUTLINE,
        }
    }

    fn path(f: HeaderFooterField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<HeaderFooterField> {
        None
    }

    fn kind(f: HeaderFooterField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: HeaderFooterField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: HeaderFooterField, _: OverrideValue) {
        match f {}
    }

    // Marks and tints drawn over surfaces other families check.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
