//! Findings lists: the status dot of the summary line. Sizes only (no colours): the colours are the surface, text and state tokens of the controls
//! inside, and no theme overrides these. The values are PhotoCraft's.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Findings tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct FindingsTokens {
    /// Radius of the summary's status dot: `findings.status_dot`.
    pub status_dot: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FindingsField {}

impl Family for FindingsTokens {
    type Field = FindingsField;
    const FIELDS: &'static [FindingsField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { FindingsTokens;
        "status_dot" 0.0..=32.0 => status_dot;
    };

    fn defaults(_: &Semantic<'_>) -> FindingsTokens {
        FindingsTokens { status_dot: values::FINDINGS_STATUS_DOT }
    }

    fn path(f: FindingsField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<FindingsField> {
        None
    }

    fn kind(f: FindingsField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: FindingsField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: FindingsField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
