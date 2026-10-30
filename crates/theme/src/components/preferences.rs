//! Preferences windows: the page list column and the gap between it and the page. Sizes only (no
//! colours): the look is the list, field and surface families'.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::values;

/// Preferences window tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PreferencesTokens {
    /// Width of the page list column: `preferences.list_width`.
    pub list_width: f32,
    /// Between the page list and the page: `preferences.page_gap`, `pad.panel`.
    pub page_gap: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PreferencesField {}

impl Family for PreferencesTokens {
    type Field = PreferencesField;
    const FIELDS: &'static [PreferencesField] = &[];

    fn defaults(s: &Semantic<'_>) -> PreferencesTokens {
        PreferencesTokens { list_width: values::PREFERENCES_LIST_WIDTH, page_gap: s.space.pad.panel }
    }

    fn path(f: PreferencesField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<PreferencesField> {
        None
    }

    fn kind(f: PreferencesField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: PreferencesField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: PreferencesField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
