//! Marquees: marching ants, selection bands and outlines being drawn. Their colours are the
//! semantic `overlay.marquee` and `overlay.marquee_dash`; the family holds their geometry and
//! timing, the same at every Size because marquees sit on the document.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Marquee tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct MarqueeTokens {
    /// Length of the ants' dashes and gaps, in line widths: `marquee.dash`.
    pub dash: f32,
    /// How fast the ants march, in line widths per second: `marquee.speed`.
    pub speed: f32,
    /// Milliseconds between the ants' repaints: `marquee.tick_ms`.
    pub tick_ms: u32,
    /// Dash length of a selection band, in points: `marquee.band_dash`.
    pub band_dash: f32,
    /// Gap between a band's dashes: `marquee.band_gap`.
    pub band_gap: f32,
    /// Width of the light line under an outline being drawn: `marquee.drawing_halo`.
    pub drawing_halo: f32,
    /// Farthest a press may move and still be a click, in points: `marquee.click_slop`.
    pub click_slop: f32,
}

/// Marquees have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MarqueeField {}

impl Family for MarqueeTokens {
    type Field = MarqueeField;
    const FIELDS: &'static [MarqueeField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { MarqueeTokens;
        "dash" 0.0..=16.0 => dash;
        "band_dash" 0.0..=16.0 => band_dash;
        "band_gap" 0.0..=64.0 => band_gap;
        "drawing_halo" 0.0..=16.0 => drawing_halo;
        "click_slop" 0.0..=64.0 => click_slop;
    };

    fn defaults(_: &Semantic<'_>) -> MarqueeTokens {
        MarqueeTokens {
            dash: values::MARQUEE_DASH,
            speed: values::MARQUEE_SPEED,
            tick_ms: values::MARQUEE_TICK_MS,
            band_dash: values::MARQUEE_BAND_DASH,
            band_gap: values::MARQUEE_BAND_GAP,
            drawing_halo: values::MARQUEE_DRAWING_HALO,
            click_slop: values::MARQUEE_CLICK_SLOP,
        }
    }

    fn path(f: MarqueeField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<MarqueeField> {
        None
    }

    fn kind(f: MarqueeField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: MarqueeField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: MarqueeField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own: it draws in semantic tokens.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
