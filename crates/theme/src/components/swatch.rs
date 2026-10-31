//! Swatches: a colour well. The well fills its box and shows its colour over a checkerboard
//! where it has alpha, inside a one-point `line.control_edge` ring. This family holds the size,
//! corner and the colours the well draws itself; the colour shown is the caller's.

use super::{BySize, ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
use crate::contrast::contrast_ratio;
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
    /// to hit as the controls beside it. The well fills it.
    pub size: BySize<f32>,
    /// Corner radius of the well: `shape.radius.control` or the theme's `swatch.radius`, at
    /// most [`values::SWATCH_RADIUS_MAX`].
    pub radius: f32,
    /// Width of the selection ring.
    pub selected_width: f32,
    /// Ring on the outline, 1 point wide: `line.control_edge`.
    pub edge: Rgba,
    /// Side of a checkerboard cell: [`values::SWATCH_CHECKER_CELL`].
    pub checker_cell: f32,
    /// Checkerboard cell that holds the corners: PhotoCraft's light grey in every theme.
    pub checker_a: Rgba,
    /// The other checkerboard cell: PhotoCraft's mid grey.
    pub checker_b: Rgba,
    /// Selection ring, drawn inside the well's outline: `accent.solid`. The keyboard focus ring
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

    /// A ring of black or white at [`values::SWATCH_INNER_EDGE_ALPHA`] that stands out from `color`
    /// (straight alpha, over the checkerboard): for compositions that draw a well apart from its
    /// colours, such as the colour pair's ring.
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
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { SwatchTokens;
        "size.sm" 0.0..=1024.0 => size.sm;
        "size.md" 0.0..=1024.0 => size.md;
        "selected_width" 0.0..=16.0 => selected_width;
        "checker_cell" 0.0..=1024.0 => checker_cell;
    };

    fn defaults(s: &Semantic<'_>) -> SwatchTokens {
        let (c, size) = (s.color, s.size);
        let (checker_a, checker_b) = values::SWATCH_CHECKER;
        SwatchTokens {
            size: BySize { sm: size.control.sm, md: size.control.md },
            radius: s.shape.radius.control.min(values::SWATCH_RADIUS_MAX),
            selected_width: values::SWATCH_SELECTED_WIDTH,
            edge: c.line.control_edge,
            checker_cell: values::SWATCH_CHECKER_CELL,
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
    /// of a mixed value: 3:1 (4.5:1 with high contrast). The edge, which only shows the bounds,
    /// is a design target.
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
