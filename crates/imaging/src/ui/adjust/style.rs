//! [`AdjustStyle`]: the adjustment editors' own sizes and colours, from the `adjust.*` tokens
//!. Sliders, histograms, Levels and Curves keep their own component tokens.
//!
//! Image tones stay constants in code, not tokens: the picker dots' greys (0, 128, 255) and the
//! two-colour slider tracks show what the adjustment does to the image, as the Levels thumb faces
//! do.

use egui::{Color32, FontId, vec2};

use craft_theme::TypeRole;
use craft_ui::theme::{ToEgui, UiTokens};

/// What the adjustment editors draw besides their controls.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct AdjustStyle {
    /// A Hue/Saturation spectrum bar's height: `adjust.spectrum_bar` (10, PhotoCraft
    /// `hue_saturation.rs:149-150`).
    pub spectrum_bar: f32,
    /// The range band's height between the bars: `adjust.range_band` (6, `:166`, mid ± 3).
    pub range_band: f32,
    /// The height between the two bars that holds the range thumbs: `adjust.spectrum_master_extra`
    /// (20: the 40 high area, `:147`, less the two bars).
    pub spectrum_gap: f32,
    /// A range thumb: 5 × 12 (`:177`), 1 pt edge.
    pub range_thumb: egui::Vec2,
    /// A range thumb's corner radius: `adjust.range_thumb_radius`.
    pub range_thumb_radius: f32,
    /// How close a press must come to a range thumb to take it: `adjust.hue_grab`.
    pub hue_grab: f32,
    /// The range band's outer (fall-off) and inner (fully affected) fills: `text.faint` and
    /// `text.muted` (`:172-173`).
    pub band_outer: Color32,
    #[allow(missing_docs)]
    pub band_inner: Color32,
    /// A range thumb's fill, inner thumbs and outer thumbs: `text.normal` and `surface.sunken`;
    /// its edge `text.muted` (`:176-178`).
    pub thumb_inner: Color32,
    #[allow(missing_docs)]
    pub thumb_outer: Color32,
    #[allow(missing_docs)]
    pub thumb_edge: Color32,
    /// The degree readout under the bars: `type.mono` in `text.faint` (`:207`).
    pub degrees_font: FontId,
    #[allow(missing_docs)]
    pub degrees_color: Color32,
    /// A point picker's tone dot: radius `adjust.picker_dot` (3.5) centred `adjust.picker_dot_inset`
    /// (5.5) in from the button's bottom-trailing corner, with a 1 pt ring in `adjust.picker_ring`
    /// (PhotoCraft `adjust_dialog.rs:180-181`), faded by `adjust.disabled_alpha` on a disabled
    /// button.
    pub picker_dot: f32,
    #[allow(missing_docs)]
    pub picker_dot_inset: f32,
    #[allow(missing_docs)]
    pub picker_ring: Color32,
    #[allow(missing_docs)]
    pub disabled_alpha: f32,
    /// Threshold's histogram: `adjust.threshold_histogram` high (90) and at most
    /// `adjust.threshold_width` wide (300) (`basic.rs:73-74`); the level marker 1 pt in
    /// `accent.strong` (`:83`); the marker's travel is `adjust.threshold_inset` in from each edge
    /// (`:78`).
    pub threshold_histogram: f32,
    #[allow(missing_docs)]
    pub threshold_width: f32,
    #[allow(missing_docs)]
    pub threshold_inset: f32,
    #[allow(missing_docs)]
    pub threshold_marker_width: f32,
    #[allow(missing_docs)]
    pub threshold_marker: Color32,
    /// Channel Mixer's Total readout: `type.mono`, `text.normal`, and `status.warn.fg` above 100 %
    /// (`basic.rs:271`).
    pub total_font: FontId,
    #[allow(missing_docs)]
    pub total_color: Color32,
    #[allow(missing_docs)]
    pub total_warn: Color32,
    /// The Curves hint and the "Unavailable in this color mode" note: `type.caption` in
    /// `text.faint` (`curves.rs:402`, `adjust_dialog.rs:206`).
    pub note_font: FontId,
    #[allow(missing_docs)]
    pub note_color: Color32,
}

impl AdjustStyle {
    /// The adjustment editors' look in `tokens`.
    pub fn of(tokens: &UiTokens) -> AdjustStyle {
        // pc: crates/ui-egui/src/dialogs/adjust_editors/hue_saturation.rs:147-207 (bars, band,
        // thumbs), crates/ui-egui/src/dialogs/adjust_dialog.rs:177-183 `picker_button`,
        // crates/ui-egui/src/dialogs/adjust_editors/basic.rs:71-85 (threshold), :271 (Total).
        let (a, c) = (&tokens.component.adjust, &tokens.color);
        AdjustStyle {
            spectrum_bar: a.spectrum_bar,
            range_band: a.range_band,
            spectrum_gap: a.spectrum_master_extra,
            range_thumb: vec2(a.range_thumb_width, a.range_thumb_height),
            range_thumb_radius: a.range_thumb_radius,
            hue_grab: a.hue_grab,
            band_outer: c.text.faint.to_egui(),
            band_inner: c.text.muted.to_egui(),
            thumb_inner: c.text.normal.to_egui(),
            thumb_outer: c.surface.sunken.to_egui(),
            thumb_edge: c.text.muted.to_egui(),
            degrees_font: tokens.font(TypeRole::Mono),
            degrees_color: c.text.faint.to_egui(),
            picker_dot: a.picker_dot,
            picker_dot_inset: a.picker_dot_inset,
            picker_ring: a.picker_ring.to_egui(),
            disabled_alpha: a.disabled_alpha,
            threshold_histogram: a.threshold_histogram,
            threshold_width: a.threshold_width,
            threshold_inset: a.threshold_inset,
            threshold_marker_width: a.threshold_marker_width,
            threshold_marker: c.accent.strong.to_egui(),
            total_font: tokens.font(TypeRole::Mono),
            total_color: c.text.normal.to_egui(),
            total_warn: c.status.warn.fg.to_egui(),
            note_font: tokens.font(TypeRole::Caption),
            note_color: c.text.faint.to_egui(),
        }
    }
}
