//! Password prompts: the width of the dialog. Sizes only: the colours are the surface, text and state tokens of the controls inside, and no theme overrides these.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Password tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct PasswordTokens {
    /// Width of the dialog: `password.width`.
    pub width: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PasswordField {}

impl Family for PasswordTokens {
    type Field = PasswordField;
    const FIELDS: &'static [PasswordField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { PasswordTokens;
        "width" 0.0..=4096.0 => width;
    };

    fn defaults(_: &Semantic<'_>) -> PasswordTokens {
        PasswordTokens { width: values::PASSWORD_WIDTH }
    }

    fn path(f: PasswordField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<PasswordField> {
        None
    }

    fn kind(f: PasswordField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: PasswordField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: PasswordField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
