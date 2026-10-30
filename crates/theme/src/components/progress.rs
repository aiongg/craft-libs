//! Progress bars: a sunken track and a fill from the reading start. The track takes its edge from
//! the `Surface` primitive; this family holds its thickness, corners and colours.

use super::{ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
use crate::tokens::{ColorToken, TokenRef};
use crate::values;

/// Progress tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ProgressTokens {
    /// Default length of the bar: `progress.width`.
    pub width: f32,
    /// Thickness of the track.
    pub track_height: f32,
    /// Corner radius of the track and the fill, each clamped to half the thickness; 999 gives a
    /// pill.
    pub radius: f32,
    /// The track's fill: `surface.sunken`.
    pub track: Rgba,
    /// The track's one-point outline, inside it: `line.control_edge`. A bevelled theme draws its
    /// own edge instead.
    pub edge: Rgba,
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
    /// `progress.edge`
    Edge,
    /// `progress.fill`
    Fill,
}

impl Family for ProgressTokens {
    type Field = ProgressField;
    const FIELDS: &'static [ProgressField] = &[ProgressField::Radius, ProgressField::Track, ProgressField::Edge, ProgressField::Fill];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { ProgressTokens;
        "width" 0.0..=1024.0 => width;
        "track_height" 0.0..=1024.0 => track_height;
    };

    fn defaults(s: &Semantic<'_>) -> ProgressTokens {
        ProgressTokens {
            width: s.set.progress_width,
            track_height: values::PROGRESS_TRACK,
            radius: s.shape.radius.pill,
            track: s.color.surface.sunken,
            edge: s.color.line.control_edge,
            fill: s.color.accent.solid,
        }
    }

    fn path(f: ProgressField) -> String {
        match f {
            ProgressField::Radius => "radius",
            ProgressField::Track => "track",
            ProgressField::Edge => "edge",
            ProgressField::Fill => "fill",
        }
        .to_owned()
    }

    fn from_path(path: &str) -> Option<ProgressField> {
        Some(match path {
            "radius" => ProgressField::Radius,
            "track" => ProgressField::Track,
            "edge" => ProgressField::Edge,
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
            ProgressField::Edge => OverrideValue::Color(self.edge),
            ProgressField::Fill => OverrideValue::Color(self.fill),
        }
    }

    fn set(&mut self, f: ProgressField, v: OverrideValue) {
        match (f, v) {
            (ProgressField::Radius, OverrideValue::Length(r)) => self.radius = r,
            (ProgressField::Track, OverrideValue::Color(c)) => self.track = c,
            (ProgressField::Edge, OverrideValue::Color(c)) => self.edge = c,
            (ProgressField::Fill, OverrideValue::Color(c)) => self.fill = c,
            // Mismatched kinds are rejected before this is called.
            _ => {}
        }
    }

    /// The fill against the track: a state indicator (WCAG 1.4.11), 3:1 and 4.5:1 with high
    /// contrast, as the slider's fill.
    ///
    /// The track's outline against the panel and raised surfaces: a control boundary, 3:1 in High.
    fn pairs() -> Vec<ContrastPair> {
        let edge = TokenRef::Component(ComponentField::Progress(ProgressField::Edge));
        vec![
            ContrastPair {
                fg: TokenRef::Component(ComponentField::Progress(ProgressField::Fill)),
                bg: TokenRef::Component(ComponentField::Progress(ProgressField::Track)),
                minimum: values::NON_TEXT_MIN,
                alternate: None,
            },
            ContrastPair { fg: edge, bg: TokenRef::Color(ColorToken::SurfacePanel), minimum: values::CONTROL_EDGE_MIN, alternate: None },
            ContrastPair { fg: edge, bg: TokenRef::Color(ColorToken::SurfaceRaised), minimum: values::CONTROL_EDGE_MIN, alternate: None },
        ]
    }
}
