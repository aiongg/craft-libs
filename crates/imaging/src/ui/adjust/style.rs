//! [`AdjustStyle`]: the adjustment editors' own sizes and colours, from the semantic tokens.
//!
//! Image tones stay constants in code, not tokens: the picker dots' greys (0, 128, 255) and the
//! two-colour slider tracks show what the adjustment does to the image, as the Levels thumb faces
//! do.

use egui::{Color32, FontId, vec2};

use craft_theme::TypeRole;
use craft_ui::theme::{ToEgui, UiTokens};

/// The range band's height between the spectrum bars.
const RANGE_BAND_HEIGHT: f32 = 6.0;
/// A range thumb's width.
const RANGE_THUMB_WIDTH: f32 = 5.0;
/// How far a disabled picker's tone dot fades, as an alpha multiplier.
const DISABLED_ALPHA: f32 = 0.4;
/// A Threshold histogram's height, in control heights.
const THRESHOLD_HISTOGRAM_ROWS: f32 = 3.0;
/// A Threshold histogram's widest extent, in control heights.
const THRESHOLD_WIDTH_ROWS: f32 = 12.5;

/// What the adjustment editors draw besides their controls.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct AdjustStyle {
    /// A Hue/Saturation spectrum bar's height: `size.icon.sm`.
    pub spectrum_bar: f32,
    /// The range band's height between the bars: 6 (mid ± 3).
    pub range_band: f32,
    /// The height between the two bars that holds the range thumbs: `space.pad.dialog`.
    pub spectrum_gap: f32,
    /// A range thumb: 5 wide, `size.icon.sm` high, 1 pt edge.
    pub range_thumb: egui::Vec2,
    /// A range thumb's corner radius: `shape.radius.sm`.
    pub range_thumb_radius: f32,
    /// How close a press must come to a range thumb to take it: `size.hit.target`.
    pub hue_grab: f32,
    /// The range band's outer (fall-off) and inner (fully affected) fills: `text.faint` and
    /// `text.muted`.
    pub band_outer: Color32,
    #[allow(missing_docs)]
    pub band_inner: Color32,
    /// A range thumb's fill, inner thumbs and outer thumbs: `text.normal` and `surface.sunken`;
    /// its edge `text.muted`.
    pub thumb_inner: Color32,
    #[allow(missing_docs)]
    pub thumb_outer: Color32,
    #[allow(missing_docs)]
    pub thumb_edge: Color32,
    /// The degree readout under the bars: `type.mono` in `text.faint` (`:207`).
    pub degrees_font: FontId,
    #[allow(missing_docs)]
    pub degrees_color: Color32,
    /// A point picker's tone dot: radius `size.handle.sm / 2` centred `size.handle.md - 2.5` in from
    /// the button's bottom-trailing corner, with a 1 pt ring in `line.control_edge`, faded by an
    /// alpha of 0.4 on a disabled button.
    pub picker_dot: f32,
    #[allow(missing_docs)]
    pub picker_dot_inset: f32,
    #[allow(missing_docs)]
    pub picker_ring: Color32,
    #[allow(missing_docs)]
    pub disabled_alpha: f32,
    /// Threshold's histogram: 3 control heights high and at most 12.5 control heights wide; the level marker
    /// `stroke.thin` in `accent.strong`; the marker's travel is `space.gap.inline` in from each edge
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
        let c = &tokens.color;
        AdjustStyle {
            spectrum_bar: tokens.size.icon.sm,
            range_band: RANGE_BAND_HEIGHT,
            spectrum_gap: tokens.space.pad.dialog,
            range_thumb: vec2(RANGE_THUMB_WIDTH, tokens.size.icon.sm),
            range_thumb_radius: tokens.shape.radius.sm,
            hue_grab: tokens.size.hit.target,
            band_outer: c.text.faint.to_egui(),
            band_inner: c.text.muted.to_egui(),
            thumb_inner: c.text.normal.to_egui(),
            thumb_outer: c.surface.sunken.to_egui(),
            thumb_edge: c.text.muted.to_egui(),
            degrees_font: tokens.font(TypeRole::Mono),
            degrees_color: c.text.faint.to_egui(),
            picker_dot: tokens.size.handle.sm / 2.0,
            picker_dot_inset: tokens.size.handle.md - 2.5,
            picker_ring: c.line.control_edge.to_egui(),
            disabled_alpha: DISABLED_ALPHA,
            threshold_histogram: tokens.size.control.md * THRESHOLD_HISTOGRAM_ROWS,
            threshold_width: tokens.size.control.md * THRESHOLD_WIDTH_ROWS,
            threshold_inset: tokens.space.gap.inline,
            threshold_marker_width: tokens.stroke.thin,
            threshold_marker: c.accent.strong.to_egui(),
            total_font: tokens.font(TypeRole::Mono),
            total_color: c.text.normal.to_egui(),
            total_warn: c.status.warn.fg.to_egui(),
            note_font: tokens.font(TypeRole::Caption),
            note_color: c.text.faint.to_egui(),
        }
    }
}
