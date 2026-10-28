//! Histograms: the plot height, fill, outline and marker of the tone and RGB looks. Sizes and alphas only (no colours): no theme overrides them.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic};
use crate::values;

/// Histogram tokens, in points except the alphas.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct HistogramTokens {
    /// Height of the plot: `histogram.height`.
    pub height: f32,
    /// Alpha of the RGB ribbons' fills: `histogram.fill_alpha`.
    pub fill_alpha: f32,
    /// Alpha of the tone look behind a curve: `histogram.backdrop_alpha`.
    pub backdrop_alpha: f32,
    /// Width of the RGB ribbons' outlines: `histogram.line`.
    pub line: f32,
    /// Alpha of the marker band at a level: `histogram.marker_alpha`.
    pub marker_alpha: f32,
    /// Width of the marker band: `histogram.marker_width`.
    pub marker_width: f32,
}

/// No overridable field: the sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum HistogramField {}

impl Family for HistogramTokens {
    type Field = HistogramField;
    const FIELDS: &'static [HistogramField] = &[];

    /// The Compact height (PhotoCraft's panel), scaled by the control height of the size and rounded to whole points.
    fn defaults(s: &Semantic<'_>) -> HistogramTokens {
        let k = s.size.control.md / values::HISTOGRAM_BASE_CONTROL;
        HistogramTokens {
            height: (values::HISTOGRAM_HEIGHT * k).round(),
            fill_alpha: values::HISTOGRAM_FILL_ALPHA,
            backdrop_alpha: values::HISTOGRAM_BACKDROP_ALPHA,
            line: values::HISTOGRAM_LINE,
            marker_alpha: values::HISTOGRAM_MARKER_ALPHA,
            marker_width: values::HISTOGRAM_MARKER_WIDTH,
        }
    }

    fn path(f: HistogramField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<HistogramField> {
        None
    }

    fn kind(f: HistogramField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: HistogramField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: HistogramField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
