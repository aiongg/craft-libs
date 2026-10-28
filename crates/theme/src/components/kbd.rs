//! Key chords. Plain, a chord is muted text; as keycaps, each key sits in a small raised box that
//! takes its edge from the `Surface` primitive. This family holds the keycap's size, corner and
//! colours.

use super::{ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::color::Rgba;
use crate::tokens::{TokenRef, TypeRole};
use crate::values;

/// Kbd tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct KbdTokens {
    /// Height of a keycap: the keys' line and [`pad_y`](KbdTokens::pad_y) above and below it, so
    /// it grows with the Text size setting.
    pub height: f32,
    /// Above and below a keycap's line.
    pub pad_y: f32,
    /// At the start and end of a keycap's text: `space.gap.inline`. Half of it separates keycaps.
    pub pad_x: f32,
    /// Type role of the keys: Caption, in the interface face. The macOS modifier symbols are not
    /// in the monospace face, and a chord needs no equal widths.
    pub label: TypeRole,
    /// Corner radius of a keycap: `shape.radius.control` or the theme's `kbd.radius`, at most
    /// [`values::KBD_RADIUS_MAX`].
    pub radius: f32,
    /// Keycap fill: `surface.raised`.
    pub fill: Rgba,
    /// Keys, plain or on a keycap: `text.muted`.
    pub text: Rgba,
}

/// An overridable kbd field. Paths: `radius`, `fill`, `text`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum KbdField {
    /// `kbd.radius`
    Radius,
    /// `kbd.fill`
    Fill,
    /// `kbd.text`
    Text,
}

impl Family for KbdTokens {
    type Field = KbdField;
    const FIELDS: &'static [KbdField] = &[KbdField::Radius, KbdField::Fill, KbdField::Text];

    fn defaults(s: &Semantic<'_>) -> KbdTokens {
        let label = TypeRole::Caption;
        KbdTokens {
            height: s.typography.get(label).line_height + 2.0 * values::KBD_PAD_Y,
            pad_y: values::KBD_PAD_Y,
            pad_x: s.space.gap.inline,
            label,
            radius: s.shape.radius.control.min(values::KBD_RADIUS_MAX),
            fill: s.color.surface.raised,
            text: s.color.text.muted,
        }
    }

    fn path(f: KbdField) -> String {
        match f {
            KbdField::Radius => "radius",
            KbdField::Fill => "fill",
            KbdField::Text => "text",
        }
        .to_owned()
    }

    fn from_path(path: &str) -> Option<KbdField> {
        Some(match path {
            "radius" => KbdField::Radius,
            "fill" => KbdField::Fill,
            "text" => KbdField::Text,
            _ => return None,
        })
    }

    fn kind(f: KbdField) -> FieldKind {
        match f {
            KbdField::Radius => FieldKind::Length,
            _ => FieldKind::Color,
        }
    }

    fn get(&self, f: KbdField) -> OverrideValue {
        match f {
            KbdField::Radius => OverrideValue::Length(self.radius),
            KbdField::Fill => OverrideValue::Color(self.fill),
            KbdField::Text => OverrideValue::Color(self.text),
        }
    }

    fn set(&mut self, f: KbdField, v: OverrideValue) {
        match (f, v) {
            (KbdField::Radius, OverrideValue::Length(r)) => self.radius = r.min(values::KBD_RADIUS_MAX),
            (KbdField::Fill, OverrideValue::Color(c)) => self.fill = c,
            (KbdField::Text, OverrideValue::Color(c)) => self.text = c,
            // Mismatched kinds are rejected before this is called.
            _ => {}
        }
    }

    /// The keys are text on the keycap.
    fn pairs() -> Vec<ContrastPair> {
        vec![ContrastPair {
            fg: TokenRef::Component(ComponentField::Kbd(KbdField::Text)),
            bg: TokenRef::Component(ComponentField::Kbd(KbdField::Fill)),
            minimum: values::TEXT_MIN,
        }]
    }
}
