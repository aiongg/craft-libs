//! Where tiles and group headers go: `arrange` lays out justified rows and card grids under group headers, and
//! `GridLayout` is the item layout the grid hands the item step. Equal tiles without groups are
//! `craft_behavior::TileLayout`; everything else is an arrangement.

use craft_behavior::{ItemLayout, ItemSource, NavShape, ScrollPos, SizeHint, TileLayout as EqualTiles};
use egui::{Pos2, Rect, Vec2, pos2, vec2};
use std::ops::Range;
use std::sync::Arc;

/// A run of tiles under one header (a date, a folder): `start..start + count` in display order.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct TileGroup {
    /// The first tile's position.
    pub start: usize,
    /// How many tiles.
    pub count: usize,
    /// The header's title ("October 2026").
    pub title: String,
    /// The header's detail at the trailing end ("128 photos").
    pub detail: String,
    /// Whether the group shows its tiles; a closed group shows its header only.
    pub open: bool,
}

impl TileGroup {
    /// An open group of `count` tiles from position `start`, titled `title`.
    pub fn new(start: usize, count: usize, title: impl Into<String>) -> Self {
        TileGroup { start, count, title: title.into(), detail: String::new(), open: true }
    }

    /// The detail at the header's trailing end.
    #[must_use]
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = detail.into();
        self
    }

    /// Whether the group shows its tiles.
    #[must_use]
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }
}

/// How the tiles of an arrangement are sized.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Tiling {
    /// Rows of pictures at their own aspect, `row_height` high, each row but a group's last scaled to fill the
    /// width.
    #[non_exhaustive]
    Justified {
        /// Row height.
        row_height: f32,
    },
    /// Equal cards `side` wide, the picture `side / aspect` high, with a band of `caption` below the picture.
    #[non_exhaustive]
    Cards {
        /// Card width.
        side: f32,
        /// Picture width divided by height.
        aspect: f32,
        /// The height of the caption band under the picture; 0 without captions.
        caption: f32,
    },
}

impl Tiling {
    /// Justified rows `row_height` high.
    pub fn justified(row_height: f32) -> Self {
        Tiling::Justified { row_height }
    }

    /// Cards `side` wide.
    pub fn cards(side: f32, aspect: f32, caption: f32) -> Self {
        Tiling::Cards { side, aspect, caption }
    }
}

/// One laid-out row: its tiles and its band.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
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

/// Where every tile and header goes for one width, tiling and set of aspects.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct TileArrangement {
    /// Rows, top to bottom.
    pub rows: Vec<TileRow>,
    /// Each tile's picture rect from the arrangement's top left, without the caption; `Rect::NOTHING` for a tile
    /// of a closed group.
    pub tiles: Vec<Rect>,
    /// Each group header's rect, in group order.
    pub headers: Vec<Rect>,
    /// The whole height.
    pub height: f32,
}

impl TileArrangement {
    /// The rows that cross `top..bottom` (from the arrangement's top), by binary search over the row tops.
    pub fn rows_between(&self, top: f32, bottom: f32) -> Range<usize> {
        let first = self.rows.partition_point(|r| r.top + r.height < top);
        let rest = self.rows.get(first..).unwrap_or(&[]);
        let count = rest.partition_point(|r| r.top <= bottom);
        first..first + count
    }

    /// The tile counts of the rows in order, with each run of tiles of a closed group as a row of its own.
    pub fn row_lengths(&self) -> Vec<usize> {
        let mut lengths = Vec::with_capacity(self.rows.len());
        let mut next = 0;
        for row in &self.rows {
            if row.tiles.start > next {
                lengths.push(row.tiles.start - next);
            }
            lengths.push(row.tiles.len());
            next = row.tiles.end;
        }
        if next < self.tiles.len() {
            lengths.push(self.tiles.len() - next);
        }
        lengths
    }

    /// The row holding tile `at`.
    pub fn row_of(&self, at: usize) -> Option<usize> {
        let row = self.rows.partition_point(|r| r.tiles.end <= at);
        self.rows.get(row).filter(|r| r.tiles.contains(&at)).map(|_| row)
    }

    /// The group whose tiles cross the top edge `top`, for the pinned header.
    pub fn group_at(&self, top: f32) -> Option<usize> {
        let row = self.rows.get(self.rows_between(top, top).start)?;
        row.group
    }
}

/// The spacing `arrange` reads.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ArrangeSpacing {
    /// Between tiles.
    pub gap: f32,
    /// Around the content.
    pub inset: f32,
    /// A group header's height.
    pub group_header: f32,
}

impl ArrangeSpacing {
    /// Spacing of `gap` between tiles, `inset` around the content and headers `group_header` high.
    pub fn new(gap: f32, inset: f32, group_header: f32) -> Self {
        ArrangeSpacing { gap, inset, group_header }
    }
}

/// The aspect of every tile of `source`: `SizeHint::Aspect`, or a `SizeHint::Rect`'s width over height; a tile that
/// gives neither, or a non-finite or non-positive one, counts as 1.
pub fn aspects_of<K>(source: &dyn ItemSource<K>) -> Vec<f32> {
    let sane = |aspect: f32| if aspect.is_finite() && aspect > 0.0 { aspect } else { 1.0 };
    (0..source.len())
        .map(|at| match source.size_hint(at) {
            SizeHint::Aspect(aspect) => sane(aspect),
            SizeHint::Rect(size) if size.y > 0.0 => sane(size.x / size.y),
            _ => 1.0,
        })
        .collect()
}

/// Lays out `aspects` (width over height, one per tile in display order) in `width`.
///
/// - `Justified`: tiles fill a row at `row_height` until the next would pass the width; the row is then scaled so
///   its tiles and gaps fill the width exactly; a group's last row keeps `row_height`. Each tile has a column of
///   `start` at its left, outside the picture, so a row's pictures and columns fill the width together.
/// - `Cards`: as many columns as fit; each group starts a new row below its header. A card with a `start` column
///   is that much wider and has no caption band.
/// - `groups` are in display order and cover only the tiles they name; tiles outside every group sit in rows
///   without a header. A closed group has its header only.
pub fn arrange(aspects: &[f32], groups: &[TileGroup], width: f32, tiling: Tiling, spacing: ArrangeSpacing, start: f32) -> TileArrangement {
    let sanitized: Vec<f32> = aspects.iter().map(|a| if a.is_finite() && *a > 0.0 { *a } else { 1.0 }).collect();
    let n = sanitized.len();
    let ArrangeSpacing { gap, inset, group_header } = spacing;
    let content = (width - 2.0 * inset).max(1.0);
    let mut out = TileArrangement { tiles: vec![Rect::NOTHING; n], ..TileArrangement::default() };

    let mut segments: Vec<(Option<usize>, Range<usize>)> = Vec::new();
    let mut at = 0;
    for (index, group) in groups.iter().enumerate() {
        let first = group.start.clamp(at, n);
        let end = group.start.saturating_add(group.count).clamp(first, n);
        if first > at {
            segments.push((None, at..first));
        }
        segments.push((Some(index), first..end));
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
        if let Some(index) = group {
            out.headers.push(Rect::from_min_size(pos2(inset, y), vec2(content, group_header)));
            y += group_header;
            if !groups[index].open {
                continue;
            }
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
        match tiling {
            Tiling::Justified { row_height } => {
                let step = gap + start;
                let room = content - start;
                let mut i = range.start;
                while i < range.end {
                    let mut j = i + 1;
                    let mut sum = sanitized[i];
                    while j < range.end && (sum + sanitized[j]) * row_height + step * (j - i) as f32 <= room {
                        sum += sanitized[j];
                        j += 1;
                    }
                    let height = if j < range.end { (room - step * (j - i - 1) as f32) / sum } else { row_height };
                    let top = place(&mut out, &mut y, i..j, height);
                    let mut x = inset + start;
                    for (tile, aspect) in out.tiles[i..j].iter_mut().zip(&sanitized[i..j]) {
                        let w = aspect * height;
                        *tile = Rect::from_min_size(pos2(x, top), vec2(w, height));
                        x += w + step;
                    }
                    i = j;
                }
            }
            Tiling::Cards { side, aspect, caption } => {
                let aspect = if aspect.is_finite() && aspect > 0.0 { aspect } else { 1.0 };
                let card = side + start;
                let columns = columns_in(content, card, gap);
                let picture = vec2(side, side / aspect);
                let height = picture.y + if start > 0.0 { 0.0 } else { caption };
                let mut i = range.start;
                while i < range.end {
                    let j = (i + columns).min(range.end);
                    let top = place(&mut out, &mut y, i..j, height);
                    for (c, k) in (i..j).enumerate() {
                        out.tiles[k] = Rect::from_min_size(pos2(inset + start + c as f32 * (card + gap), top), picture);
                    }
                    i = j;
                }
            }
        }
    }
    out.height = y + inset;
    out
}

/// Cards of `card` with `gap` between them that fit in `width`, at least one: the count `EqualTiles::columns` gives.
fn columns_in(width: f32, card: f32, gap: f32) -> usize {
    (((width + gap) / (card + gap)).floor() as usize).max(1)
}

/// The item layout of a tile grid.
#[derive(Clone, Debug)]
pub struct GridLayout {
    kind: Kind,
}

#[derive(Clone, Debug)]
enum Kind {
    /// Equal cards in no group, `inset` from every edge.
    Equal { tiles: EqualTiles, inset: f32 },
    /// An arrangement; each card also holds the `start` column left of its picture.
    Arranged { arrangement: Arc<TileArrangement>, rows: Arc<[usize]>, width: f32, start: f32 },
}

impl GridLayout {
    /// Equal cards of `card` in no group: `craft_behavior::TileLayout` inside `inset` on every side.
    pub fn equal<K>(source: &dyn ItemSource<K>, width: f32, inset: f32, card: Vec2, gap: f32) -> Self {
        let tiles = EqualTiles::new(source, (width - 2.0 * inset).max(1.0), card, gap);
        GridLayout { kind: Kind::Equal { tiles, inset } }
    }

    /// The layout `arrangement` makes in `width`; cards hold their `start` column.
    pub fn arranged(arrangement: TileArrangement, width: f32, start: f32) -> Self {
        let rows = arrangement.row_lengths().into();
        GridLayout { kind: Kind::Arranged { arrangement: Arc::new(arrangement), rows, width, start } }
    }

    /// The arrangement behind the layout, when it is not a plain equal grid.
    pub fn arrangement(&self) -> Option<&Arc<TileArrangement>> {
        match &self.kind {
            Kind::Arranged { arrangement, .. } => Some(arrangement),
            Kind::Equal { .. } => None,
        }
    }

    /// The card of tile `at` in the arrangement: its picture, its start column and the row's band below.
    fn card(arrangement: &TileArrangement, start: f32, at: usize) -> Rect {
        let picture = arrangement.tiles.get(at).copied().unwrap_or(Rect::NOTHING);
        if picture == Rect::NOTHING {
            return Rect::NOTHING;
        }
        let bottom = arrangement.row_of(at).map_or(picture.bottom(), |row| arrangement.rows[row].top + arrangement.rows[row].height);
        Rect::from_min_max(pos2(picture.left() - start, picture.top()), pos2(picture.right(), bottom))
    }
}

fn shifted(offset: ScrollPos, inset: f32) -> ScrollPos {
    ScrollPos { x: offset.x - f64::from(inset), y: offset.y - f64::from(inset) }
}

impl ItemLayout for GridLayout {
    fn content_size(&self) -> ScrollPos {
        match &self.kind {
            Kind::Equal { tiles, inset } => {
                let inner = tiles.content_size();
                let pad = 2.0 * f64::from(*inset);
                ScrollPos { x: inner.x + pad, y: if tiles.count == 0 { 0.0 } else { inner.y + pad } }
            }
            Kind::Arranged { arrangement, width, .. } => ScrollPos { x: f64::from(*width), y: f64::from(arrangement.height) },
        }
    }

    fn visible(&self, offset: ScrollPos, view: Vec2, out: &mut Vec<usize>) {
        match &self.kind {
            Kind::Equal { tiles, inset } => tiles.visible(shifted(offset, *inset), view, out),
            Kind::Arranged { arrangement, .. } => {
                let top = offset.y as f32;
                for row in &arrangement.rows[arrangement.rows_between(top, top + view.y)] {
                    out.extend(row.tiles.clone());
                }
            }
        }
    }

    fn rect(&self, at: usize, offset: ScrollPos) -> Rect {
        match &self.kind {
            Kind::Equal { tiles, inset } => tiles.rect(at, shifted(offset, *inset)),
            Kind::Arranged { arrangement, start, .. } => {
                let card = Self::card(arrangement, *start, at);
                if card == Rect::NOTHING { card } else { card.translate(vec2(-(offset.x as f32), -(offset.y as f32))) }
            }
        }
    }

    fn hit(&self, point: Pos2, offset: ScrollPos) -> Option<usize> {
        match &self.kind {
            Kind::Equal { tiles, inset } => tiles.hit(point, shifted(offset, *inset)),
            Kind::Arranged { arrangement, start, .. } => {
                let at = pos2(point.x + offset.x as f32, point.y + offset.y as f32);
                let row = arrangement.rows.get(arrangement.rows_between(at.y, at.y).start)?;
                row.tiles.clone().find(|&tile| Self::card(arrangement, *start, tile).contains(at))
            }
        }
    }

    fn nav(&self) -> NavShape {
        match &self.kind {
            Kind::Equal { tiles, .. } => tiles.nav(),
            Kind::Arranged { .. } => NavShape::RaggedGrid,
        }
    }

    fn row_len(&self, row: usize) -> Option<usize> {
        match &self.kind {
            Kind::Arranged { rows, .. } => rows.get(row).copied(),
            Kind::Equal { .. } => None,
        }
    }

    fn shown(&self, at: usize) -> bool {
        self.arrangement().is_none_or(|arrangement| arrangement.tiles.get(at).is_some_and(|tile| *tile != Rect::NOTHING))
    }
}
