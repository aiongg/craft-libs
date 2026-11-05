//! Adaptive Wide Angle's constraint marks. The same at every Size (they sit on the image). The line colours are the semantic `accent`, `status.warn.fg` and `status.bad.fg`; the halo under them and the colour of a constraint being drawn are overridable.

use super::{ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
use crate::tokens::{ColorToken as T, TokenRef};
use crate::values;

/// Adaptive Wide Angle's constraint marks.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct WideAngleTokens {
    /// A constraint line's width: `wide_angle.line`.
    pub line: f32,
    /// The end dots' radius: `wide_angle.end`.
    pub end: f32,
    /// How near a right-click must pass a curve to open its menu: `wide_angle.hit`.
    pub hit: f32,
    /// A drag shorter than this many screen points adds no constraint: `wide_angle.min_drag`.
    pub min_drag: f32,
    /// The extra width of the halo under each line: `wide_angle.halo_width`.
    pub halo_width: f32,
    /// The halo under each line: `wide_angle.halo`. `surface.panel`, the colour the line colours are already held against, so a dark accent still separates from it.
    pub halo: Rgba,
    /// The constraint being drawn: `wide_angle.drawing`, the semantic `accent.soft` (PhotoCraft's `accent_soft`).
    pub drawing: Rgba,
}

/// An overridable Wide Angle field. Paths: `halo`, `drawing`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum WideAngleField {
    /// `wide_angle.halo`
    Halo,
    /// `wide_angle.drawing`
    Drawing,
}

impl WideAngleField {
    const ALL: [WideAngleField; 2] = [WideAngleField::Halo, WideAngleField::Drawing];

    const fn name(self) -> &'static str {
        match self {
            WideAngleField::Halo => "halo",
            WideAngleField::Drawing => "drawing",
        }
    }
}

impl Family for WideAngleTokens {
    type Field = WideAngleField;
    const FIELDS: &'static [WideAngleField] = &WideAngleField::ALL;
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { WideAngleTokens;
        "line" 0.0..=16.0 => line;
        "end" 0.5..=32.0 => end;
        "hit" 0.0..=64.0 => hit;
        "min_drag" 0.0..=64.0 => min_drag;
        "halo_width" 0.0..=16.0 => halo_width;
    };

    fn defaults(s: &Semantic<'_>) -> WideAngleTokens {
        WideAngleTokens {
            line: values::WIDE_ANGLE_LINE,
            end: values::WIDE_ANGLE_END,
            hit: values::WIDE_ANGLE_HIT,
            min_drag: values::WIDE_ANGLE_MIN_DRAG,
            halo_width: values::WIDE_ANGLE_HALO_WIDTH,
            halo: s.color.surface.panel,
            drawing: s.color.accent.soft,
        }
    }

    fn path(f: WideAngleField) -> String {
        f.name().to_owned()
    }

    fn from_path(path: &str) -> Option<WideAngleField> {
        WideAngleField::ALL.into_iter().find(|f| f.name() == path)
    }

    fn kind(_: WideAngleField) -> FieldKind {
        FieldKind::Color
    }

    fn get(&self, f: WideAngleField) -> OverrideValue {
        OverrideValue::Color(match f {
            WideAngleField::Halo => self.halo,
            WideAngleField::Drawing => self.drawing,
        })
    }

    fn set(&mut self, f: WideAngleField, v: OverrideValue) {
        if let OverrideValue::Color(c) = v {
            match f {
                WideAngleField::Halo => self.halo = c,
                WideAngleField::Drawing => self.drawing = c,
            }
        }
    }

    /// The line colours mark the constraint's orientation on top of the halo: 3:1 (4.5:1 with high contrast).
    fn pairs() -> Vec<ContrastPair> {
        [T::AccentSolid, T::StatusWarnFg, T::StatusBadFg]
            .into_iter()
            .map(|fg| ContrastPair {
                fg: TokenRef::Color(fg),
                bg: TokenRef::Component(ComponentField::WideAngle(WideAngleField::Halo)),
                minimum: values::NON_TEXT_MIN,
                alternate: None,
            })
            .collect()
    }
}
