//! The scope panel: the sizes and ratios it adds to the histogram's plot. Colours come from the
//! histogram and data tokens. No theme overrides it.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::values;

/// Scopes tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ScopesTokens {
    /// A clipping warning button's side: [`values::SCOPES_WARNING_SIZE`].
    pub warning_size: f32,
    /// The gap between the statistics columns: [`values::SCOPES_STATISTICS_GAP`].
    pub statistics_gap: f32,
    /// Each per-channel plot of All Channels View: [`values::SCOPES_ALL_CHANNELS_HEIGHT`].
    pub all_channels_height: f32,
    /// The vectorscope rings' alpha on `text.faint`: [`values::SCOPES_VECTOR_RING_ALPHA`].
    pub vector_ring_alpha: f32,
    /// The smallest half-side of a cloud dot: [`values::SCOPES_VECTOR_DOT_MIN`].
    pub vector_dot_min: f32,
    /// The largest half-side of a cloud dot: [`values::SCOPES_VECTOR_DOT_MAX`].
    pub vector_dot_max: f32,
    /// The hover readout badge's height: [`values::SCOPES_HOVER_BADGE_HEIGHT`].
    pub hover_badge_height: f32,
    /// The hovered zone band's alpha on `text.normal`: [`values::SCOPES_ZONE_ALPHA`].
    pub zone_alpha: f32,
    /// The vectorscope's smallest default height: [`values::SCOPES_VECTOR_HEIGHT_MIN`].
    pub vector_height_min: f32,
    /// The vectorscope's largest default height: [`values::SCOPES_VECTOR_HEIGHT_MAX`].
    pub vector_height_max: f32,
    /// The vectorscope's margin between its outer ring and the plot's edge:
    /// [`values::SCOPES_VECTOR_MARGIN`].
    pub vector_margin: f32,
    /// The inset of a clipping warning's triangle in its button: [`values::SCOPES_WARNING_INSET`].
    pub warning_inset: f32,
}

/// Scopes have no overridable field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ScopesField {}

impl Family for ScopesTokens {
    type Field = ScopesField;
    const FIELDS: &'static [ScopesField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { ScopesTokens;
        "warning_size" 8.0..=64.0 => warning_size;
        "statistics_gap" 0.0..=32.0 => statistics_gap;
        "all_channels_height" 24.0..=400.0 => all_channels_height;
        "vector_ring_alpha" 0.0..=1.0 => vector_ring_alpha;
        "vector_dot_min" 0.1..=8.0 => vector_dot_min;
        "vector_dot_max" 0.1..=16.0 => vector_dot_max;
        "hover_badge_height" 8.0..=64.0 => hover_badge_height;
        "zone_alpha" 0.0..=1.0 => zone_alpha;
        "vector_height_min" 24.0..=400.0 => vector_height_min;
        "vector_height_max" 24.0..=800.0 => vector_height_max;
        "vector_margin" 0.0..=48.0 => vector_margin;
        "warning_inset" 0.0..=16.0 => warning_inset;
    };

    fn defaults(_: &Semantic<'_>) -> ScopesTokens {
        ScopesTokens {
            warning_size: values::SCOPES_WARNING_SIZE,
            statistics_gap: values::SCOPES_STATISTICS_GAP,
            all_channels_height: values::SCOPES_ALL_CHANNELS_HEIGHT,
            vector_ring_alpha: values::SCOPES_VECTOR_RING_ALPHA,
            vector_dot_min: values::SCOPES_VECTOR_DOT_MIN,
            vector_dot_max: values::SCOPES_VECTOR_DOT_MAX,
            hover_badge_height: values::SCOPES_HOVER_BADGE_HEIGHT,
            zone_alpha: values::SCOPES_ZONE_ALPHA,
            vector_height_min: values::SCOPES_VECTOR_HEIGHT_MIN,
            vector_height_max: values::SCOPES_VECTOR_HEIGHT_MAX,
            vector_margin: values::SCOPES_VECTOR_MARGIN,
            warning_inset: values::SCOPES_WARNING_INSET,
        }
    }

    fn path(f: ScopesField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<ScopesField> {
        None
    }

    fn kind(f: ScopesField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: ScopesField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: ScopesField, _: OverrideValue) {
        match f {}
    }

    // No colours of its own.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
