//! Style of the `ruler` family.

use egui::{Color32, FontId, Stroke};

use crate::theme::{ToEgui as _, UiTokens};

const LABEL_PITCH: f32 = 8.5;
const STEP_MIN: f32 = 60.0;
const TICK_MID: f32 = 6.0;
const TICK_MINOR: f32 = 3.0;
const MARKER_HALF: f32 = 4.5;
const MARKER_COLUMN: f32 = 7.0;
const MARKER_TIP: f32 = 5.0;
const MARKER_BAR: f32 = 4.0;
const MARKER_STEM: f32 = 5.0;
const MARKER_FOOT: f32 = 4.0;
const MARKER_SELECTOR: f32 = 14.0;

/// The rulers' look, from [`Rulers::style`](crate::canvas::Rulers::style).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct RulerStyle {
    /// Thickness of a strip and side of the corner: `size.icon.sm`.
    pub size: f32,
    /// The labels' font: the UI face at `typography.caption.size`.
    pub font: FontId,
    /// Between a tick and its label: `space.gap.tight`.
    pub label_pad: f32,
    /// The strips and the corner: `surface.chrome`.
    pub fill: Color32,
    /// `stroke.thin` of `text.faint`.
    pub tick: Stroke,
    /// Labels: `text.muted`.
    pub label: Color32,
    /// The pointer's marker and the cross hairs of a zero point drag: `stroke.thin` of `text.normal`.
    pub marker: Stroke,
    /// The margin part of a strip, where the text part keeps `fill`: `surface.sunken`.
    pub margin_shade: Color32,
    /// Between the characters of the left ruler's stacked labels.
    pub label_pitch: f32,
    /// Least distance between labelled ticks.
    pub step_min: f32,
    /// Length of the tick halfway between labelled ticks.
    pub tick_mid: f32,
    /// Length of the other minor ticks.
    pub tick_minor: f32,
    /// Half the width of a text-ruler marker.
    pub marker_half: f32,
    /// Width of a table-ruler column marker.
    pub marker_column: f32,
    /// Height of a text-ruler indent triangle.
    pub marker_tip: f32,
    /// Height of the left indent's box under its triangle.
    pub marker_bar: f32,
    /// Height of a tab stop's stem.
    pub marker_stem: f32,
    /// Length of a tab stop's foot, to either side of the stem.
    pub marker_foot: f32,
    /// Side of the tab selector's box.
    pub marker_selector: f32,
}

impl RulerStyle {
    /// The ruler style for `tokens`.
    pub fn new(tokens: &UiTokens) -> Self {
        let c = &tokens.color;
        let family = tokens.families().family(craft_theme::FontSlot::Ui).clone();
        Self {
            size: tokens.size.icon.sm,
            font: FontId::new(tokens.typography.caption.size, family),
            label_pad: tokens.space.gap.tight,
            fill: c.surface.chrome.to_egui(),
            tick: Stroke::new(tokens.stroke.thin, c.text.faint.to_egui()),
            label: c.text.muted.to_egui(),
            marker: Stroke::new(tokens.stroke.thin, c.text.normal.to_egui()),
            margin_shade: c.surface.sunken.to_egui(),
            label_pitch: LABEL_PITCH,
            step_min: STEP_MIN,
            tick_mid: TICK_MID,
            tick_minor: TICK_MINOR,
            marker_half: MARKER_HALF,
            marker_column: MARKER_COLUMN,
            marker_tip: MARKER_TIP,
            marker_bar: MARKER_BAR,
            marker_stem: MARKER_STEM,
            marker_foot: MARKER_FOOT,
            marker_selector: MARKER_SELECTOR,
        }
    }
}
