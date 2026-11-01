//! The marks find paints on a page: every match and the current one. No theme overrides it.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
use crate::values;

/// Find tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct FindTokens {
    /// Corner radius of the current match: [`values::FIND_CURRENT_RADIUS`].
    pub current_radius: f32,
    /// How much the current match grows past the others: [`values::FIND_CURRENT_GROW`].
    pub current_grow: f32,
    /// A match's tint (`find.match`): `find.match`, `data.yellow` at alpha [`values::FIND_MATCHED_ALPHA`].
    pub matched: Rgba,
    /// The current match's tint: `find.current`, `status.warn.solid` at alpha [`values::FIND_CURRENT_ALPHA`].
    pub current: Rgba,
}

/// Find tokens have no overridable field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FindField {}

impl Family for FindTokens {
    type Field = FindField;
    const FIELDS: &'static [FindField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { FindTokens;
        "current_radius" 0.0..=16.0 => current_radius;
        "current_grow" 0.0..=16.0 => current_grow;
    };

    fn defaults(s: &Semantic<'_>) -> FindTokens {
        let c = s.color;
        FindTokens {
            current_radius: values::FIND_CURRENT_RADIUS,
            current_grow: values::FIND_CURRENT_GROW,
            matched: c.data.yellow.with_alpha(values::FIND_MATCHED_ALPHA),
            current: c.status.warn.solid.with_alpha(values::FIND_CURRENT_ALPHA),
        }
    }

    fn path(f: FindField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<FindField> {
        None
    }

    fn kind(f: FindField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: FindField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: FindField, _: OverrideValue) {
        match f {}
    }

    // Marks and tints drawn over surfaces other families check.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
