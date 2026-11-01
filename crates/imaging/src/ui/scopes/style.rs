//! [`ScopePanelStyle`]: the scope panel's look, from tokens.

use craft_ui::theme::UiTokens;

use super::panel::ScopePanel;

/// The scope panel's sizes and ratios. Colours come from the histogram's and the data tokens
/// (`histogram.*`, `data.*`, `data.scope`, `field.fill`, `text.*`, `status.warn.fg`); this holds
/// only what the `scopes` family adds.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ScopePanelStyle {
    /// The Compact plot's height and the Expanded one's: `histogram.height` (100, lead call L30).
    pub plot_height: f32,
    /// A clipping warning button's side: `scopes.warning_size` (18, lead call L12).
    pub warning_size: f32,
    /// The gap between the statistics columns: `scopes.statistics_gap` (6).
    pub statistics_gap: f32,
    /// Each per-channel plot of All Channels View: `scopes.all_channels_height` (90).
    pub all_channels_height: f32,
    /// The vectorscope rings' alpha on `text.faint`: `scopes.vector_ring_alpha` (0.45).
    pub vector_ring_alpha: f32,
    /// The smallest and largest half-side of a cloud dot: `scopes.vector_dot_min` (0.7),
    /// `scopes.vector_dot_max` (3.0).
    pub vector_dot: [f32; 2],
    /// The hover readout badge's height: `scopes.hover_badge_height` (20).
    pub hover_badge_height: f32,
    /// The hovered zone band's alpha on `text.normal`: `scopes.zone_alpha` (0.055).
    pub zone_alpha: f32,
}

impl ScopePanel<'_> {
    /// The scope panel's look in `tokens`.
    ///
    /// Skeleton values are PhotoCraft's literals; TK2 swaps them for the `scopes.*` tokens.
    pub fn style(tokens: &UiTokens) -> ScopePanelStyle {
        // pc: crates/ui-egui/src/panels/histogram_panel.rs:750 (100), :797 (90), :721 (gap 6);
        // pc: crates/ui-egui/src/dialogs/camera_raw_scope_ui.rs:389 (18), :397 (badge 20), :417 (0.055), :461 (0.45), :471 (0.7..3.0).
        ScopePanelStyle {
            plot_height: tokens.component.histogram.height,
            warning_size: 18.0,
            statistics_gap: 6.0,
            all_channels_height: 90.0,
            vector_ring_alpha: 0.45,
            vector_dot: [0.7, 3.0],
            hover_badge_height: 20.0,
            zone_alpha: 0.055,
        }
    }
}
