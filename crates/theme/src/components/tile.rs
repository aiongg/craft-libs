//! Media tiles: the grid of pictures, the compare tile and the media bin's label chip. Rows,
//! tables, panels and footers are the list, table and panel families'. No theme overrides it.

use super::{ContrastPair, Family, FieldKind, OverrideValue, Semantic, SizeLeaf};
use crate::color::Rgba;
use crate::values;

/// Tile tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TileTokens {
    /// Between tiles: `tile.gap` (Compact 6, Comfortable 8).
    pub gap: f32,
    /// From the grid's edge to its first tile: `tile.inset` (4 / 6).
    pub inset: f32,
    /// Height of a group header: `tile.group_header` (40 / 44).
    pub group_header: f32,
    /// Between the picture and its caption: `tile.caption_gap` (2 / 3).
    pub caption_gap: f32,
    /// Height of the badge bar: [`values::TILE_BAR`].
    pub bar: f32,
    /// Side of the flag glyph: [`values::TILE_FLAG`].
    pub flag: f32,
    /// Side of the colour dot: [`values::TILE_DOT`].
    pub dot: f32,
    /// Side of the check box in check mode: [`values::TILE_CHECKBOX`].
    pub checkbox: f32,
    /// Margin of the scrim behind the check box, around its side: [`values::TILE_CHECKBOX_BACKDROP`].
    pub checkbox_backdrop: f32,
    /// Height of a badge pill: [`values::TILE_BADGE_HEIGHT`].
    pub badge_height: f32,
    /// Height of the status band: [`values::TILE_STATUS_BAND`].
    pub status_band: f32,
    /// Height of the In/Out range bar: [`values::TILE_RANGE_BAR`].
    pub range_bar: f32,
    /// Width of the selection ring: [`values::TILE_RING_WIDTH`].
    pub ring_width: f32,
    /// Width of the hover ring: [`values::TILE_RING_HOVER_WIDTH`].
    pub ring_hover_width: f32,
    /// Width of the halo outside the selection ring: [`values::TILE_RING_HALO_WIDTH`].
    pub ring_halo_width: f32,
    /// Share of the grid's height one wheel notch scrolls: [`values::TILE_WHEEL_SHARE`].
    pub wheel_share: f32,
    /// Height of the compare caption strip: [`values::TILE_COMPARE_CAPTION`].
    pub compare_caption: f32,
    /// Margin around the compare panes: [`values::TILE_COMPARE_MARGIN`].
    pub compare_margin: f32,
    /// Gap between the compare panes: [`values::TILE_COMPARE_GAP`].
    pub compare_gap: f32,
    /// Room the compare caption needs to keep the name: [`values::TILE_COMPARE_NAME_ROOM`].
    pub compare_name_room: f32,
    /// Side of the media bin's label chip: [`values::TILE_BIN_CHIP`].
    pub bin_chip: f32,
    /// Radius of the media bin's label chip: [`values::TILE_BIN_CHIP_RADIUS`].
    pub bin_chip_radius: f32,
    /// The selection ring: `tile.ring.selected`, `accent.solid`.
    pub ring_selected: Rgba,
    /// The halo outside the selection ring: `tile.ring.halo`, `overlay.halo`.
    pub ring_halo: Rgba,
    /// The hover ring: `tile.ring.hover`, `text.muted`.
    pub ring_hover: Rgba,
    /// A caption: `tile.caption`, `text.muted`.
    pub caption: Rgba,
    /// The caption of a selected tile: `tile.caption_selected`, `text.normal`.
    pub caption_selected: Rgba,
    /// The detail line: `tile.detail`, `text.faint`.
    pub detail: Rgba,
    /// A group header's title: `tile.group.text`, `text.normal`.
    pub group_text: Rgba,
    /// A group header's count: `tile.group.count`, `text.faint`.
    pub group_count: Rgba,
    /// The pinned group header's fill: `tile.group.pinned`, `surface.panel`.
    pub group_pinned: Rgba,
    /// The badge bar's fill: `tile.scrim`, `surface.scrim`.
    pub scrim: Rgba,
    /// Text and glyphs on the badge bar: `tile.badge.text`, `text.on_strong`.
    pub badge_text: Rgba,
    /// The veil over a rejected tile: `tile.veil`, `surface.scrim` at [`values::TILE_VEIL_ALPHA`].
    pub veil: Rgba,
    /// The In/Out range bar: `tile.range`, `accent.solid`.
    pub range: Rgba,
    /// The scrub playhead: `tile.scrub`, `accent.solid`.
    pub scrub: Rgba,
    /// The marquee's fill: `tile.marquee.fill`, `accent.soft`.
    pub marquee_fill: Rgba,
    /// The marquee's outline: `tile.marquee.line`, `accent.border`.
    pub marquee_line: Rgba,
}

/// Tiles have no overridable field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TileField {}

impl Family for TileTokens {
    type Field = TileField;
    const FIELDS: &'static [TileField] = &[];
    const SIZES: &'static [SizeLeaf<Self>] = sizes! { TileTokens;
        "gap" 0.0..=32.0 => gap;
        "inset" 0.0..=32.0 => inset;
        "group_header" 16.0..=96.0 => group_header;
        "caption_gap" 0.0..=16.0 => caption_gap;
        "bar" 8.0..=64.0 => bar;
        "flag" 4.0..=32.0 => flag;
        "dot" 4.0..=32.0 => dot;
        "checkbox" 8.0..=48.0 => checkbox;
        "checkbox_backdrop" 0.0..=16.0 => checkbox_backdrop;
        "badge_height" 8.0..=48.0 => badge_height;
        "status_band" 8.0..=48.0 => status_band;
        "range_bar" 1.0..=16.0 => range_bar;
        "ring_width" 0.0..=8.0 => ring_width;
        "ring_hover_width" 0.0..=8.0 => ring_hover_width;
        "ring_halo_width" 0.0..=8.0 => ring_halo_width;
        "wheel_share" 0.05..=1.0 => wheel_share;
        "compare_caption" 12.0..=96.0 => compare_caption;
        "compare_margin" 0.0..=64.0 => compare_margin;
        "compare_gap" 0.0..=64.0 => compare_gap;
        "compare_name_room" 0.0..=400.0 => compare_name_room;
        "bin_chip" 4.0..=32.0 => bin_chip;
        "bin_chip_radius" 0.0..=16.0 => bin_chip_radius;
    };

    fn defaults(s: &Semantic<'_>) -> TileTokens {
        let c = s.color;
        TileTokens {
            gap: s.set.tile_gap,
            inset: s.set.tile_inset,
            group_header: s.set.tile_group_header,
            caption_gap: s.set.tile_caption_gap,
            bar: values::TILE_BAR,
            flag: values::TILE_FLAG,
            dot: values::TILE_DOT,
            checkbox: values::TILE_CHECKBOX,
            checkbox_backdrop: values::TILE_CHECKBOX_BACKDROP,
            badge_height: values::TILE_BADGE_HEIGHT,
            status_band: values::TILE_STATUS_BAND,
            range_bar: values::TILE_RANGE_BAR,
            ring_width: values::TILE_RING_WIDTH,
            ring_hover_width: values::TILE_RING_HOVER_WIDTH,
            ring_halo_width: values::TILE_RING_HALO_WIDTH,
            wheel_share: values::TILE_WHEEL_SHARE,
            compare_caption: values::TILE_COMPARE_CAPTION,
            compare_margin: values::TILE_COMPARE_MARGIN,
            compare_gap: values::TILE_COMPARE_GAP,
            compare_name_room: values::TILE_COMPARE_NAME_ROOM,
            bin_chip: values::TILE_BIN_CHIP,
            bin_chip_radius: values::TILE_BIN_CHIP_RADIUS,
            ring_selected: c.accent.solid,
            ring_halo: c.overlay.halo,
            ring_hover: c.text.muted,
            caption: c.text.muted,
            caption_selected: c.text.normal,
            detail: c.text.faint,
            group_text: c.text.normal,
            group_count: c.text.faint,
            group_pinned: c.surface.panel,
            scrim: c.surface.scrim,
            badge_text: c.text.on_strong,
            veil: c.surface.scrim.faded(values::TILE_VEIL_ALPHA),
            range: c.accent.solid,
            scrub: c.accent.solid,
            marquee_fill: c.accent.soft,
            marquee_line: c.accent.border,
        }
    }

    fn path(f: TileField) -> String {
        match f {}
    }

    fn from_path(_: &str) -> Option<TileField> {
        None
    }

    fn kind(f: TileField) -> FieldKind {
        match f {}
    }

    fn get(&self, f: TileField) -> OverrideValue {
        match f {}
    }

    fn set(&mut self, f: TileField, _: OverrideValue) {
        match f {}
    }

    // Pictures are user pixels: no ratio is promised against them. The ring's pairs sit with the
    // swatch family's.
    fn pairs() -> Vec<ContrastPair> {
        Vec::new()
    }
}
