//! `TileLayout`, `TileArrangement` and `arrange`: where each tile and group header goes; and
//! `fit_name`, the caption's middle shortening.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::ops::Range;

use egui::{Rect, Vec2};

/// How tiles are laid out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TileLayout {
    /// Rows of pictures at their own aspect, `row_height` points high, each row but a group's
    /// last scaled to fill the width (LightCraft's library grid). Right to left, rows fill from
    /// the right.
    Justified { row_height: f32 },
    /// Equal cards `side` points wide, the picture `side / aspect` high, with a caption below
    /// when `caption` (Bridge, PdfCraft, FigCraft, LightCraft's square grid as
    /// `Cards { aspect: 1.0, caption: true }`). Columns follow the width (`GridNav::columns`).
    Cards { side: f32, aspect: f32, caption: bool },
}

/// A run of tiles under one header (a date, a folder): `start..start + count` in display order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TileGroup {
    /// The first tile's position.
    pub start: usize,
    /// How many tiles.
    pub count: usize,
    /// The header's title ("October 2026").
    pub title: String,
    /// The header's detail at the trailing end ("128 photos").
    pub detail: String,
}

/// One laid-out row: its tiles and its band.
#[derive(Clone, Debug, PartialEq)]
pub struct TileRow {
    /// The tiles in it, in display order.
    pub tiles: Range<usize>,
    /// The row's top, from the arrangement's top.
    pub top: f32,
    /// Its height, the caption included.
    pub height: f32,
    /// The group it belongs to, when groups are given.
    pub group: Option<usize>,
}

/// Where every tile and header goes for one width, size and set of aspects: cached by the grid
/// and rebuilt only when the count, the aspect list's revision, the width, the layout or the
/// groups change (100 000 tiles stay interactive).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TileArrangement {
    /// Rows, top to bottom.
    pub rows: Vec<TileRow>,
    /// Each tile's picture rect from the arrangement's top left (without the caption).
    pub tiles: Vec<Rect>,
    /// Each group header's rect, `tile.group_header` high, in group order.
    pub headers: Vec<Rect>,
    /// The whole height.
    pub height: f32,
}

impl TileArrangement {
    /// The rows that cross `top..bottom` (from the arrangement's top), by binary search over the
    /// row tops.
    pub fn rows_between(&self, top: f32, bottom: f32) -> Range<usize> {
        // lightcraft: crates/ui-egui/src/panels/grid.rs:279 (`rows_between`).
        todo!()
    }

    /// The tile counts of the rows, for `GridNav::ragged`.
    pub fn row_lengths(&self) -> Vec<usize> {
        // lightcraft: crates/ui-egui/src/panels/grid.rs:536 (`vertical_neighbor` over the layout rows).
        todo!()
    }

    /// The row holding tile `at`.
    pub fn row_of(&self, at: usize) -> Option<usize> {
        // new: for scrolling a tile into view.
        todo!()
    }
}

/// Spacing that `arrange` reads, from `TileStyle`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArrangeSpacing {
    /// Between tiles: `tile.gap` (6 / 8).
    pub gap: f32,
    /// Around the content: `tile.inset` (4 / 6).
    pub inset: f32,
    /// A group header's height: `tile.group_header` (40 / 44).
    pub group_header: f32,
    /// A card's caption height (one `Caption` line, a detail line, `tile.caption_gap`); 0 without
    /// captions.
    pub caption: f32,
}

/// Lays out `aspects` (width over height, one per tile in display order; non-finite or ≤ 0
/// counts as 1) in `width` points.
///
/// - `Justified`: tiles fill a row at `row_height` until the next would pass the width; the row
///   is then scaled so its tiles and gaps fill the width exactly; a group's last row keeps
///   `row_height`. Each group starts a new row below its header. Content starts at `inset`.
/// - `Cards`: `GridNav::columns(width, inset, gap, side)` columns (at least 1); each group starts
///   a new row below its header.
/// - `groups` are in display order and cover only the tiles they name; tiles outside every group
///   sit in rows without a header.
pub fn arrange(aspects: &[f32], groups: &[TileGroup], width: f32, layout: TileLayout, spacing: ArrangeSpacing) -> TileArrangement {
    // lightcraft: crates/ui-egui/src/panels/grid.rs:588 (`layout`: justified rows, square cells, groups).
    todo!()
}

/// `name` shortened in the middle to fit, keeping its extension: `fits` says whether a text fits
/// the caption width. The whole name when it fits; else the longest `head…tail.ext` that fits,
/// cut on character boundaries; `….ext` when nothing more fits.
pub fn fit_name(name: &str, fits: impl Fn(&str) -> bool) -> String {
    // pdfcraft: crates/ui-egui/src/workflows/combine_grid.rs:78 (`fit_name`).
    todo!()
}

/// The size a card cell takes: `side` wide, `side / aspect` plus the caption high.
pub fn card_size(side: f32, aspect: f32, caption: f32) -> Vec2 {
    // pdfcraft: crates/ui-egui/src/workflows/combine_grid.rs:41 (card cell from the paper and its surround).
    todo!()
}
