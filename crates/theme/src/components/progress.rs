//! Progress bars: a sunken track and a fill from the reading start. The track takes its edge from
//! the `Surface` primitive; this family holds its thickness, corners and colours.

use super::{ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::color::Rgba;
use crate::tokens::TokenRef;
use crate::values;

/// Progress tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ProgressTokens {
    /// Thickness of the track.
    pub track_height: f32,
    /// Corner radius of the track and the fill, each clamped to half the thickness; 999 gives a
    /// pill.
    pub radius: f32,
    /// The track's fill: `surface.sunken`.
    pub track: Rgba,
    /// The filled part: `accent.solid`.
    pub fill: Rgba,
}

/// An overridable progress field. Paths: `radius`, `track`, `fill`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ProgressField {
    /// `progress.radius`
    Radius,
    /// `progress.track`
    Track,
    /// `progress.fill`
    Fill,
}

impl Family for ProgressTokens {
    type Field = ProgressField;
    const FIELDS: &'static [ProgressField] = &[ProgressField::Radius, ProgressField::Track, ProgressField::Fill];

    fn defaults(s: &Semantic<'_>) -> ProgressTokens {
        ProgressTokens {
            track_height: values::PROGRESS_TRACK,
            radius: values::PROGRESS_RADIUS.px(),
            track: s.color.surface.sunken,
            fill: s.color.accent.solid,
        }
    }

    fn path(f: ProgressField) -> String {
        match f {
            ProgressField::Radius => "radius",
            ProgressField::Track => "track",
            ProgressField::Fill => "fill",
        }
        .to_owned()
    }

    fn from_path(path: &str) -> Option<ProgressField> {
        Some(match path {
            "radius" => ProgressField::Radius,
            "track" => ProgressField::Track,
            "fill" => ProgressField::Fill,
            _ => return None,
        })
    }

    fn kind(f: ProgressField) -> FieldKind {
        match f {
            ProgressField::Radius => FieldKind::Length,
            _ => FieldKind::Color,
        }
    }

    fn get(&self, f: ProgressField) -> OverrideValue {
        match f {
            ProgressField::Radius => OverrideValue::Length(self.radius),
            ProgressField::Track => OverrideValue::Color(self.track),
            ProgressField::Fill => OverrideValue::Color(self.fill),
        }
    }

    fn set(&mut self, f: ProgressField, v: OverrideValue) {
        match (f, v) {
            (ProgressField::Radius, OverrideValue::Length(r)) => self.radius = r,
            (ProgressField::Track, OverrideValue::Color(c)) => self.track = c,
            (ProgressField::Fill, OverrideValue::Color(c)) => self.fill = c,
            // Mismatched kinds are rejected before this is called.
            _ => {}
        }
    }

    /// The fill against the track: a state indicator (WCAG 1.4.11), 3:1 and 4.5:1 with high
    /// contrast, as the slider's fill.
    fn pairs() -> Vec<ContrastPair> {
        vec![ContrastPair {
            fg: ComponentField::Progress(ProgressField::Fill),
            bg: TokenRef::Component(ComponentField::Progress(ProgressField::Track)),
            minimum: values::NON_TEXT_MIN,
        }]
    }
}
