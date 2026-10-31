//! Color Sampler points: the reticle drawn on the canvas at each numbered point. The sizes are the
//! same at every Size, as the canvas overlays are; the colours are the semantic `overlay.*` tokens.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Color Sampler tokens, in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct SamplerTokens {
    /// Radius of the reticle's circle: `sampler.marker_radius`.
    pub marker_radius: f32,
    /// Distance from the point at which each of the four ticks starts: `sampler.tick_start`.
    pub tick_start: f32,
    /// Distance from the point at which each tick ends: `sampler.tick_end`.
    pub tick_end: f32,
}

/// Samplers have no overridable field: their sizes come from the code, as every size does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SamplerField {}

impl Family for SamplerTokens {
    type Field = SamplerField;
    const FIELDS: &'static [SamplerField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { SamplerTokens;
        "marker_radius" 0.0..=64.0 => marker_radius;
        "tick_start" 0.0..=64.0 => tick_start;
        "tick_end" 0.0..=64.0 => tick_end;
    };

    fn defaults(_: &Semantic<'_>) -> SamplerTokens {
        SamplerTokens { marker_radius: values::SAMPLER_MARKER_RADIUS, tick_start: values::SAMPLER_TICK_START, tick_end: values::SAMPLER_TICK_END }
    }

    fn path(f: SamplerField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<SamplerField> {
        None
    }

    fn kind(f: SamplerField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: SamplerField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: SamplerField, _: OverrideValue) {
        match f {}
    }

    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
