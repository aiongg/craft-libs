//! The grid's parts for page panels (Acrobat's Pages panel, PowerPoint's slide strip; craft-pages'
//! `PageThumbnails` is built on them): the current tile, captions at the reading start with
//! marks, groups that open and close, group menus, the new item row and file drops from outside.
//!
//! The builders on [`TileGrid`](super::TileGrid) store these in a [`Strip`]; a grid that sets none
//! of them behaves as before.

use crate::behavior::{Axis, DisplayOrder, Reorder};
use crate::icons::{self, Icon, IconSize};
use crate::list::drop_line;
use crate::primitives::Interactive;
use crate::theme::{Tokens, TokensOf as _, UiTokens, role_format};
use craft_theme::TypeRole;
use egui::accesskit::Role;
use egui::text::LayoutJob;
use egui::{Galley, Painter, Rect, Stroke, StrokeKind, Ui, pos2, vec2};
use std::sync::Arc;

use super::layout::arrange_closed;
use super::{ArrangeSpacing, Mapper, Pass, TextureKey, TileContent, TileGroup, TileLayout, TileLook, TileSource, TileStyle, fit_name};

/// Where a caption goes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CaptionPlace {
    /// Under the picture (Bridge; Acrobat's page labels).
    #[default]
    Below,
    /// In a column at the picture's reading start, top aligned, `tile.start_caption_width` wide
    /// (PowerPoint's slide numbers): the name in `tile.caption` (`tile.caption_selected` when
    /// selected), crossed by a slash when [`TileContent::hidden`](super::TileContent::hidden), and
    /// the tile's [`marks`](super::TileContent::marks) under it, one per line, in `tile.detail`.
    /// The arrangement makes each card that much wider; no caption band under the picture.
    Start,
}

/// A glyph under a [`CaptionPlace::Start`] caption (PowerPoint's animation star).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TileMark {
    /// The glyph, drawn at `IconSize::Sm`.
    pub icon: Icon,
    /// Its tooltip, and a part of the tile's accessible name ("Has animations").
    pub tip: String,
}

/// What the strip builders set on one grid.
pub(super) struct Strip<'a, K> {
    pub current: Option<K>,
    pub caption_place: CaptionPlace,
    pub group_open: Option<&'a [bool]>,
    pub group_menus: bool,
    pub new_item: Option<String>,
    pub file_drops: bool,
}

impl<K> Default for Strip<'_, K> {
    fn default() -> Self {
        Self { current: None, caption_place: CaptionPlace::Below, group_open: None, group_menus: false, new_item: None, file_drops: false }
    }
}

impl<K> Strip<'_, K> {
    /// The width the `Start` caption column adds to each tile; 0 for any other place, and for
    /// `Cards` without captions.
    pub fn start_width(&self, style: &TileStyle, layout: TileLayout) -> f32 {
        match (self.caption_place, layout) {
            (CaptionPlace::Start, TileLayout::Cards { caption: true, .. } | TileLayout::Justified { .. }) => style.start_caption_width,
            _ => 0.0,
        }
    }

    /// The closed groups by index; empty when every group is open.
    pub fn closed(&self, groups: &[TileGroup]) -> Vec<bool> {
        let Some(open) = self.group_open else { return Vec::new() };
        let closed: Vec<bool> = (0..groups.len()).map(|g| !open.get(g).copied().unwrap_or(true)).collect();
        if closed.contains(&true) { closed } else { Vec::new() }
    }
}

/// [`arrange_closed`] for a grid whose cards are `start_width` wider: the pictures keep the size
/// the layout asks for and the column sits at their reading start (the arrangement is left to
/// right; the grid mirrors it).
pub(super) fn arrange_strip(
    aspects: &[f32],
    groups: &[TileGroup],
    closed: &[bool],
    width: f32,
    layout: TileLayout,
    spacing: ArrangeSpacing,
    start_width: f32,
) -> super::TileArrangement {
    let TileLayout::Cards { side, aspect, .. } = layout else { return arrange_closed(aspects, groups, closed, width, layout, spacing, start_width) };
    if start_width <= 0.0 {
        return arrange_closed(aspects, groups, closed, width, layout, spacing, 0.0);
    }
    let aspect = if aspect.is_finite() && aspect > 0.0 { aspect } else { 1.0 };
    let wide = side + start_width;
    let cards = TileLayout::Cards { side: wide, aspect: aspect * wide / side.max(f32::EPSILON), caption: false };
    let mut arrangement = arrange_closed(aspects, groups, closed, width, cards, spacing, 0.0);
    for tile in arrangement.tiles.iter_mut().filter(|t| **t != Rect::NOTHING) {
        tile.min.x += start_width;
    }
    arrangement
}

/// The caption column of the tile whose picture is `picture` (arrangement coordinates).
pub(super) fn start_column(picture: Rect, width: f32) -> Rect {
    Rect::from_min_max(pos2(picture.min.x - width, picture.min.y), pos2(picture.min.x, picture.max.y))
}

/// The current tile's ring, outside the picture edge (PdfCraft draws the current page's border
/// outside its thumbnail; over the selection ring when the tile is selected).
pub(super) fn ring_current(painter: &Painter, style: &TileStyle, picture: Rect) {
    // pdfcraft: crates/ui-egui/src/panels/pages.rs:176 (the current page's heavier border); deckcraft: crates/ui-egui/src/panels/thumbs.rs:230 (the selected slide's ring).
    painter.rect_stroke(picture, 0.0, Stroke::new(style.ring_current_width, style.ring_current), StrokeKind::Outside);
}

/// The colour of a `Start` caption's name.
fn name_color(style: &TileStyle, look: &TileLook<'_>) -> egui::Color32 {
    if look.selected { style.caption_selected } else { style.caption }
}

/// A `Start` caption's name shortened to the column, and where it goes: centred, below
/// `tile.caption_gap` from the column's top.
pub(super) fn start_name(painter: &Painter, tokens: &UiTokens, style: &TileStyle, look: &TileLook<'_>) -> Option<(Arc<Galley>, egui::Pos2)> {
    let column = look.caption?;
    let color = name_color(style, look);
    let layout = |text: &str| painter.layout_job(LayoutJob::single_section(text.to_owned(), role_format(tokens, TypeRole::Caption, color)));
    let shown = fit_name(&look.content.name, |t| layout(t).size().x <= column.width());
    let galley = layout(&shown);
    let at = pos2(column.center().x - galley.size().x / 2.0, column.top() + style.caption_gap);
    Some((galley, at))
}

/// The rects of the marks under a `Start` caption in `column`, one per line below the name.
pub(super) fn mark_rects(column: Rect, tokens: &UiTokens, style: &TileStyle, count: usize) -> Vec<Rect> {
    let glyph = IconSize::Sm.points(tokens);
    let line = tokens.typography.get(TypeRole::Caption).line_height;
    let first = column.top() + 2.0 * style.caption_gap + line;
    (0..count)
        .map(|n| Rect::from_min_size(pos2(column.center().x - glyph / 2.0, first + n as f32 * (glyph + style.caption_gap)), vec2(glyph, glyph)))
        .collect()
}

/// The tooltip of the mark under the pointer, when the pointer is over one.
pub(super) fn mark_tip(ui: &Ui, tokens: &UiTokens, style: &TileStyle, content: &TileContent, column: Option<Rect>) -> Option<String> {
    let pointer = ui.input(|i| i.pointer.hover_pos())?;
    let rects = mark_rects(column?, tokens, style, content.marks.len());
    rects.iter().zip(&content.marks).find(|(r, _)| r.contains(pointer)).map(|(_, mark)| mark.tip.clone())
}

/// [`paint_tile`](super::paint_tile)'s part for [`TileContent::hidden`](super::TileContent::hidden)
/// and [`TileContent::marks`](super::TileContent::marks): the veil over a hidden tile's picture
/// (`dimmed`'s), and with a `Start` caption the slash across the name and the marks.
pub(super) fn paint_hidden_and_marks(painter: &Painter, tokens: &UiTokens, style: &TileStyle, look: &TileLook<'_>, set: icons::IconSet) {
    // deckcraft: crates/ui-egui/src/panels/thumbs.rs:198 (the hidden slide's slash, its picture at alpha 140) and :202 (the animation star under the number).
    let content = look.content;
    if content.hidden {
        painter.rect_filled(look.picture, 0.0, style.veil);
    }
    if look.caption_place != CaptionPlace::Start {
        return;
    }
    let Some(column) = look.caption else { return };
    if content.hidden
        && let Some((galley, at)) = start_name(painter, tokens, style, look)
    {
        let name = Rect::from_min_size(at, galley.size());
        painter.line_segment([name.left_bottom(), name.right_top()], Stroke::new(IconSize::Sm.stroke(), name_color(style, look)));
    }
    let glyph = IconSize::Sm.points(tokens);
    for (rect, mark) in mark_rects(column, tokens, style, content.marks.len()).into_iter().zip(&content.marks) {
        icons::paint_on(painter, rect, mark.icon, set, glyph, IconSize::Sm.stroke(), style.detail);
    }
}

impl<'a, K, S> Pass<'a, '_, K, S>
where
    K: TextureKey + std::fmt::Debug,
    S: TileSource<K> + ?Sized,
{
    /// A new current key scrolls its tile into view as little as it takes, except during a drag
    /// (then the change counts as seen). Waits for the first pass that knows the view's height.
    pub(super) fn follow_current(&mut self, dragging: bool) {
        // pdfcraft: crates/ui-egui/src/panels/pages.rs:117 (`panel_followed`).
        if self.state.view.is_none() || self.state.current == self.strip.current {
            return;
        }
        if !dragging && let Some(at) = self.strip.current.as_ref().and_then(|k| DisplayOrder::position(&self.order, k)) {
            self.state.reveal = Some(at);
        }
        self.state.current.clone_from(&self.strip.current);
    }

    /// The new item row's rect in the arrangement: as wide as a card, `tile.group_header` high,
    /// below the last row.
    fn new_item_rect(&self) -> Rect {
        let inset = self.style.inset;
        let card = self
            .arr
            .tiles
            .iter()
            .find(|t| **t != Rect::NOTHING)
            .map_or(self.width - 2.0 * inset, |t| t.width() + self.start_width)
            .min(self.width - 2.0 * inset);
        Rect::from_min_size(pos2(inset, self.arr.height - inset + self.style.gap), vec2(card, self.style.group_header))
    }

    /// The new item row; whether it was activated (a click, or Enter or Space while focused).
    pub(super) fn new_item_row(&self, ui: &mut Ui, mapper: &Mapper) -> bool {
        // deckcraft: crates/ui-egui/src/panels/thumbs.rs:284 (the New Slide prompt).
        let Some(label) = &self.strip.new_item else { return false };
        let rect = mapper.rect(self.new_item_rect());
        let response = Interactive::with_id(self.id.with("new-item"), Role::Button, label.clone()).interact_at(ui, rect).finish(ui);
        if ui.is_rect_visible(rect) {
            let tokens = Tokens::of(ui);
            let color = self.style.detail;
            let layout =
                |text: &str| ui.painter().layout_job(LayoutJob::single_section(text.to_owned(), role_format(&tokens, TypeRole::Caption, color)));
            let galley = layout(&fit_name(label, |t| layout(t).size().x <= rect.width()));
            ui.painter().galley(rect.center() - galley.size() / 2.0, galley, color);
        }
        response.clicked()
    }

    /// Files dragged in from outside the app: the gap under the pointer gets the insert line and
    /// is reported in `file_gap`; near the edges the grid scrolls.
    pub(super) fn file_drop(&mut self, ui: &Ui, view: Rect, mapper: &Mapper) {
        // pdfcraft: crates/ui-egui/src/panels/pages.rs:277 (`panel_gap`).
        if !self.strip.file_drops || ui.input(|i| i.raw.hovered_files.is_empty()) {
            return;
        }
        let Some(pointer) = ui.input(|i| i.pointer.latest_pos()).filter(|p| view.contains(*p)) else { return };
        let rows = &self.arr.rows;
        let y = pointer.y - mapper.origin.y;
        let Some(last) = rows.len().checked_sub(1) else {
            self.file_gap = Some(0);
            return;
        };
        let row = &rows[rows.partition_point(|r| r.top + r.height < y).min(last)];
        // The first tile whose middle the pointer has not passed, in reading order. Each tile's
        // middle is its own card's (its `Start` column and its picture), so rows of tiles of
        // unequal width split at their own midpoints.
        let card = |i: usize| {
            let tile = mapper.rect(self.arr.tiles[i]);
            if self.rtl { tile.with_max_x(tile.right() + self.start_width) } else { tile.with_min_x(tile.left() - self.start_width) }
        };
        let gap = row
            .tiles
            .clone()
            .find(|&i| {
                let middle = card(i).center().x;
                if self.rtl { pointer.x > middle } else { pointer.x < middle }
            })
            .unwrap_or(row.tiles.end);
        self.file_gap = Some(gap);
        let half = self.style.gap / 2.0;
        let (x, tile) = if gap < row.tiles.end {
            let card = card(gap);
            (if self.rtl { card.right() + half } else { card.left() - half }, card)
        } else {
            let card = card(gap - 1);
            (if self.rtl { card.left() - half } else { card.right() + half }, card)
        };
        drop_line(ui, pos2(x, tile.top()), pos2(x, tile.bottom()));
        self.state.drift = Reorder::<K>::new(Axis::Vertical).auto_scroll(pointer, view, ui.input(|i| i.stable_dt));
    }
}
