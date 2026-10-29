//! Curve editors: the graph of a tone curve, its points and its gradient bars. Sizes scale with
//! the Size preset; the colours are overridable.

use super::{ComponentField, ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::color::Rgba;
use crate::tokens::{ColorToken as T, TokenRef};
use crate::values;

/// Curve editor tokens. Sizes are in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct CurveTokens {
    /// Least side of the graph, gradient bars included: `curve.min_side`.
    pub min_side: f32,
    /// Greatest side of the graph, gradient bars included: `curve.max_side`.
    pub max_side: f32,
    /// Inset of the value area inside the plot: `curve.pad`.
    pub pad: f32,
    /// Thickness of the gradient bars: `curve.bar`.
    pub bar: f32,
    /// Between the plot and the gradient bars: `curve.bar_gap`.
    pub bar_gap: f32,
    /// Radius around a point's centre that takes a press: `curve.hit`.
    pub hit: f32,
    /// How far outside the plot a dragged interior point is removed: `curve.drag_off`.
    pub drag_off: f32,
    /// Width of the edited curve: `curve.width`.
    pub width: f32,
    /// Width of the other channels' curves: `curve.other_width`.
    pub other_width: f32,
    /// Opacity of the other channels' curves, 0 to 1: `curve.other_alpha`.
    pub other_alpha: f32,
    /// The plot's fill: `field.fill.rest`.
    pub fill: Rgba,
    /// The plot's border: `field.border.rest`.
    pub border: Rgba,
    /// The quarter grid: `line.divider`.
    pub grid: Rgba,
    /// The diagonal of the unchanged curve: `line.border`.
    pub baseline: Rgba,
    /// An unselected point's inside, and the outline round a selected one: `field.fill.rest`.
    pub point_fill: Rgba,
    /// An unselected point's outline, a selected point's inside and a neutral channel's curve:
    /// `text.normal`.
    pub point_stroke: Rgba,
}

/// An overridable curve field. Paths: `fill`, `border`, `grid`, `baseline`, `point_fill`,
/// `point_stroke`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CurveField {
    /// `curve.fill`
    Fill,
    /// `curve.border`
    Border,
    /// `curve.grid`
    Grid,
    /// `curve.baseline`
    Baseline,
    /// `curve.point_fill`
    PointFill,
    /// `curve.point_stroke`
    PointStroke,
}

impl CurveField {
    const ALL: [CurveField; 6] =
        [CurveField::Fill, CurveField::Border, CurveField::Grid, CurveField::Baseline, CurveField::PointFill, CurveField::PointStroke];

    const fn name(self) -> &'static str {
        match self {
            CurveField::Fill => "fill",
            CurveField::Border => "border",
            CurveField::Grid => "grid",
            CurveField::Baseline => "baseline",
            CurveField::PointFill => "point_fill",
            CurveField::PointStroke => "point_stroke",
        }
    }
}

impl CurveTokens {
    fn color(&self, f: CurveField) -> Rgba {
        match f {
            CurveField::Fill => self.fill,
            CurveField::Border => self.border,
            CurveField::Grid => self.grid,
            CurveField::Baseline => self.baseline,
            CurveField::PointFill => self.point_fill,
            CurveField::PointStroke => self.point_stroke,
        }
    }

    fn color_mut(&mut self, f: CurveField) -> &mut Rgba {
        match f {
            CurveField::Fill => &mut self.fill,
            CurveField::Border => &mut self.border,
            CurveField::Grid => &mut self.grid,
            CurveField::Baseline => &mut self.baseline,
            CurveField::PointFill => &mut self.point_fill,
            CurveField::PointStroke => &mut self.point_stroke,
        }
    }
}

impl Family for CurveTokens {
    type Field = CurveField;
    const FIELDS: &'static [CurveField] = &CurveField::ALL;

    /// The Compact sizes (PhotoCraft's Curves), scaled by the control height of the size and
    /// rounded to whole points; widths and the opacity stay as they are.
    fn defaults(s: &Semantic<'_>) -> CurveTokens {
        let k = s.size.control.md / values::CURVE_BASE_CONTROL;
        let scale = |w: f32| (w * k).round();
        let c = s.color;
        // `field.fill.rest` and `field.border.rest`, which the field family takes from these.
        let (field_fill, field_border) = (c.surface.sunken, c.line.control_edge);
        CurveTokens {
            min_side: scale(values::CURVE_MIN_SIDE),
            max_side: scale(values::CURVE_MAX_SIDE),
            pad: scale(values::CURVE_PAD),
            bar: scale(values::CURVE_BAR),
            bar_gap: scale(values::CURVE_BAR_GAP),
            hit: scale(values::CURVE_HIT),
            drag_off: scale(values::CURVE_DRAG_OFF),
            width: values::CURVE_WIDTH,
            other_width: values::CURVE_OTHER_WIDTH,
            other_alpha: values::CURVE_OTHER_ALPHA,
            fill: field_fill,
            border: field_border,
            grid: c.line.divider,
            baseline: c.line.border,
            point_fill: field_fill,
            point_stroke: c.text.normal,
        }
    }

    fn path(f: CurveField) -> String {
        f.name().to_owned()
    }

    fn from_path(path: &str) -> Option<CurveField> {
        CurveField::ALL.into_iter().find(|f| f.name() == path)
    }

    fn kind(_: CurveField) -> FieldKind {
        FieldKind::Color
    }

    fn get(&self, f: CurveField) -> OverrideValue {
        OverrideValue::Color(self.color(f))
    }

    fn set(&mut self, f: CurveField, v: OverrideValue) {
        if let OverrideValue::Color(c) = v {
            *self.color_mut(f) = c;
        }
    }

    /// The cues that show the editor's state on the plot (WCAG 1.4.11), 3:1 and 4.5:1 with high
    /// contrast: the point outline, which draws the neutral curve, the unselected points' edges
    /// and the selected point; and the channel colours (`data.*`) a channel's curve is drawn in.
    /// High contrast moves `curve.fill` for a channel colour, which stays as the data rule left it.
    fn pairs() -> Vec<ContrastPair> {
        let fill = TokenRef::Component(ComponentField::Curve(CurveField::Fill));
        let pair = |fg| ContrastPair { fg, bg: fill, minimum: values::NON_TEXT_MIN, alternate: None };
        let channels = [T::DataRed, T::DataGreen, T::DataBlue, T::DataCyan, T::DataMagenta, T::DataYellow];
        std::iter::once(pair(TokenRef::Component(ComponentField::Curve(CurveField::PointStroke))))
            .chain(channels.map(|t| pair(TokenRef::Color(t))))
            .collect()
    }
}
