//! `TileStyle` from the semantic tokens.

use crate::style::tile::TileStyle as TileSizes;
use crate::theme::{ToEgui as _, UiTokens, role_format};
use craft_theme::TypeRole;
use egui::text::LayoutJob;
use egui::{Color32, Painter};

/// Widest sample of a caption's figures: sizes the column of a start-placed caption.
const START_CAPTION_SAMPLE: &str = "000";

/// Share of the scrim's opacity that veils a tile.
const VEIL_OPACITY: f32 = 0.45;

/// The tile grid's and compare tile's look: the `tile` family.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TileStyle {
    /// Between tiles.
    pub gap: f32,
    /// Inside a tile, around its image.
    pub inset: f32,
    /// Height of a group header.
    pub group_header: f32,
    /// Between a tile and its caption.
    pub caption_gap: f32,
    /// Height of a tile bar.
    pub bar: f32,
    /// Side of a flag icon.
    pub flag: f32,
    /// Diameter of a status dot.
    pub dot: f32,
    /// Side of a tile checkbox.
    pub checkbox: f32,
    /// Margin of the checkbox backdrop.
    pub checkbox_backdrop: f32,
    /// Height of a badge.
    pub badge_height: f32,
    /// Height of the status band.
    pub status_band: f32,
    /// Height of the range bar.
    pub range_bar: f32,
    /// Width of the selection ring.
    pub ring_width: f32,
    /// Width of the hover ring.
    pub ring_hover_width: f32,
    /// Width of the ring halo.
    pub ring_halo_width: f32,
    /// Width of the current tile's ring.
    pub ring_current_width: f32,
    /// Share of the grid's height that one wheel notch scrolls.
    pub wheel_share: f32,
    /// Ring of a selected tile.
    pub ring_selected: Color32,
    /// Halo around a ring.
    pub ring_halo: Color32,
    /// Ring of a hovered tile.
    pub ring_hover: Color32,
    /// Ring of the current tile.
    pub ring_current: Color32,
    /// Caption text.
    pub caption: Color32,
    /// Caption text of a selected tile.
    pub caption_selected: Color32,
    /// Detail text.
    pub detail: Color32,
    /// Group header text.
    pub group_text: Color32,
    /// Group item count.
    pub group_count: Color32,
    /// Fill of a pinned group header.
    pub group_pinned: Color32,
    /// Scrim behind overlaid text.
    pub scrim: Color32,
    /// Text on a badge.
    pub badge_text: Color32,
    /// Veil over a disabled tile.
    pub veil: Color32,
    /// Range bar fill.
    pub range: Color32,
    /// Scrub line.
    pub scrub: Color32,
    /// Marquee fill.
    pub marquee_fill: Color32,
    /// Marquee outline.
    pub marquee_line: Color32,
}

impl TileStyle {
    /// Width of the caption column of `CaptionPlace::Start`: a sample of figures in the caption's
    /// type role.
    pub fn start_caption_width(painter: &Painter, tokens: &UiTokens) -> f32 {
        let job = LayoutJob::single_section(START_CAPTION_SAMPLE.to_owned(), role_format(tokens, TypeRole::Caption, Color32::PLACEHOLDER));
        painter.layout_job(job).size().x
    }

    /// The style of `tokens`.
    pub fn style(tokens: &UiTokens) -> TileStyle {
        let (color, space, size, stroke) = (&tokens.color, &tokens.space, &tokens.size, &tokens.stroke);
        let tile = TileSizes::new(tokens);
        TileStyle {
            gap: space.gap.stack,
            inset: space.gap.inline,
            group_header: size.panel_header,
            caption_gap: space.gap.tight,
            bar: size.control.sm,
            flag: size.icon.sm,
            dot: tile.dot,
            checkbox: size.icon.sm,
            checkbox_backdrop: space.gap.tight,
            badge_height: size.icon.sm,
            status_band: size.icon.md,
            range_bar: tile.range_bar,
            ring_width: stroke.thick,
            ring_hover_width: stroke.thin,
            ring_halo_width: stroke.thin,
            ring_current_width: stroke.thick,
            wheel_share: tile.wheel_share,
            ring_selected: color.accent.solid.to_egui(),
            ring_halo: color.overlay.halo.to_egui(),
            ring_hover: color.text.muted.to_egui(),
            ring_current: color.accent.solid.to_egui(),
            caption: color.text.muted.to_egui(),
            caption_selected: color.text.normal.to_egui(),
            detail: color.text.faint.to_egui(),
            group_text: color.text.normal.to_egui(),
            group_count: color.text.faint.to_egui(),
            group_pinned: color.surface.panel.to_egui(),
            scrim: color.surface.scrim.to_egui(),
            badge_text: color.text.on_strong.to_egui(),
            veil: color.surface.scrim.faded(VEIL_OPACITY).to_egui(),
            range: color.accent.solid.to_egui(),
            scrub: color.accent.solid.to_egui(),
            marquee_fill: color.accent.soft.to_egui(),
            marquee_line: color.accent.border.to_egui(),
        }
    }
}
