//! [`ScopePanelStyle`]: the scope panel's look, from tokens.

use craft_ui::theme::UiTokens;

use super::panel::ScopePanel;

/// A plot's height, in control heights.
const PLOT_ROWS: f32 = 4.2;
/// A clipping warning button's side.
const WARNING_SIZE: f32 = 18.0;
/// The gap between the statistics columns.
const STATISTICS_GAP: f32 = 6.0;
/// Each per-channel plot's height in All Channels View.
const ALL_CHANNELS_HEIGHT: f32 = 90.0;
/// The vectorscope rings' alpha on `text.faint`.
const VECTOR_RING_ALPHA: f32 = 0.45;
/// The smallest half-side of a cloud dot.
const VECTOR_DOT_MIN: f32 = 0.7;
/// The largest half-side of a cloud dot.
const VECTOR_DOT_MAX: f32 = 3.0;
/// The hover readout badge's height.
const HOVER_BADGE_HEIGHT: f32 = 20.0;
/// The hovered zone band's alpha on `text.normal`.
const ZONE_ALPHA: f32 = 0.055;
/// The vectorscope's smallest default height.
const VECTOR_HEIGHT_MIN: f32 = 100.0;
/// The vectorscope's largest default height.
const VECTOR_HEIGHT_MAX: f32 = 220.0;
/// The vectorscope's margin between its outer ring and the plot's edge.
const VECTOR_MARGIN: f32 = 14.0;
/// The inset of a clipping warning's triangle in its button.
const WARNING_INSET: f32 = 3.0;

/// The scope panel's sizes and ratios. Colours come from the data and semantic tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ScopePanelStyle {
    /// The Compact plot's height and the Expanded one's.
    pub plot_height: f32,
    /// A clipping warning button's side.
    pub warning_size: f32,
    /// The gap between the statistics columns.
    pub statistics_gap: f32,
    /// Each per-channel plot of All Channels View.
    pub all_channels_height: f32,
    /// The vectorscope rings' alpha on `text.faint`.
    pub vector_ring_alpha: f32,
    /// The smallest and largest half-side of a cloud dot.
    pub vector_dot: [f32; 2],
    /// The hover readout badge's height.
    pub hover_badge_height: f32,
    /// The hovered zone band's alpha on `text.normal`.
    pub zone_alpha: f32,
    /// The vectorscope's default height range, 30 % of the available height clamped to it.
    pub vector_height: [f32; 2],
    /// The vectorscope's margin between its outer ring and the plot's edge.
    pub vector_margin: f32,
    /// The inset of a clipping warning's triangle in its button.
    pub warning_inset: f32,
}

impl ScopePanel<'_> {
    /// The scope panel's look in `tokens`.
    pub fn style(tokens: &UiTokens) -> ScopePanelStyle {
        ScopePanelStyle {
            plot_height: tokens.size.control.md * PLOT_ROWS,
            warning_size: WARNING_SIZE,
            statistics_gap: STATISTICS_GAP,
            all_channels_height: ALL_CHANNELS_HEIGHT,
            vector_ring_alpha: VECTOR_RING_ALPHA,
            vector_dot: [VECTOR_DOT_MIN, VECTOR_DOT_MAX],
            hover_badge_height: HOVER_BADGE_HEIGHT,
            zone_alpha: ZONE_ALPHA,
            vector_height: [VECTOR_HEIGHT_MIN, VECTOR_HEIGHT_MAX],
            vector_margin: VECTOR_MARGIN,
            warning_inset: WARNING_INSET,
        }
    }
}
