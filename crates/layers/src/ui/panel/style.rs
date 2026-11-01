//! `LayersStyle` from the `layers` tokens.

use craft_ui::theme::{ToEgui as _, UiTokens};
use egui::Color32;

/// The Layers panel's own sizes and colours, besides the list's (`list.*`, from
/// [`ListRow::style`](craft_ui::list::ListRow::style)) and the footer's. A pure function of the
/// tokens ([`LayersStyle::of`]).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct LayersStyle {
    /// The box of an indicator icon (lock, link, warning, Smart Filter badge):
    /// `layers.indicator.icon`.
    pub indicator_icon: f32,
    /// The effects triangle's width: `layers.indicator.triangle`.
    pub indicator_triangle: f32,
    /// The `fx` mark (type role `caption`, semibold): `layers.indicator.fx`.
    pub fx: Color32,
    /// The blend label (type role `caption`): `layers.indicator.blend`.
    pub blend: Color32,
    /// The lock indicator: `layers.indicator.lock`.
    pub lock: Color32,
    /// The link indicator: `layers.indicator.link`.
    pub link: Color32,
    /// The warning indicator: `layers.indicator.warn`.
    pub warn: Color32,
    /// A mask badge's box: `layers.mask.badge` (its fill and glyph colours below).
    pub mask_badge: f32,
    /// A mask badge's fill.
    pub mask_badge_fill: Color32,
    /// A mask badge's glyph.
    pub mask_badge_glyph: Color32,
    /// The cross over a disabled mask: `layers.mask.disabled`.
    pub mask_disabled: Color32,
    /// The least width of the blend select before the "Opacity:" label hides:
    /// `layers.blend.min_width`.
    pub blend_min_width: f32,
    /// The Opacity and Fill fields' width, without the pop-up arrow: `layers.percent_field_width`.
    pub percent_field_width: f32,
    /// Between the filter bar's controls: `layers.filter.gap`.
    pub filter_gap: f32,
    /// The filter type select's width: `layers.filter.type_width`.
    pub filter_type_width: f32,
    /// Panel Options' sample side as a factor of the row thumbnail: `layers.options.sample_scale`.
    pub sample_scale: f32,
}

impl LayersStyle {
    /// The style of `tokens`: the `layers` family.
    pub fn of(tokens: &UiTokens) -> LayersStyle {
        let l = &tokens.component.layers;
        LayersStyle {
            indicator_icon: l.indicator_icon,
            indicator_triangle: l.indicator_triangle,
            fx: l.indicator_fx.to_egui(),
            blend: l.indicator_blend.to_egui(),
            lock: l.indicator_lock.to_egui(),
            link: l.indicator_link.to_egui(),
            warn: l.indicator_warn.to_egui(),
            mask_badge: l.mask_badge,
            mask_badge_fill: l.mask_badge_fill.to_egui(),
            mask_badge_glyph: l.mask_badge_glyph.to_egui(),
            mask_disabled: l.mask_disabled.to_egui(),
            blend_min_width: l.blend_min_width,
            percent_field_width: l.percent_field_width,
            filter_gap: l.filter_gap,
            filter_type_width: l.filter_type_width,
            sample_scale: l.options_sample_scale,
        }
    }
}
