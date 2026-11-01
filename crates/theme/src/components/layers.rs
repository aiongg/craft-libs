//! The Layers panel: the sizes of its indicators, mask badges, blend rows and filter bar, and the
//! colours it draws itself. Rows, fills and the footer are the list and panel footer families'.
//! No theme overrides it.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
use crate::values;

/// Layers tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct LayersTokens {
    /// Box of an indicator icon (lock, link, warning, Smart Filter badge):
    /// [`values::LAYERS_INDICATOR_ICON`].
    pub indicator_icon: f32,
    /// Width of the effects triangle: [`values::LAYERS_INDICATOR_TRIANGLE`].
    pub indicator_triangle: f32,
    /// The `fx` mark: `layers.indicator.fx`, `text.muted`.
    pub indicator_fx: Rgba,
    /// The blend label: `layers.indicator.blend`, `text.faint`.
    pub indicator_blend: Rgba,
    /// The lock indicator: `layers.indicator.lock`, `text.faint`.
    pub indicator_lock: Rgba,
    /// The link indicator: `layers.indicator.link`, `text.faint`.
    pub indicator_link: Rgba,
    /// The warning indicator: `layers.indicator.warn`, `status.warn.fg`.
    pub indicator_warn: Rgba,
    /// Box of a mask badge: [`values::LAYERS_MASK_BADGE`].
    pub mask_badge: f32,
    /// A mask badge's fill: `layers.mask.badge_fill`, `surface.raised`.
    pub mask_badge_fill: Rgba,
    /// A mask badge's glyph: `layers.mask.badge_glyph`, `text.normal`.
    pub mask_badge_glyph: Rgba,
    /// The cross over a disabled mask: `layers.mask.disabled`, `status.bad.fg`.
    pub mask_disabled: Rgba,
    /// Least width of the blend select before the "Opacity:" label hides:
    /// [`values::LAYERS_BLEND_MIN_WIDTH`].
    pub blend_min_width: f32,
    /// Width of the Opacity and Fill fields, without the pop-up arrow:
    /// [`values::LAYERS_PERCENT_FIELD_WIDTH`].
    pub percent_field_width: f32,
    /// Between the filter bar's controls: `space.gap.inline`.
    pub filter_gap: f32,
    /// Width of the filter type select: [`values::LAYERS_FILTER_TYPE_WIDTH`].
    pub filter_type_width: f32,
    /// Panel Options' sample side as a factor of the row thumbnail:
    /// [`values::LAYERS_OPTIONS_SAMPLE_SCALE`].
    pub options_sample_scale: f32,
}

/// Layers have no overridable field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LayersField {}

impl Family for LayersTokens {
    type Field = LayersField;
    const FIELDS: &'static [LayersField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { LayersTokens;
        "indicator.icon" 8.0..=32.0 => indicator_icon;
        "indicator.triangle" 4.0..=32.0 => indicator_triangle;
        "mask.badge" 4.0..=32.0 => mask_badge;
        "blend.min_width" 32.0..=240.0 => blend_min_width;
        "percent_field_width" 32.0..=240.0 => percent_field_width;
        "filter.gap" 0.0..=32.0 => filter_gap;
        "filter.type_width" 32.0..=240.0 => filter_type_width;
        "options.sample_scale" 1.0..=4.0 => options_sample_scale;
    };

    fn defaults(s: &Semantic<'_>) -> LayersTokens {
        let c = s.color;
        LayersTokens {
            indicator_icon: values::LAYERS_INDICATOR_ICON,
            indicator_triangle: values::LAYERS_INDICATOR_TRIANGLE,
            indicator_fx: c.text.muted,
            indicator_blend: c.text.faint,
            indicator_lock: c.text.faint,
            indicator_link: c.text.faint,
            indicator_warn: c.status.warn.fg,
            mask_badge: values::LAYERS_MASK_BADGE,
            mask_badge_fill: c.surface.raised,
            mask_badge_glyph: c.text.normal,
            mask_disabled: c.status.bad.fg,
            blend_min_width: values::LAYERS_BLEND_MIN_WIDTH,
            percent_field_width: values::LAYERS_PERCENT_FIELD_WIDTH,
            filter_gap: s.space.gap.inline,
            filter_type_width: values::LAYERS_FILTER_TYPE_WIDTH,
            options_sample_scale: values::LAYERS_OPTIONS_SAMPLE_SCALE,
        }
    }

    fn path(f: LayersField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<LayersField> {
        None
    }

    fn kind(f: LayersField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: LayersField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: LayersField, _: OverrideValue) {
        match f {}
    }

    // The indicators are glyph cues drawn over list rows whose fills the list family checks.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
