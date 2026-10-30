//! Empty states: the message of an empty panel, list or canvas. The family holds the width of a
//! centred column and the text colour of the inline message.

use super::{ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::color::Rgba;
use crate::tokens::{ColorToken, FontSlot, TokenRef, TypeStyle};
use crate::values;

/// Empty state tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct EmptyStateTokens {
    /// Widest the column of a centred empty state grows: `empty_state.max_width`.
    pub max_width: f32,
    /// The inline message and the brand byline: `text.faint`.
    pub text: Rgba,
    /// Least width of each action of a centred empty state: `empty_state.action_min_width`.
    pub action_min_width: f32,
    /// The brand title of a centred empty state, the app's name: `empty_state.brand_title`.
    pub brand_title: TypeStyle,
    /// The brand byline beside or under the brand title: `empty_state.brand_byline`.
    pub brand_byline: TypeStyle,
    /// Between the brand title and its byline: `empty_state.brand_gap`.
    pub brand_gap: f32,
    /// How far the byline's bottom edge sits above the brand title's: `empty_state.brand_byline_lift`.
    pub brand_byline_lift: f32,
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
        EmptyStateTokens {
            max_width: values::EMPTY_STATE_MAX_WIDTH,
            text: s.color.text.faint,
            action_min_width: values::EMPTY_STATE_ACTION_MIN_WIDTH,
            brand_title: TypeStyle {
                size: values::EMPTY_STATE_BRAND_TITLE_SIZE,
                line_height: values::EMPTY_STATE_BRAND_TITLE_LINE_HEIGHT,
                font: FontSlot::UiSemibold,
            },
            brand_byline: TypeStyle {
                size: values::EMPTY_STATE_BRAND_BYLINE_SIZE,
                line_height: values::EMPTY_STATE_BRAND_BYLINE_LINE_HEIGHT,
                font: FontSlot::Ui,
            },
            brand_gap: values::EMPTY_STATE_BRAND_GAP,
            brand_byline_lift: values::EMPTY_STATE_BRAND_BYLINE_LIFT,
        }
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
            fg: TokenRef::Component(ComponentField::EmptyState(EmptyStateField::Text)),
            bg: TokenRef::Color(ColorToken::SurfacePanel),
            minimum: values::TEXT_MIN,
            alternate: None,
        }]
    }
}
