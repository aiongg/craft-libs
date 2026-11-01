//! `TileStyle` from the `tile` tokens.

use craft_ui::theme::{ToEgui as _, UiTokens};
use egui::Color32;

/// The tile grid's and compare tile's look. Until the `tile` token family exists (task TK3) the
/// values come from the tokens they will derive from and the spec's numbers at Compact.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TileStyle {
    /// `tile.gap` (6 / 8).
    pub gap: f32,
    /// `tile.inset` (4 / 6).
    pub inset: f32,
    /// `tile.group_header` (40 / 44).
    pub group_header: f32,
    /// `tile.caption_gap` (2 / 3).
    pub caption_gap: f32,
    /// `tile.bar` (24).
    pub bar: f32,
    /// `tile.flag` (14).
    pub flag: f32,
    /// `tile.dot` (10).
    pub dot: f32,
    /// `tile.checkbox` (16).
    pub checkbox: f32,
    /// `tile.badge_height` (16).
    pub badge_height: f32,
    /// `tile.status_band` (20).
    pub status_band: f32,
    /// `tile.range_bar` (4).
    pub range_bar: f32,
    /// `tile.ring_width` (2).
    pub ring_width: f32,
    /// `tile.ring_hover_width` (1).
    pub ring_hover_width: f32,
    /// `tile.ring_halo_width` (1).
    pub ring_halo_width: f32,
    /// `tile.wheel_share` (a third of the grid's height per wheel notch).
    pub wheel_share: f32,
    /// `tile.ring.selected` (`accent.solid`).
    pub ring_selected: Color32,
    /// `tile.ring.halo` (`overlay.halo`).
    pub ring_halo: Color32,
    /// `tile.ring.hover` (`text.muted`).
    pub ring_hover: Color32,
    /// `tile.caption` (`text.muted`).
    pub caption: Color32,
    /// `tile.caption_selected` (`text.normal`).
    pub caption_selected: Color32,
    /// `tile.detail` (`text.faint`).
    pub detail: Color32,
    /// `tile.group.text` (`text.normal`).
    pub group_text: Color32,
    /// `tile.group.count` (`text.faint`).
    pub group_count: Color32,
    /// `tile.group.pinned` (`surface.panel`).
    pub group_pinned: Color32,
    /// `tile.scrim` (`surface.scrim`).
    pub scrim: Color32,
    /// `tile.badge.text` (`text.on_strong`).
    pub badge_text: Color32,
    /// `tile.veil` (`surface.scrim` at 45 %).
    pub veil: Color32,
    /// `tile.range` (`accent.solid`).
    pub range: Color32,
    /// `tile.scrub` (`accent.solid`).
    pub scrub: Color32,
    /// `tile.marquee.fill` (`accent.soft`).
    pub marquee_fill: Color32,
    /// `tile.marquee.line` (`accent.border`).
    pub marquee_line: Color32,
}

impl TileStyle {
    /// The style of `tokens`.
    pub fn of(tokens: &UiTokens) -> TileStyle {
        let c = &tokens.color;
        TileStyle {
            gap: 6.0,
            inset: 4.0,
            group_header: 40.0,
            caption_gap: 2.0,
            bar: 24.0,
            flag: 14.0,
            dot: 10.0,
            checkbox: 16.0,
            badge_height: 16.0,
            status_band: 20.0,
            range_bar: 4.0,
            ring_width: 2.0,
            ring_hover_width: 1.0,
            ring_halo_width: 1.0,
            wheel_share: 1.0 / 3.0,
            ring_selected: c.accent.solid.to_egui(),
            ring_halo: c.overlay.halo.to_egui(),
            ring_hover: c.text.muted.to_egui(),
            caption: c.text.muted.to_egui(),
            caption_selected: c.text.normal.to_egui(),
            detail: c.text.faint.to_egui(),
            group_text: c.text.normal.to_egui(),
            group_count: c.text.faint.to_egui(),
            group_pinned: c.surface.panel.to_egui(),
            scrim: c.surface.scrim.to_egui(),
            badge_text: c.text.on_strong.to_egui(),
            veil: c.surface.scrim.to_egui().gamma_multiply(0.45),
            range: c.accent.solid.to_egui(),
            scrub: c.accent.solid.to_egui(),
            marquee_fill: c.accent.soft.to_egui(),
            marquee_line: c.accent.border.to_egui(),
        }
    }
}
