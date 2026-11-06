//! `LayersStyle` from the semantic tokens.

use craft_ui::theme::{ToEgui as _, UiTokens};
use egui::Color32;

const INDICATOR_TRIANGLE: f32 = 10.0;
const MASK_CROSS_INSET: f32 = 1.0;
const OPTIONS_SAMPLE_SCALE: f32 = 2.0;
const BLEND_LABEL_SLACK: f32 = 16.0;
const BLEND_FLOOR_WIDTH: f32 = 40.0;
/// The Type layer's kind tile ink, the same in every theme.
pub const DEFAULT_KIND_TYPE_INK: Color32 = Color32::from_rgb(0x28, 0x28, 0x28);

/// The Layers panel's own sizes and colours, besides the list's (`list.*`, from
/// [`ListRow::style`](craft_ui::list::ListRow::style)) and the footer's. A pure function of the
/// tokens ([`LayersPanel::style`](super::LayersPanel::style)).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct LayersStyle {
    /// The box of an indicator icon (lock, link, warning, Smart Filter badge).
    pub indicator_icon: f32,
    /// The effects triangle's width.
    pub indicator_triangle: f32,
    /// The `fx` mark (type role `caption`, semibold).
    pub fx: Color32,
    /// The blend label (type role `caption`).
    pub blend: Color32,
    /// The lock indicator.
    pub lock: Color32,
    /// The link indicator.
    pub link: Color32,
    /// The warning indicator.
    pub warn: Color32,
    /// A mask badge's box .
    pub mask_badge: f32,
    /// A mask badge's fill.
    pub mask_badge_fill: Color32,
    /// A mask badge's glyph.
    pub mask_badge_glyph: Color32,
    /// The cross over a disabled mask.
    pub mask_disabled: Color32,
    /// The least width of the blend select before the "Opacity:" label hides.
    pub blend_min_width: f32,
    /// The Opacity and Fill fields' text part, without the pop-up arrow. The box adds the number field's pop-up arrow.
    pub percent_field_width: f32,
    /// Between the filter bar's controls.
    pub filter_gap: f32,
    /// The filter type select's width.
    pub filter_type_width: f32,
    /// The space after the filter type select, besides `filter_gap`.
    pub filter_type_gap: f32,
    /// The room kept free between the "Opacity:" label and its field.
    pub blend_label_slack: f32,
    /// The blend select's least width when the row is too narrow for anything else.
    pub blend_floor_width: f32,
    /// Between the blend, Opacity, Lock and Fill rows.
    pub blend_row_gap: f32,
    /// The Type layer's kind tile ink.
    pub kind_type_ink: Color32,
    /// Between the effects triangle and the `fx` mark.
    pub indicator_fx_gap: f32,
    /// The stroke of the cross over a disabled mask.
    pub mask_cross_width: f32,
    /// The cross's inset from the thumbnail.
    pub mask_cross_inset: f32,
    /// The stroke of the Smart Filters rings.
    pub indicator_ring_width: f32,
    /// Panel Options' sample side as a factor of the row thumbnail.
    pub sample_scale: f32,
}

impl LayersStyle {
    /// The style of `tokens`: the `layers` family. [`LayersPanel::style`](super::LayersPanel::style)
    /// is the public form; the crate's own code, which has no tree type to name, calls this.
    pub(crate) fn of(tokens: &UiTokens) -> LayersStyle {
        let color = &tokens.color;
        let control = tokens.size.control.md;
        LayersStyle {
            indicator_icon: tokens.size.icon.sm,
            indicator_triangle: INDICATOR_TRIANGLE,
            fx: color.text.muted.to_egui(),
            blend: color.text.faint.to_egui(),
            lock: color.text.faint.to_egui(),
            link: color.text.faint.to_egui(),
            warn: color.status.warn.fg.to_egui(),
            mask_badge: tokens.size.icon.sm * 0.65,
            mask_badge_fill: color.surface.raised.to_egui(),
            mask_badge_glyph: color.text.normal.to_egui(),
            mask_disabled: color.status.bad.fg.to_egui(),
            blend_min_width: control * 3.0,
            percent_field_width: control * 2.5,
            filter_gap: tokens.space.gap.inline,
            filter_type_width: control * 2.5,
            filter_type_gap: tokens.space.gap.tight,
            blend_label_slack: BLEND_LABEL_SLACK,
            blend_floor_width: BLEND_FLOOR_WIDTH,
            blend_row_gap: tokens.space.gap.row,
            kind_type_ink: DEFAULT_KIND_TYPE_INK,
            indicator_fx_gap: tokens.stroke.thin,
            mask_cross_width: tokens.stroke.medium,
            mask_cross_inset: MASK_CROSS_INSET,
            indicator_ring_width: tokens.stroke.thin,
            sample_scale: OPTIONS_SAMPLE_SCALE,
        }
    }
}
