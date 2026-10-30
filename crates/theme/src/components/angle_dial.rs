//! Angle dials: the round face with a needle that points in a direction. Sizes scale with the
//! Size preset; the colours are overridable.

use super::{ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
use crate::tokens::TokenRef;
use crate::values;

/// Angle dial tokens. Sizes are in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct AngleDialTokens {
    /// Side of the dial's box: `angle_dial.size`.
    pub size: f32,
    /// Between the box and the face's edge: `angle_dial.margin`.
    pub margin: f32,
    /// Width of the needle: `angle_dial.needle_width`.
    pub needle_width: f32,
    /// Distance from the face's edge to the needle's tip: `angle_dial.needle_inset`.
    pub needle_inset: f32,
    /// Radius of the dot at the centre: `angle_dial.hub`.
    pub hub: f32,
    /// Radius round the centre within which the pointer changes nothing: `angle_dial.dead_zone`.
    pub dead_zone: f32,
    /// The face: `surface.sunken`.
    pub face: Rgba,
    /// The face's one-point edge: `line.control_edge`.
    pub edge: Rgba,
    /// The needle, which shows the value: `accent.solid`.
    pub needle: Rgba,
    /// The dot at the centre: `text.muted`.
    pub hub_fill: Rgba,
}

/// An overridable angle dial field. Paths: `face`, `edge`, `needle`, `hub_fill`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AngleDialField {
    /// `angle_dial.face`
    Face,
    /// `angle_dial.edge`
    Edge,
    /// `angle_dial.needle`
    Needle,
    /// `angle_dial.hub_fill`
    HubFill,
}

impl AngleDialField {
    const ALL: [AngleDialField; 4] = [AngleDialField::Face, AngleDialField::Edge, AngleDialField::Needle, AngleDialField::HubFill];

    const fn name(self) -> &'static str {
        match self {
            AngleDialField::Face => "face",
            AngleDialField::Edge => "edge",
            AngleDialField::Needle => "needle",
            AngleDialField::HubFill => "hub_fill",
        }
    }
}

impl AngleDialTokens {
    fn color(&self, f: AngleDialField) -> Rgba {
        match f {
            AngleDialField::Face => self.face,
            AngleDialField::Edge => self.edge,
            AngleDialField::Needle => self.needle,
            AngleDialField::HubFill => self.hub_fill,
        }
    }

    fn color_mut(&mut self, f: AngleDialField) -> &mut Rgba {
        match f {
            AngleDialField::Face => &mut self.face,
            AngleDialField::Edge => &mut self.edge,
            AngleDialField::Needle => &mut self.needle,
            AngleDialField::HubFill => &mut self.hub_fill,
        }
    }
}

impl Family for AngleDialTokens {
    type Field = AngleDialField;
    const FIELDS: &'static [AngleDialField] = &AngleDialField::ALL;
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { AngleDialTokens;
        "size" 0.0..=1024.0 => size;
        "margin" 0.0..=64.0 => margin;
        "needle_width" 0.0..=1024.0 => needle_width;
        "needle_inset" 0.0..=64.0 => needle_inset;
        "hub" 0.0..=1024.0 => hub;
        "dead_zone" 0.0..=1024.0 => dead_zone;
    };

    /// The Compact sizes (PhotoCraft's `angle_dial_row`), scaled by the control height of the size
    /// and rounded to whole points.
    fn defaults(s: &Semantic<'_>) -> AngleDialTokens {
        let k = s.size.control.md / values::ANGLE_DIAL_BASE_CONTROL;
        let scale = |w: f32| (w * k).round();
        let c = s.color;
        AngleDialTokens {
            size: scale(values::ANGLE_DIAL_SIZE),
            margin: scale(values::ANGLE_DIAL_MARGIN),
            needle_width: scale(values::ANGLE_DIAL_NEEDLE_WIDTH),
            needle_inset: scale(values::ANGLE_DIAL_NEEDLE_INSET),
            hub: scale(values::ANGLE_DIAL_HUB),
            dead_zone: scale(values::ANGLE_DIAL_DEAD_ZONE),
            face: c.surface.sunken,
            edge: c.line.control_edge,
            needle: c.accent.solid,
            hub_fill: c.text.muted,
        }
    }

    fn path(f: AngleDialField) -> String {
        f.name().to_owned()
    }

    fn from_path(path: &str) -> Option<AngleDialField> {
        AngleDialField::ALL.into_iter().find(|f| f.name() == path)
    }

    fn kind(_: AngleDialField) -> FieldKind {
        FieldKind::Color
    }

    fn get(&self, f: AngleDialField) -> OverrideValue {
        OverrideValue::Color(self.color(f))
    }

    fn set(&mut self, f: AngleDialField, v: OverrideValue) {
        if let OverrideValue::Color(c) = v {
            *self.color_mut(f) = c;
        }
    }

    /// The needle is the value (WCAG 1.4.11), so it meets the face at 3:1 and 4.5:1 with high
    /// contrast.
    fn pairs() -> Vec<ContrastPair> {
        let fg = TokenRef::Component(ComponentField::AngleDial(AngleDialField::Needle));
        let bg = TokenRef::Component(ComponentField::AngleDial(AngleDialField::Face));
        vec![ContrastPair { fg, bg, minimum: values::NON_TEXT_MIN, alternate: None }]
    }
}
