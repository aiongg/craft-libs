//! [`ScopePanelStyle`]: the scope panel's look, from tokens.

use craft_ui::theme::UiTokens;

use super::panel::ScopePanel;

/// The scope panel's sizes and ratios. Colours come from the histogram's and the data tokens
/// (`histogram.*`, `data.*`, `data.scope`, `field.fill`, `text.*`, `status.warn.fg`); this holds
/// only what the `scopes` family adds.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ScopePanelStyle {
    /// The Compact plot's height and the Expanded one's: `histogram.height`.
    pub plot_height: f32,
    /// A clipping warning button's side: `scopes.warning_size`.
    pub warning_size: f32,
    /// The gap between the statistics columns: `scopes.statistics_gap`.
    pub statistics_gap: f32,
    /// Each per-channel plot of All Channels View: `scopes.all_channels_height`.
    pub all_channels_height: f32,
    /// The vectorscope rings' alpha on `text.faint`: `scopes.vector_ring_alpha`.
    pub vector_ring_alpha: f32,
    /// The smallest and largest half-side of a cloud dot: `scopes.vector_dot_min`,
    /// `scopes.vector_dot_max`.
    pub vector_dot: [f32; 2],
    /// The hover readout badge's height: `scopes.hover_badge_height`.
    pub hover_badge_height: f32,
    /// The hovered zone band's alpha on `text.normal`: `scopes.zone_alpha`.
    pub zone_alpha: f32,
    /// The vectorscope's default height range, 30 % of the available height clamped to it:
    /// `scopes.vector_height_min`, `scopes.vector_height_max`.
    pub vector_height: [f32; 2],
    /// The vectorscope's margin between its outer ring and the plot's edge:
    /// `scopes.vector_margin`.
    pub vector_margin: f32,
    /// The inset of a clipping warning's triangle in its button: `scopes.warning_inset`.
    pub warning_inset: f32,
}

impl ScopePanel<'_> {
    /// The scope panel's look in `tokens`.
    pub fn style(tokens: &UiTokens) -> ScopePanelStyle {
        let c = &tokens.component;
        ScopePanelStyle {
            plot_height: c.histogram.height,
            warning_size: c.scopes.warning_size,
            statistics_gap: c.scopes.statistics_gap,
            all_channels_height: c.scopes.all_channels_height,
            vector_ring_alpha: c.scopes.vector_ring_alpha,
            vector_dot: [c.scopes.vector_dot_min, c.scopes.vector_dot_max],
            hover_badge_height: c.scopes.hover_badge_height,
            zone_alpha: c.scopes.zone_alpha,
            vector_height: [c.scopes.vector_height_min, c.scopes.vector_height_max],
            vector_margin: c.scopes.vector_margin,
            warning_inset: c.scopes.warning_inset,
        }
    }
}
