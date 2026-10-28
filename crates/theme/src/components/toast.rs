//! Toasts: a stack of cards in a corner of the window. The family holds the card's size, its
//! corner and colours, and each tone's edge and title colour.

use super::notice::Tone;
use super::{ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::color::Rgba;
use crate::tokens::TokenRef;
use crate::values;

/// Toast tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ToastTokens {
    /// Outer width of a card: `toast.width`.
    pub width: f32,
    /// Between a card's edge and its content: `toast.pad`, half a small control.
    pub pad: f32,
    /// From the host rect's edge at the reading end and from its bottom edge to the stack:
    /// `toast.inset`, `space.pad.dialog` and `space.gap.section`.
    pub inset: [f32; 2],
    /// Between cards: `space.gap.stack`.
    pub gap: f32,
    /// Corner radius: `shape.radius.popup`.
    pub radius: f32,
    /// Card fill: `surface.raised`.
    pub fill: Rgba,
    /// Bullet lines: `text.muted`.
    pub line: Rgba,
    /// The "…and n more" line: `text.faint`.
    pub more: Rgba,
    edge: [Rgba; Tone::ALL.len()],
    title: [Rgba; Tone::ALL.len()],
}

impl ToastTokens {
    /// The card's outline for `tone`: `status.<tone>.fg`; neutral is `line.border`. A bevel theme
    /// keeps its bevel.
    pub fn edge(&self, tone: Tone) -> Rgba {
        self.edge[tone as usize]
    }

    /// The title for `tone`: `status.<tone>.fg`; neutral is `text.normal`.
    pub fn title(&self, tone: Tone) -> Rgba {
        self.title[tone as usize]
    }
}

/// An overridable toast field. Paths: `radius`, `fill`, `line`, `more`, `<tone>.edge`,
/// `<tone>.title`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ToastField {
    /// `toast.radius`
    Radius,
    /// `toast.fill`
    Fill,
    /// `toast.line`
    Line,
    /// `toast.more`
    More,
    /// `toast.<tone>.edge`
    Edge(Tone),
    /// `toast.<tone>.title`
    Title(Tone),
}

const FIELD_COUNT: usize = 4 + 2 * Tone::ALL.len();

const FIELDS: [ToastField; FIELD_COUNT] = {
    let mut out = [ToastField::Radius; FIELD_COUNT];
    out[1] = ToastField::Fill;
    out[2] = ToastField::Line;
    out[3] = ToastField::More;
    let mut i = 0;
    while i < Tone::ALL.len() {
        out[4 + 2 * i] = ToastField::Edge(Tone::ALL[i]);
        out[5 + 2 * i] = ToastField::Title(Tone::ALL[i]);
        i += 1;
    }
    out
};

impl Family for ToastTokens {
    type Field = ToastField;
    const FIELDS: &'static [ToastField] = &FIELDS;

    fn defaults(s: &Semantic<'_>) -> ToastTokens {
        let c = s.color;
        ToastTokens {
            width: values::TOAST_WIDTH,
            pad: s.size.control.sm / 2.0,
            inset: [s.space.pad.dialog, s.space.gap.section],
            gap: s.space.gap.stack,
            radius: s.shape.radius.popup,
            fill: c.surface.raised,
            line: c.text.muted,
            more: c.text.faint,
            edge: Tone::ALL.map(|t| t.colors(c).map_or(c.line.border, |s| s.fg)),
            title: Tone::ALL.map(|t| t.colors(c).map_or(c.text.normal, |s| s.fg)),
        }
    }

    fn path(f: ToastField) -> String {
        match f {
            ToastField::Radius => "radius".to_owned(),
            ToastField::Fill => "fill".to_owned(),
            ToastField::Line => "line".to_owned(),
            ToastField::More => "more".to_owned(),
            ToastField::Edge(tone) => format!("{}.edge", tone.name()),
            ToastField::Title(tone) => format!("{}.title", tone.name()),
        }
    }

    fn from_path(path: &str) -> Option<ToastField> {
        Some(match path {
            "radius" => ToastField::Radius,
            "fill" => ToastField::Fill,
            "line" => ToastField::Line,
            "more" => ToastField::More,
            _ => {
                let (tone, part) = path.split_once('.')?;
                let tone = Tone::from_name(tone)?;
                match part {
                    "edge" => ToastField::Edge(tone),
                    "title" => ToastField::Title(tone),
                    _ => return None,
                }
            }
        })
    }

    fn kind(f: ToastField) -> FieldKind {
        match f {
            ToastField::Radius => FieldKind::Length,
            _ => FieldKind::Color,
        }
    }

    fn get(&self, f: ToastField) -> OverrideValue {
        match f {
            ToastField::Radius => OverrideValue::Length(self.radius),
            ToastField::Fill => OverrideValue::Color(self.fill),
            ToastField::Line => OverrideValue::Color(self.line),
            ToastField::More => OverrideValue::Color(self.more),
            ToastField::Edge(tone) => OverrideValue::Color(self.edge(tone)),
            ToastField::Title(tone) => OverrideValue::Color(self.title(tone)),
        }
    }

    fn set(&mut self, f: ToastField, v: OverrideValue) {
        match (f, v) {
            (ToastField::Radius, OverrideValue::Length(r)) => self.radius = r,
            (ToastField::Fill, OverrideValue::Color(c)) => self.fill = c,
            (ToastField::Line, OverrideValue::Color(c)) => self.line = c,
            (ToastField::More, OverrideValue::Color(c)) => self.more = c,
            (ToastField::Edge(tone), OverrideValue::Color(c)) => self.edge[tone as usize] = c,
            (ToastField::Title(tone), OverrideValue::Color(c)) => self.title[tone as usize] = c,
            // Mismatched kinds are rejected before this is called.
            _ => {}
        }
    }

    /// Titles, lines and the "more" text on the card fill. Edges are design targets, not rules:
    /// the title text identifies the tone.
    fn pairs() -> Vec<ContrastPair> {
        let fill = TokenRef::Component(ComponentField::Toast(ToastField::Fill));
        let pair = |f| ContrastPair { fg: ComponentField::Toast(f), bg: fill, minimum: values::TEXT_MIN };
        let mut out: Vec<ContrastPair> = Tone::ALL.into_iter().map(|t| pair(ToastField::Title(t))).collect();
        out.push(pair(ToastField::Line));
        out.push(pair(ToastField::More));
        out
    }
}
