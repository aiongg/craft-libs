//! `TileStyle` from the `tile` tokens.

use crate::theme::{ToEgui as _, UiTokens};
use egui::Color32;

/// The tile grid's and compare tile's look: the `tile` family.
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
    /// `tile.checkbox_backdrop` (2).
    pub checkbox_backdrop: f32,
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
    /// `tile.ring_current_width` (2): the current tile's ring (`TileGrid::current`).
    pub ring_current_width: f32,
    /// `tile.start_caption_width` (30): the caption column of `CaptionPlace::Start`.
    pub start_caption_width: f32,
    /// `tile.wheel_share` (a third of the grid's height per wheel notch).
    pub wheel_share: f32,
    /// `tile.ring.selected` (`accent.solid`).
    pub ring_selected: Color32,
    /// `tile.ring.halo` (`overlay.halo`).
    pub ring_halo: Color32,
    /// `tile.ring.hover` (`text.muted`).
    pub ring_hover: Color32,
    /// `tile.ring.current` (`accent.solid`).
    pub ring_current: Color32,
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
    pub fn style(tokens: &UiTokens) -> TileStyle {
        let t = &tokens.component.tile;
        TileStyle {
            gap: t.gap,
            inset: t.inset,
            group_header: t.group_header,
            caption_gap: t.caption_gap,
            bar: t.bar,
            flag: t.flag,
            dot: t.dot,
            checkbox: t.checkbox,
            checkbox_backdrop: t.checkbox_backdrop,
            badge_height: t.badge_height,
            status_band: t.status_band,
            range_bar: t.range_bar,
            ring_width: t.ring_width,
            ring_hover_width: t.ring_hover_width,
            ring_halo_width: t.ring_halo_width,
            ring_current_width: t.ring_current_width,
            start_caption_width: t.start_caption_width,
            wheel_share: t.wheel_share,
            ring_selected: t.ring_selected.to_egui(),
            ring_halo: t.ring_halo.to_egui(),
            ring_hover: t.ring_hover.to_egui(),
            ring_current: t.ring_current.to_egui(),
            caption: t.caption.to_egui(),
            caption_selected: t.caption_selected.to_egui(),
            detail: t.detail.to_egui(),
            group_text: t.group_text.to_egui(),
            group_count: t.group_count.to_egui(),
            group_pinned: t.group_pinned.to_egui(),
            scrim: t.scrim.to_egui(),
            badge_text: t.badge_text.to_egui(),
            veil: t.veil.to_egui(),
            range: t.range.to_egui(),
            scrub: t.scrub.to_egui(),
            marquee_fill: t.marquee_fill.to_egui(),
            marquee_line: t.marquee_line.to_egui(),
        }
    }
}
