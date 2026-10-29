//! Swatches: a colour well. The well shows its colour over a checkerboard where it has alpha,
//! inside two rings: an outer `line.control_edge` ring against the panel and an inner black or
//! white ring against the colour, so the bounds show for any colour. This family holds the size,
//! corner and the colours the well draws itself; the colour shown is the caller's.

use super::{BySize, ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::color::Rgba;
use crate::contrast::{contrast_ratio, relative_luminance};
use crate::derive::over;
use crate::tokens::{ColorToken, TokenRef};
use crate::values;

/// A colour a swatch draws itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SwatchColor {
    /// The ring on the well's outline.
    Edge,
    /// The checkerboard's cells that hold the corners.
    CheckerA,
    /// The checkerboard's other cells.
    CheckerB,
    /// The selection ring.
    Selected,
    /// The well of a mixed value.
    MixedFill,
    /// The dash on a mixed value.
    MixedMark,
}

impl SwatchColor {
    /// Every colour, in path order.
    pub const ALL: [SwatchColor; 6] =
        [SwatchColor::Edge, SwatchColor::CheckerA, SwatchColor::CheckerB, SwatchColor::Selected, SwatchColor::MixedFill, SwatchColor::MixedMark];

    /// The path after `swatch.`, e.g. `"checker.a"`.
    pub const fn name(self) -> &'static str {
        match self {
            SwatchColor::Edge => "edge",
            SwatchColor::CheckerA => "checker.a",
            SwatchColor::CheckerB => "checker.b",
            SwatchColor::Selected => "selected",
            SwatchColor::MixedFill => "mixed.fill",
            SwatchColor::MixedMark => "mixed.mark",
        }
    }
}

/// Swatch tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SwatchTokens {
    /// Side of the swatch: the control height, so a swatch sits in a control row and is as easy
    /// to hit as the controls beside it. The well is inset from it by the selection ring's gap
    /// and width.
    pub size: BySize<f32>,
    /// Corner radius of the well: `shape.radius.control` or the theme's `swatch.radius`, at
    /// most [`values::SWATCH_RADIUS_MAX`]. The rings around it are concentric.
    pub radius: f32,
    /// Width of the selection ring.
    pub selected_width: f32,
    /// Between the well and the selection ring.
    pub selected_gap: f32,
    /// Ring on the outline, 1 point wide: `line.control_edge`. Inside it the well draws a ring
    /// from its colour, [`inner_edge`](SwatchTokens::inner_edge).
    pub edge: Rgba,
    /// Checkerboard cell that holds the corners: white in a light theme, dark grey in a dark one.
    pub checker_a: Rgba,
    /// The other checkerboard cell: a light or mid grey beside `checker_a`.
    pub checker_b: Rgba,
    /// Selection ring, outside the well past the gap: `accent.solid`. The keyboard focus ring
    /// goes outside it.
    pub selected: Rgba,
    /// The well when the value is mixed: `surface.sunken`.
    pub mixed_fill: Rgba,
    /// The dash on a mixed well: `text.muted`.
    pub mixed_mark: Rgba,
}

impl SwatchTokens {
    /// The colour `c` names.
    pub fn color(&self, c: SwatchColor) -> Rgba {
        match c {
            SwatchColor::Edge => self.edge,
            SwatchColor::CheckerA => self.checker_a,
            SwatchColor::CheckerB => self.checker_b,
            SwatchColor::Selected => self.selected,
            SwatchColor::MixedFill => self.mixed_fill,
            SwatchColor::MixedMark => self.mixed_mark,
        }
    }

    fn color_mut(&mut self, c: SwatchColor) -> &mut Rgba {
        match c {
            SwatchColor::Edge => &mut self.edge,
            SwatchColor::CheckerA => &mut self.checker_a,
            SwatchColor::CheckerB => &mut self.checker_b,
            SwatchColor::Selected => &mut self.selected,
            SwatchColor::MixedFill => &mut self.mixed_fill,
            SwatchColor::MixedMark => &mut self.mixed_mark,
        }
    }

    /// The ring just inside the edge of a well showing `color` (straight alpha, over the
    /// checkerboard): black or white at [`values::SWATCH_INNER_EDGE_ALPHA`], whichever stands out
    /// more from the colour, as macOS draws its colour wells. The well's bounds then show even
    /// where the colour equals the panel and the outer edge doesn't.
    pub fn inner_edge(&self, color: Rgba) -> Rgba {
        let shown = [over(color, self.checker_a), over(color, self.checker_b)];
        let ring = |base: Rgba| base.with_alpha(values::SWATCH_INNER_EDGE_ALPHA);
        let least = |base: Rgba| shown.iter().map(|&s| contrast_ratio(over(ring(base), s), s)).fold(f64::INFINITY, f64::min);
        if least(Rgba::BLACK) >= least(Rgba::WHITE) { ring(Rgba::BLACK) } else { ring(Rgba::WHITE) }
    }
}

/// An overridable swatch field. Paths: `radius`, `edge`, `checker.a`, `checker.b`, `selected`,
/// `mixed.fill`, `mixed.mark`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SwatchField {
    /// `swatch.radius`
    Radius,
    /// `swatch.<colour>`
    Color(SwatchColor),
}

const FIELDS: [SwatchField; 1 + SwatchColor::ALL.len()] = {
    let mut out = [SwatchField::Radius; 1 + SwatchColor::ALL.len()];
    let mut i = 0;
    while i < SwatchColor::ALL.len() {
        out[1 + i] = SwatchField::Color(SwatchColor::ALL[i]);
        i += 1;
    }
    out
};

impl Family for SwatchTokens {
    type Field = SwatchField;
    const FIELDS: &'static [SwatchField] = &FIELDS;

    fn defaults(s: &Semantic<'_>) -> SwatchTokens {
        let (c, size) = (s.color, s.size);
        let dark = relative_luminance(c.text.normal) > relative_luminance(c.surface.panel);
        let (checker_a, checker_b) = if dark { values::SWATCH_CHECKER_DARK } else { values::SWATCH_CHECKER_LIGHT };
        SwatchTokens {
            size: BySize { sm: size.control.sm, md: size.control.md },
            radius: s.shape.radius.control.min(values::SWATCH_RADIUS_MAX),
            selected_width: values::SWATCH_SELECTED_WIDTH,
            selected_gap: values::SWATCH_SELECTED_GAP,
            edge: c.line.control_edge,
            checker_a,
            checker_b,
            selected: c.accent.solid,
            mixed_fill: c.surface.sunken,
            mixed_mark: c.text.muted,
        }
    }

    fn path(f: SwatchField) -> String {
        match f {
            SwatchField::Radius => "radius".to_owned(),
            SwatchField::Color(c) => c.name().to_owned(),
        }
    }

    fn from_path(path: &str) -> Option<SwatchField> {
        if path == "radius" {
            return Some(SwatchField::Radius);
        }
        SwatchColor::ALL.into_iter().find(|c| c.name() == path).map(SwatchField::Color)
    }

    fn kind(f: SwatchField) -> FieldKind {
        match f {
            SwatchField::Radius => FieldKind::Length,
            SwatchField::Color(_) => FieldKind::Color,
        }
    }

    fn get(&self, f: SwatchField) -> OverrideValue {
        match f {
            SwatchField::Radius => OverrideValue::Length(self.radius),
            SwatchField::Color(c) => OverrideValue::Color(self.color(c)),
        }
    }

    fn set(&mut self, f: SwatchField, v: OverrideValue) {
        match (f, v) {
            (SwatchField::Radius, OverrideValue::Length(r)) => self.radius = r.min(values::SWATCH_RADIUS_MAX),
            (SwatchField::Color(c), OverrideValue::Color(v)) => *self.color_mut(c) = v,
            // Mismatched kinds are rejected before this is called.
            _ => {}
        }
    }

    /// The selection ring is a selection mark in the accent colour and the dash is the only cue
    /// of a mixed value: 3:1 (4.5:1 with high contrast). The rings, which only show the bounds,
    /// are design targets.
    fn pairs() -> Vec<ContrastPair> {
        let field = |c| ComponentField::Swatch(SwatchField::Color(c));
        vec![
            ContrastPair {
                fg: TokenRef::Component(field(SwatchColor::Selected)),
                bg: TokenRef::Color(ColorToken::SurfacePanel),
                minimum: values::NON_TEXT_MIN,
                alternate: None,
            },
            ContrastPair {
                fg: TokenRef::Component(field(SwatchColor::MixedMark)),
                bg: TokenRef::Component(field(SwatchColor::MixedFill)),
                minimum: values::NON_TEXT_MIN,
                alternate: None,
            },
        ]
    }
}
