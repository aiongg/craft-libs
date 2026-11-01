//! One tile: thumbnail, caption, badges, bar, status, checkbox and veil.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::canvas::textures::TextureState;
use egui::{Painter, Rect, Ui};

use super::{BadgeVisibility, TileContent, TileLabels, TileStyle};

/// How one tile is drawn this pass.
pub(super) struct TileLook<'a> {
    pub content: &'a TileContent,
    pub picture: Rect,
    pub caption: Option<Rect>,
    pub selected: bool,
    pub hovered: bool,
    pub lead_focused: bool,
    pub checked: Option<bool>,
    pub badges: BadgeVisibility,
    pub scrub: Option<f32>,
    pub rtl: bool,
}

/// Draws the picture (`Thumbnails::paint` with the tile's state, or the placeholder alone), then
/// the veil (rejected or dimmed: `tile.veil`; disabled: the whole tile at 40 %), the In and Out
/// bar (`tile.range_bar` high, `tile.range`), the scrub line (`tile.scrub`, 1 pt), the offline
/// band (`tile.status_band`), the badge bar (`tile.bar` high on `tile.scrim` per
/// `BadgeVisibility`: `paint_rating(.., ControlSize::Sm)` in `tile.badge.text`, flag glyph
/// `tile.flag`, colour dot `tile.dot` with a 1 pt `tile.ring.halo` outline), the corner badges
/// (`tile.badge_height` pills; a badge in the check box's corner moves below it), the check box
/// (the library `Checkbox` look at `tile.checkbox`), the rings (selected, hover, focus) and the
/// caption (name via `fit_name` in `Caption` `tile.caption` or `tile.caption_selected`, detail in
/// `tile.detail`). Returns where the stars were drawn, for the rating click.
pub(super) fn paint_tile(
    ui: &Ui,
    painter: &Painter,
    style: &TileStyle,
    labels: &TileLabels,
    look: &TileLook<'_>,
    state: Option<&TextureState>,
) -> Option<Rect> {
    // lightcraft: crates/ui-egui/src/panels/grid.rs:766 (`cell`, `paint_cell_thumbnail` :816, `paint_cell_caption` :848, `paint_cell_badges` :878).
    todo!()
}

/// The badge rects for one corner, inward from it, skipping the check box's rect (LightCraft's
/// `a_badge_never_covers_the_checkbox`).
pub(super) fn badge_rects(picture: Rect, widths: &[f32], style: &TileStyle, checkbox: Option<Rect>, at_start: bool, top: bool) -> Vec<Rect> {
    // lightcraft: crates/ui-egui/src/dialogs/import/body.rs:460 (`badge_rect`: below the box when too wide).
    todo!()
}

/// The accessible name built from `TileLabels`: name, stars, flag, label, status, badge tips.
pub(super) fn accessible_name(content: &TileContent, labels: &TileLabels, label_name: Option<&str>) -> String {
    // lightcraft: crates/ui-egui/src/panels/grid.rs:800 (`cell_description`).
    todo!()
}
