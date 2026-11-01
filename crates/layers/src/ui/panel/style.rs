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
    /// The style of `tokens`. Until the `layers` token family exists (task TK1), the sizes are
    /// PhotoCraft's (`layer_row_ui.rs:20-28`, `layers.rs:39-46`, `layer_filter_ui.rs:124-126`,
    /// `mask_thumbs_ui.rs:142-154`, `options_bar.rs:684`) and the colours come from the tokens they will derive from.
    pub fn of(tokens: &UiTokens) -> LayersStyle {
        let color = &tokens.color;
        LayersStyle {
            indicator_icon: 14.0,
            indicator_triangle: 10.0,
            fx: color.text.muted.to_egui(),
            blend: color.text.faint.to_egui(),
            lock: color.text.faint.to_egui(),
            link: color.text.faint.to_egui(),
            warn: color.status.warn.fg.to_egui(),
            mask_badge: 9.0,
            mask_badge_fill: color.surface.raised.to_egui(),
            mask_badge_glyph: color.text.normal.to_egui(),
            mask_disabled: color.status.bad.fg.to_egui(),
            blend_min_width: 80.0,
            percent_field_width: 66.0,
            filter_gap: tokens.space.gap.inline,
            filter_type_width: 64.0,
            sample_scale: 2.0,
        }
    }
}
