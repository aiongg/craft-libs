//! Histograms: the plot height, the backdrop behind a curve, the outline width and the marker of the tone and RGB looks. The channel colours are the `data.*` tokens.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::color::Rgba;
use crate::values;

/// Histogram tokens, in points except the alphas.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct HistogramTokens {
    /// Height of the plot: `histogram.height`.
    pub height: f32,
    /// The tone look behind a curve, translucent grey: `histogram.backdrop`.
    pub backdrop: Rgba,
    /// Width of the RGB ribbons' outlines: `histogram.line`.
    pub line: f32,
    /// Alpha of the selected range's band, in the accent colour: `histogram.selection_alpha`.
    pub selection_alpha: f32,
    /// Alpha of the marker band at a level: `histogram.marker_alpha`.
    pub marker_alpha: f32,
    /// Width of the marker band: `histogram.marker_width`.
    pub marker_width: f32,
}

/// An overridable histogram field. Path: `backdrop`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HistogramField {
    /// `histogram.backdrop`
    Backdrop,
}

impl Family for HistogramTokens {
    type Field = HistogramField;
    const FIELDS: &'static [HistogramField] = &[HistogramField::Backdrop];

    /// The Compact height (PhotoCraft's panel), scaled by the control height of the size and rounded to whole points.
    fn defaults(s: &Semantic<'_>) -> HistogramTokens {
        let k = s.size.control.md / values::HISTOGRAM_BASE_CONTROL;
        HistogramTokens {
            height: (values::HISTOGRAM_HEIGHT * k).round(),
            backdrop: values::HISTOGRAM_BACKDROP,
            line: values::HISTOGRAM_LINE,
            selection_alpha: values::HISTOGRAM_SELECTION_ALPHA,
            marker_alpha: values::HISTOGRAM_MARKER_ALPHA,
            marker_width: values::HISTOGRAM_MARKER_WIDTH,
        }
    }

    fn path(_: HistogramField) -> String {
        "backdrop".to_owned()
    }

    fn from_path(path: &str) -> Option<HistogramField> {
        (path == "backdrop").then_some(HistogramField::Backdrop)
    }

    fn kind(_: HistogramField) -> FieldKind {
        FieldKind::Color
    }

    fn get(&self, _: HistogramField) -> OverrideValue {
        OverrideValue::Color(self.backdrop)
    }

    fn set(&mut self, _: HistogramField, v: OverrideValue) {
        if let OverrideValue::Color(c) = v {
            self.backdrop = c;
        }
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
