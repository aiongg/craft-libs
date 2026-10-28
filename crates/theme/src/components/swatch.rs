//! Swatches: a colour well. The well shows its colour over a checkerboard where it has alpha,
//! inside two rings: an outer `line.control_edge` ring against the panel and an inner ring against
//! the colour, so the bounds show for any colour. This family holds the size, corner and the
//! colours the well draws itself; the colour shown is the caller's.

use super::{BySize, ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::color::Rgba;
use crate::contrast::relative_luminance;
use crate::tokens::{ColorToken, TokenRef};
use crate::values;

/// A colour a swatch draws itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SwatchColor {
    /// The ring on the well's outline.
    EdgeOuter,
    /// The ring just inside the outer one.
    EdgeInner,
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
    pub const ALL: [SwatchColor; 7] = [
        SwatchColor::EdgeOuter,
        SwatchColor::EdgeInner,
        SwatchColor::CheckerA,
        SwatchColor::CheckerB,
        SwatchColor::Selected,
        SwatchColor::MixedFill,
        SwatchColor::MixedMark,
    ];

    /// The path after `swatch.`, e.g. `"edge.outer"`.
    pub const fn name(self) -> &'static str {
        match self {
            SwatchColor::EdgeOuter => "edge.outer",
            SwatchColor::EdgeInner => "edge.inner",
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
    /// Side of the well: the control height, so a swatch sits in a control row and is as easy to
    /// hit as the controls beside it.
    pub size: BySize<f32>,
    /// Corner radius: `shape.radius.control`, at most [`values::SWATCH_RADIUS_MAX`].
    pub radius: f32,
    /// Width of the selection ring.
    pub selected_width: f32,
    /// Outer ring, 1 point on the outline: `line.control_edge`.
    pub edge_outer: Rgba,
    /// Inner ring, 1 point inside the outer one: `surface.raised`.
    pub edge_inner: Rgba,
    /// Checkerboard cell that holds the corners: white in a light theme, dark grey in a dark one.
    pub checker_a: Rgba,
    /// The other checkerboard cell: a light or mid grey beside `checker_a`.
    pub checker_b: Rgba,
    /// Selection ring, outside the well: `accent.solid`, which is not the keyboard focus ring's
    /// colour.
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
            SwatchColor::EdgeOuter => self.edge_outer,
            SwatchColor::EdgeInner => self.edge_inner,
            SwatchColor::CheckerA => self.checker_a,
            SwatchColor::CheckerB => self.checker_b,
            SwatchColor::Selected => self.selected,
            SwatchColor::MixedFill => self.mixed_fill,
            SwatchColor::MixedMark => self.mixed_mark,
        }
    }

    fn color_mut(&mut self, c: SwatchColor) -> &mut Rgba {
        match c {
            SwatchColor::EdgeOuter => &mut self.edge_outer,
            SwatchColor::EdgeInner => &mut self.edge_inner,
            SwatchColor::CheckerA => &mut self.checker_a,
            SwatchColor::CheckerB => &mut self.checker_b,
            SwatchColor::Selected => &mut self.selected,
            SwatchColor::MixedFill => &mut self.mixed_fill,
            SwatchColor::MixedMark => &mut self.mixed_mark,
        }
    }
}

/// An overridable swatch field. Paths: `radius`, `edge.outer`, `edge.inner`, `checker.a`,
/// `checker.b`, `selected`, `mixed.fill`, `mixed.mark`.
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
            edge_outer: c.line.control_edge,
            edge_inner: c.surface.raised,
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
            (SwatchField::Radius, OverrideValue::Length(r)) => self.radius = r,
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
            ContrastPair { fg: field(SwatchColor::Selected), bg: TokenRef::Color(ColorToken::SurfacePanel), minimum: values::NON_TEXT_MIN },
            ContrastPair { fg: field(SwatchColor::MixedMark), bg: TokenRef::Component(field(SwatchColor::MixedFill)), minimum: values::NON_TEXT_MIN },
        ]
    }
}
