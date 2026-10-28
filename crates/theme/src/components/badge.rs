//! Badges: a small label in a tone, soft (the tone's tinted background, text and border) or solid
//! (the tone's solid fill with `text.on_status`). The family holds the shape and every tone's
//! colours.

use super::{ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::color::Rgba;
use crate::tokens::{TokenRef, TypeRole};
use crate::values;

/// What a badge says about its subject.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum BadgeTone {
    /// No status: a count, a tag.
    #[default]
    Neutral,
    /// The accent: new, selected.
    Accent,
    /// Success.
    Good,
    /// Needs attention.
    Warn,
    /// Failure.
    Bad,
    /// Information.
    Info,
}

impl BadgeTone {
    /// Every tone, in path order.
    pub const ALL: [BadgeTone; 6] = [BadgeTone::Neutral, BadgeTone::Accent, BadgeTone::Good, BadgeTone::Warn, BadgeTone::Bad, BadgeTone::Info];

    /// The path segment, e.g. `"good"`.
    pub const fn name(self) -> &'static str {
        match self {
            BadgeTone::Neutral => "neutral",
            BadgeTone::Accent => "accent",
            BadgeTone::Good => "good",
            BadgeTone::Warn => "warn",
            BadgeTone::Bad => "bad",
            BadgeTone::Info => "info",
        }
    }

    /// The tone whose [`name`](BadgeTone::name) is `name`.
    pub fn from_name(name: &str) -> Option<BadgeTone> {
        BadgeTone::ALL.into_iter().find(|t| t.name() == name)
    }
}

/// How a badge fills its tone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum BadgeStyle {
    /// The tone's tinted background, text and border.
    #[default]
    Soft,
    /// The tone's solid fill, no border.
    Solid,
}

impl BadgeStyle {
    /// Both styles, in path order.
    pub const ALL: [BadgeStyle; 2] = [BadgeStyle::Soft, BadgeStyle::Solid];

    /// The path segment, e.g. `"soft"`.
    pub const fn name(self) -> &'static str {
        match self {
            BadgeStyle::Soft => "soft",
            BadgeStyle::Solid => "solid",
        }
    }

    /// The style whose [`name`](BadgeStyle::name) is `name`.
    pub fn from_name(name: &str) -> Option<BadgeStyle> {
        BadgeStyle::ALL.into_iter().find(|s| s.name() == name)
    }
}

/// One coloured part of a badge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BadgePart {
    /// The background.
    Fill,
    /// The label.
    Text,
    /// The outline; a solid badge's is its fill.
    Border,
}

impl BadgePart {
    /// Every part, in path order.
    pub const ALL: [BadgePart; 3] = [BadgePart::Fill, BadgePart::Text, BadgePart::Border];

    /// The path segment, e.g. `"fill"`.
    pub const fn name(self) -> &'static str {
        match self {
            BadgePart::Fill => "fill",
            BadgePart::Text => "text",
            BadgePart::Border => "border",
        }
    }
}

/// The colours of one tone in one style.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BadgeColors {
    /// Background.
    pub fill: Rgba,
    /// Label.
    pub text: Rgba,
    /// Outline.
    pub border: Rgba,
}

impl BadgeColors {
    /// The colour of `part`.
    pub fn part(&self, part: BadgePart) -> Rgba {
        match part {
            BadgePart::Fill => self.fill,
            BadgePart::Text => self.text,
            BadgePart::Border => self.border,
        }
    }

    fn part_mut(&mut self, part: BadgePart) -> &mut Rgba {
        match part {
            BadgePart::Fill => &mut self.fill,
            BadgePart::Text => &mut self.text,
            BadgePart::Border => &mut self.border,
        }
    }
}

/// Badge tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct BadgeTokens {
    /// Height: the label's line and [`pad_y`](BadgeTokens::pad_y) above and below it, so it
    /// grows with the Text size setting.
    pub height: f32,
    /// Above and below the label's line.
    pub pad_y: f32,
    /// At the start and end of the label: `space.pad.control`.
    pub pad_x: f32,
    /// Corner radius, clamped to half the height; 999 gives a pill.
    pub radius: f32,
    /// Label type role: Caption.
    pub label: TypeRole,
    colors: [[BadgeColors; BadgeStyle::ALL.len()]; BadgeTone::ALL.len()],
}

impl BadgeTokens {
    /// The colours of `tone` in `style`. Soft: `status.<tone>.bg`, `fg` and `border`; solid:
    /// `status.<tone>.solid` with `text.on_status`. Neutral is `surface.sunken` with `text.muted`
    /// (solid: `text.muted` with `surface.panel`); accent is `state.selected` with `text.on_selected`, the
    /// pair the semantic pass keeps readable (solid: `accent.solid` with `text.on_accent`).
    pub fn colors(&self, tone: BadgeTone, style: BadgeStyle) -> &BadgeColors {
        &self.colors[tone as usize][style as usize]
    }

    fn colors_mut(&mut self, tone: BadgeTone, style: BadgeStyle) -> &mut BadgeColors {
        &mut self.colors[tone as usize][style as usize]
    }
}

/// An overridable badge field. Paths: `radius`, `<tone>.<style>.<part>`, e.g.
/// `badge.good.solid.fill`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BadgeField {
    /// `badge.radius`
    Radius,
    /// `badge.<tone>.<style>.<part>`
    Color {
        /// The tone.
        tone: BadgeTone,
        /// The style.
        style: BadgeStyle,
        /// The part.
        part: BadgePart,
    },
}

const PER_TONE: usize = BadgeStyle::ALL.len() * BadgePart::ALL.len();
const FIELD_COUNT: usize = 1 + BadgeTone::ALL.len() * PER_TONE;

const FIELDS: [BadgeField; FIELD_COUNT] = {
    let mut out = [BadgeField::Radius; FIELD_COUNT];
    let mut i = 0;
    while i < FIELD_COUNT - 1 {
        let (tone, rest) = (i / PER_TONE, i % PER_TONE);
        out[1 + i] = BadgeField::Color {
            tone: BadgeTone::ALL[tone],
            style: BadgeStyle::ALL[rest / BadgePart::ALL.len()],
            part: BadgePart::ALL[rest % BadgePart::ALL.len()],
        };
        i += 1;
    }
    out
};

impl Family for BadgeTokens {
    type Field = BadgeField;
    const FIELDS: &'static [BadgeField] = &FIELDS;

    fn defaults(s: &Semantic<'_>) -> BadgeTokens {
        let c = s.color;
        let soft = |fill, text, border| BadgeColors { fill, text, border };
        let solid = |fill, text| BadgeColors { fill, text, border: fill };
        let status = |t: &crate::tokens::StatusColors| [soft(t.bg, t.fg, t.border), solid(t.solid, c.text.on_status)];
        let label = TypeRole::Caption;
        BadgeTokens {
            height: s.typography.get(label).line_height + 2.0 * values::BADGE_PAD_Y,
            pad_y: values::BADGE_PAD_Y,
            pad_x: s.space.pad.control,
            radius: s.shape.radius.pill,
            label,
            colors: [
                [soft(c.surface.sunken, c.text.muted, c.line.border), solid(c.text.muted, c.surface.panel)],
                [soft(c.state.selected, c.text.on_selected, c.state.selected), solid(c.accent.solid, c.text.on_accent)],
                status(&c.status.good),
                status(&c.status.warn),
                status(&c.status.bad),
                status(&c.status.info),
            ],
        }
    }

    fn path(f: BadgeField) -> String {
        match f {
            BadgeField::Radius => "radius".to_owned(),
            BadgeField::Color { tone, style, part } => format!("{}.{}.{}", tone.name(), style.name(), part.name()),
        }
    }

    fn from_path(path: &str) -> Option<BadgeField> {
        if path == "radius" {
            return Some(BadgeField::Radius);
        }
        let mut it = path.split('.');
        let (tone, style, part) = (it.next()?, it.next()?, it.next()?);
        if it.next().is_some() {
            return None;
        }
        Some(BadgeField::Color {
            tone: BadgeTone::from_name(tone)?,
            style: BadgeStyle::from_name(style)?,
            part: BadgePart::ALL.into_iter().find(|p| p.name() == part)?,
        })
    }

    fn kind(f: BadgeField) -> FieldKind {
        match f {
            BadgeField::Radius => FieldKind::Length,
            BadgeField::Color { .. } => FieldKind::Color,
        }
    }

    fn get(&self, f: BadgeField) -> OverrideValue {
        match f {
            BadgeField::Radius => OverrideValue::Length(self.radius),
            BadgeField::Color { tone, style, part } => OverrideValue::Color(self.colors(tone, style).part(part)),
        }
    }

    fn set(&mut self, f: BadgeField, v: OverrideValue) {
        match (f, v) {
            (BadgeField::Radius, OverrideValue::Length(r)) => self.radius = r,
            (BadgeField::Color { tone, style, part }, OverrideValue::Color(c)) => *self.colors_mut(tone, style).part_mut(part) = c,
            // Mismatched kinds are rejected before this is called.
            _ => {}
        }
    }

    /// The label is text on its fill, in every tone and style.
    fn pairs() -> Vec<ContrastPair> {
        let field = |tone, style, part| ComponentField::Badge(BadgeField::Color { tone, style, part });
        BadgeTone::ALL
            .into_iter()
            .flat_map(|tone| BadgeStyle::ALL.into_iter().map(move |style| (tone, style)))
            .map(|(tone, style)| ContrastPair {
                fg: field(tone, style, BadgePart::Text),
                bg: TokenRef::Component(field(tone, style, BadgePart::Fill)),
                minimum: values::TEXT_MIN,
            })
            .collect()
    }
}
