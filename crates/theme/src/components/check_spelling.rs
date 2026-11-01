//! Check Spelling dialog: the sizes of its fields and suggestion list.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// CheckSpelling tokens, in points, the same at every Size.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct CheckSpellingTokens {
    /// Width of the Change To field and the suggestion list, in points: `check_spelling.field_width`.
    pub field_width: f32,
    /// Height of the suggestion list, in points: `check_spelling.suggestions_height`.
    pub suggestions_height: f32,
}

/// CheckSpelling has no overridable field: its sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CheckSpellingField {}

impl Family for CheckSpellingTokens {
    type Field = CheckSpellingField;
    const FIELDS: &'static [CheckSpellingField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { CheckSpellingTokens;
        "field_width" 0.0..=1024.0 => field_width;
        "suggestions_height" 0.0..=1024.0 => suggestions_height;
    };

    fn defaults(_: &Semantic<'_>) -> CheckSpellingTokens {
        CheckSpellingTokens { field_width: values::CHECK_SPELLING_FIELD_WIDTH, suggestions_height: values::CHECK_SPELLING_SUGGESTIONS_HEIGHT }
    }

    fn path(f: CheckSpellingField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<CheckSpellingField> {
        None
    }

    fn kind(f: CheckSpellingField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: CheckSpellingField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: CheckSpellingField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
