//! Empty states: the message of an empty panel, list or canvas. The family holds the width of a
//! centred column and the text colour of the inline message.

use super::{ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::color::Rgba;
use crate::tokens::{ColorToken, TokenRef};
use crate::values;

/// Empty state tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct EmptyStateTokens {
    /// Widest the column of a centred empty state grows: `empty_state.max_width`.
    pub max_width: f32,
    /// The inline message: `text.faint`.
    pub text: Rgba,
}

/// An overridable empty state field. Path: `text`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EmptyStateField {
    /// `empty_state.text`
    Text,
}

impl Family for EmptyStateTokens {
    type Field = EmptyStateField;
    const FIELDS: &'static [EmptyStateField] = &[EmptyStateField::Text];

    fn defaults(s: &Semantic<'_>) -> EmptyStateTokens {
        EmptyStateTokens { max_width: values::EMPTY_STATE_MAX_WIDTH, text: s.color.text.faint }
    }

    fn path(_: EmptyStateField) -> String {
        "text".to_owned()
    }

    fn from_path(path: &str) -> Option<EmptyStateField> {
        (path == "text").then_some(EmptyStateField::Text)
    }

    fn kind(_: EmptyStateField) -> FieldKind {
        FieldKind::Color
    }

    fn get(&self, _: EmptyStateField) -> OverrideValue {
        OverrideValue::Color(self.text)
    }

    fn set(&mut self, _: EmptyStateField, v: OverrideValue) {
        if let OverrideValue::Color(c) = v {
            self.text = c;
        }
    }

    fn pairs() -> Vec<ContrastPair> {
        vec![ContrastPair {
            fg: ComponentField::EmptyState(EmptyStateField::Text),
            bg: TokenRef::Color(ColorToken::SurfacePanel),
            minimum: values::TEXT_MIN,
        }]
    }
}
