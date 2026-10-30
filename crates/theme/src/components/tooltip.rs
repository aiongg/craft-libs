//! Tooltips. A tooltip is a popup-surface card in the body type role. The family holds the widest
//! its text grows and the card's corner and colours, which default to the popup's so a theme sets
//! them only to give tooltips a look of their own (Classic: black text on a pale yellow card with
//! a black edge). The card's margin and shadow are the popup's.

use super::{ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::color::Rgba;
use crate::tokens::TokenRef;
use crate::values;

/// Tooltip tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TooltipTokens {
    /// Widest the text grows before it wraps.
    pub max_width: f32,
    /// Distance between the pointer and a pointer-anchored tooltip: `tooltip.pointer_gap`.
    pub pointer_gap: f32,
    /// Corner radius: `shape.radius.popup`.
    pub radius: f32,
    /// Card fill: `surface.panel`, the card of PhotoCraft's windows.
    pub fill: Rgba,
    /// Text on the card: `text.normal`.
    pub text: Rgba,
    /// The one-point outline: `line.border`.
    pub border: Rgba,
}

/// An overridable tooltip field. Paths: `radius`, `fill`, `text`, `border`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TooltipField {
    /// `tooltip.radius`
    Radius,
    /// `tooltip.fill`
    Fill,
    /// `tooltip.text`
    Text,
    /// `tooltip.border`
    Border,
}

impl Family for TooltipTokens {
    type Field = TooltipField;
    const FIELDS: &'static [TooltipField] = &[TooltipField::Radius, TooltipField::Fill, TooltipField::Text, TooltipField::Border];

    fn defaults(s: &Semantic<'_>) -> TooltipTokens {
        let c = s.color;
        TooltipTokens {
            max_width: values::TOOLTIP_WIDTH,
            pointer_gap: values::TOOLTIP_POINTER_GAP,
            radius: s.shape.radius.popup,
            fill: c.surface.panel,
            text: c.text.normal,
            border: c.line.border,
        }
    }

    fn path(f: TooltipField) -> String {
        match f {
            TooltipField::Radius => "radius",
            TooltipField::Fill => "fill",
            TooltipField::Text => "text",
            TooltipField::Border => "border",
        }
        .to_owned()
    }

    fn from_path(path: &str) -> Option<TooltipField> {
        Some(match path {
            "radius" => TooltipField::Radius,
            "fill" => TooltipField::Fill,
            "text" => TooltipField::Text,
            "border" => TooltipField::Border,
            _ => return None,
        })
    }

    fn kind(f: TooltipField) -> FieldKind {
        match f {
            TooltipField::Radius => FieldKind::Length,
            _ => FieldKind::Color,
        }
    }

    fn get(&self, f: TooltipField) -> OverrideValue {
        match f {
            TooltipField::Radius => OverrideValue::Length(self.radius),
            TooltipField::Fill => OverrideValue::Color(self.fill),
            TooltipField::Text => OverrideValue::Color(self.text),
            TooltipField::Border => OverrideValue::Color(self.border),
        }
    }

    fn set(&mut self, f: TooltipField, v: OverrideValue) {
        match (f, v) {
            (TooltipField::Radius, OverrideValue::Length(r)) => self.radius = r,
            (TooltipField::Fill, OverrideValue::Color(c)) => self.fill = c,
            (TooltipField::Text, OverrideValue::Color(c)) => self.text = c,
            (TooltipField::Border, OverrideValue::Color(c)) => self.border = c,
            // Mismatched kinds are rejected before this is called.
            _ => {}
        }
    }

    /// The text on the card fill. The border is a design target, not a rule.
    fn pairs() -> Vec<ContrastPair> {
        let token = |f| TokenRef::Component(ComponentField::Tooltip(f));
        vec![ContrastPair { fg: token(TooltipField::Text), bg: token(TooltipField::Fill), minimum: values::TEXT_MIN, alternate: None }]
    }
}
