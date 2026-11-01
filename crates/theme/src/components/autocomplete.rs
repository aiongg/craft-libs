//! Autocomplete lists: the popup under an editor. Sizes only: the colours are the surface, text and state tokens of the controls inside, and no theme overrides these.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Autocomplete tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct AutocompleteTokens {
    /// Least width of the list: `autocomplete.min_width`.
    pub min_width: f32,
    /// Between the editor's bottom edge and the list: `autocomplete.gap`.
    pub gap: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AutocompleteField {}

impl Family for AutocompleteTokens {
    type Field = AutocompleteField;
    const FIELDS: &'static [AutocompleteField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { AutocompleteTokens;
        "min_width" 0.0..=1024.0 => min_width;
        "gap" 0.0..=64.0 => gap;
    };

    fn defaults(_: &Semantic<'_>) -> AutocompleteTokens {
        AutocompleteTokens { min_width: values::AUTOCOMPLETE_MIN_WIDTH, gap: values::AUTOCOMPLETE_GAP }
    }

    fn path(f: AutocompleteField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<AutocompleteField> {
        None
    }

    fn kind(f: AutocompleteField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: AutocompleteField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: AutocompleteField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
