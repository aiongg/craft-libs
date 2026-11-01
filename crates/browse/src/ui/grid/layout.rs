//! `TileLayout`, `TileArrangement` and `arrange`: where each tile and group header goes; and
//! `fit_name`, the caption's middle shortening.

use std::ops::Range;

use craft_ui::behavior::GridNav;
use egui::{Rect, Vec2, pos2, vec2};

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
        let first = self.rows.partition_point(|r| r.top + r.height < top);
        let rest = self.rows.get(first..).unwrap_or(&[]);
        let count = rest.partition_point(|r| r.top <= bottom);
        first..first + count
    }

    /// The tile counts of the rows, for `GridNav::ragged`.
    pub fn row_lengths(&self) -> Vec<usize> {
        // lightcraft: crates/ui-egui/src/panels/grid.rs:536 (`vertical_neighbor` over the layout rows).
        self.rows.iter().map(|r| r.tiles.len()).collect()
    }

    /// The row holding tile `at`.
    pub fn row_of(&self, at: usize) -> Option<usize> {
        // new: for scrolling a tile into view.
        let row = self.rows.partition_point(|r| r.tiles.end <= at);
        self.rows.get(row).filter(|r| r.tiles.contains(&at)).map(|_| row)
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
    let sanitized: Vec<f32> = aspects.iter().map(|a| if a.is_finite() && *a > 0.0 { *a } else { 1.0 }).collect();
    let n = sanitized.len();
    let ArrangeSpacing { gap, inset, group_header, caption } = spacing;
    let content = (width - 2.0 * inset).max(1.0);
    let mut out = TileArrangement { tiles: vec![Rect::NOTHING; n], ..TileArrangement::default() };

    // Segments in display order: (group index, tile range); tiles outside every group have no header.
    let mut segments: Vec<(Option<usize>, Range<usize>)> = Vec::new();
    let mut at = 0;
    for (index, group) in groups.iter().enumerate() {
        let start = group.start.clamp(at, n);
        let end = (group.start.saturating_add(group.count)).clamp(start, n);
        if start > at {
            segments.push((None, at..start));
        }
        segments.push((Some(index), start..end));
        at = end;
    }
    if at < n {
        segments.push((None, at..n));
    }

    let mut y = inset;
    let mut started = false;
    for (group, range) in segments {
        if started {
            y += gap;
        }
        started = true;
        if group.is_some() {
            out.headers.push(Rect::from_min_size(pos2(inset, y), vec2(content, group_header)));
            y += group_header;
        }
        let mut first_row = true;
        let mut place = |out: &mut TileArrangement, y: &mut f32, tiles: Range<usize>, height: f32| {
            if !first_row {
                *y += gap;
            }
            first_row = false;
            out.rows.push(TileRow { tiles, top: *y, height, group });
            let top = *y;
            *y += height;
            top
        };
        match layout {
            TileLayout::Justified { row_height } => {
                let mut i = range.start;
                while i < range.end {
                    let mut j = i + 1;
                    let mut sum = sanitized[i];
                    while j < range.end && (sum + sanitized[j]) * row_height + gap * (j - i) as f32 <= content {
                        sum += sanitized[j];
                        j += 1;
                    }
                    let height = if j < range.end { (content - gap * (j - i - 1) as f32) / sum } else { row_height };
                    let top = place(&mut out, &mut y, i..j, height);
                    let mut x = inset;
                    for (tile, aspect) in out.tiles[i..j].iter_mut().zip(&sanitized[i..j]) {
                        let w = aspect * height;
                        *tile = Rect::from_min_size(pos2(x, top), vec2(w, height));
                        x += w + gap;
                    }
                    i = j;
                }
            }
            TileLayout::Cards { side, aspect, caption: captioned } => {
                let columns = GridNav::columns(width, inset, gap, side);
                let picture = card_size(side, aspect, 0.0);
                let height = card_size(side, aspect, if captioned { caption } else { 0.0 }).y;
                let mut i = range.start;
                while i < range.end {
                    let j = (i + columns).min(range.end);
                    let top = place(&mut out, &mut y, i..j, height);
                    for (c, k) in (i..j).enumerate() {
                        out.tiles[k] = Rect::from_min_size(pos2(inset + c as f32 * (side + gap), top), picture);
                    }
                    i = j;
                }
            }
        }
    }
    out.height = y + inset;
    out
}

/// `name` shortened in the middle to fit, keeping its extension: `fits` says whether a text fits
/// the caption width. The whole name when it fits; else the longest `head…tail.ext` that fits,
/// cut on character boundaries; `….ext` when nothing more fits.
pub fn fit_name(name: &str, fits: impl Fn(&str) -> bool) -> String {
    // pdfcraft: crates/ui-egui/src/workflows/combine_grid.rs:78 (`fit_name`).
    if fits(name) {
        return name.to_owned();
    }
    // A short extension after the last dot (a dot is one byte, so these are char boundaries).
    let split = name.rfind('.').filter(|p| *p > 0 && name.len() - p <= 6);
    let (stem, ext) = match split {
        Some(p) => (name.get(..p).unwrap_or(name), name.get(p..).unwrap_or("")),
        None => (name, ""),
    };
    let chars: Vec<char> = stem.chars().collect();
    // `kept` stem characters: a third of them (rounded down) from the end, the rest from the start.
    let with = |kept: usize| {
        let tail = kept / 3;
        let head: String = chars.iter().take(kept - tail).collect();
        let end: String = chars[chars.len() - tail..].iter().collect();
        format!("{head}…{end}{ext}")
    };
    // The most characters that fit, found by halving; the result is monotonic in `kept`.
    let (mut lo, mut hi) = (0usize, chars.len());
    while lo < hi {
        let mid = lo + (hi - lo).div_ceil(2);
        if fits(&with(mid)) { lo = mid } else { hi = mid - 1 }
    }
    with(lo)
}

/// The size a card cell takes: `side` wide, `side / aspect` plus the caption high.
pub fn card_size(side: f32, aspect: f32, caption: f32) -> Vec2 {
    // pdfcraft: crates/ui-egui/src/workflows/combine_grid.rs:41 (card cell from the paper and its surround).
    let aspect = if aspect.is_finite() && aspect > 0.0 { aspect } else { 1.0 };
    vec2(side, side / aspect + caption)
}
