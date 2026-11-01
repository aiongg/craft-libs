//! The Icon view: cards with hover scrub and In and Out marks.

use std::fmt::Debug;

use craft_ui::behavior::{Axis, DisplayOrder, DropPosition, ListDrag, Reorder, Selection};
use craft_ui::canvas::textures::{Revision, TextureKey, ThumbRender, ThumbRequest, ThumbSource, Thumbnails};
use craft_ui::edit::{CancelReason, EditEvent};
use craft_ui::icons::{self, Icon, IconSize};
use craft_ui::list::{DropMark, SortOrder, TableSort, paint_drop_mark};
use craft_ui::theme::{Tokens, TokensOf as _};
use egui::{DragAndDrop, Id, Rect, Response, Ui};

use super::list::{inside_any, item_of, open_event, sort_entries};
use super::{BinEntry, BinEvent, BinLabels, BinModel, BinState, BinStyle, ThumbPair};
use crate::ui::grid::{BadgeVisibility, RatingKeys, SpaceAction, TileContent, TileEvent, TileGrid, TileLayout, TileSource, TileZoom};

/// The cards' picture is 16:9 (FilmCraft's).
pub(super) const ASPECT: f32 = 16.0 / 9.0;

/// What the Icon view reads and edits.
pub(super) struct IconInput<'a, K: TextureKey, M: ?Sized> {
    pub id: Id,
    pub model: &'a M,
    pub state: &'a BinState<K>,
    pub selection: &'a mut Selection<K>,
    pub thumbnails: Option<ThumbPair<'a, K>>,
    pub hover_scrub: bool,
    pub zoom_range: (f32, f32),
    pub default_size: f32,
    pub labels: &'a BinLabels,
}

/// The cards of the shown bin in display order: bins before items, in the model's order or the
/// Sort Icons order; while searching, the matching items of every sub-bin, flat. The flag is
/// whether the order is the model's own ("User Order", the only one that accepts a reorder).
pub(super) fn entries_of<K, M>(model: &M, state: &BinState<K>) -> (Vec<BinEntry<K>>, bool)
where
    K: Clone,
    M: BinModel<K> + ?Sized,
{
    let sort = state.settings.icon_sort.as_ref().map(|(column, descending)| TableSort {
        column: column.clone(),
        order: if *descending { SortOrder::Descending } else { SortOrder::Ascending },
    });
    let searching = !state.search.trim().is_empty();
    let mut found: Vec<BinEntry<K>> = Vec::new();
    if searching {
        let mut keys = Vec::new();
        super::search::collect_matches(model, state.shown.as_ref(), &state.search, 0, &mut keys);
        found.extend(keys.into_iter().map(BinEntry::Item));
    } else {
        model.children(state.shown.as_ref(), &mut found);
    }
    (sort_entries(model, sort.as_ref(), &mut found), sort.is_none() && !searching)
}

/// The cards as a `TileSource`; `stacks` (empty or one per card) are the Freeform view's "×N"
/// counts on a stack's first card.
pub(super) struct CardList<'a, K, M: ?Sized> {
    pub model: &'a M,
    pub keys: Vec<K>,
    pub bins: Vec<bool>,
    pub stacks: Vec<Option<usize>>,
}

impl<'a, K, M: ?Sized> CardList<'a, K, M> {
    pub fn new(model: &'a M, entries: Vec<BinEntry<K>>) -> Self {
        let bins = entries.iter().map(|entry| matches!(entry, BinEntry::Bin(_))).collect();
        let keys = entries.into_iter().map(|(BinEntry::Bin(key) | BinEntry::Item(key))| key).collect();
        Self { model, keys, bins, stacks: Vec::new() }
    }
}

impl<K, M: ?Sized> DisplayOrder<K> for CardList<'_, K, M> {
    fn len(&self) -> usize {
        self.keys.len()
    }

    fn key(&self, at: usize) -> &K {
        &self.keys[at]
    }
}

impl<K: Clone, M: BinModel<K> + ?Sized> TileSource<K> for CardList<'_, K, M> {
    fn aspect(&self, _at: usize) -> f32 {
        ASPECT
    }

    fn content(&self, at: usize, content: &mut TileContent) {
        let item = item_of(self.model, &self.keys[at]);
        content.name = item.name;
        content.label_color = item.label_color;
        content.badges = item.badges;
        content.status = item.status;
        content.stack = self.stacks.get(at).copied().flatten();
        if !self.bins[at] {
            content.detail = item.duration;
            content.range = item.range;
        }
    }
}

/// The app's thumbnails for items only: a bin has no picture (`Blank`), so its card shows the
/// folder.
pub(super) struct ItemThumbs<'a, K, M: ?Sized> {
    pub model: &'a M,
    pub inner: &'a dyn ThumbSource<K>,
}

impl<K, M: BinModel<K> + ?Sized> ThumbSource<K> for ItemThumbs<'_, K, M> {
    fn revision(&self, key: &K, request: &ThumbRequest) -> Revision {
        if self.model.is_bin(key) { Revision(0) } else { self.inner.revision(key, request) }
    }

    fn render(&self, key: &K, request: &ThumbRequest) -> ThumbRender {
        if self.model.is_bin(key) { ThumbRender::Blank } else { self.inner.render(key, request) }
    }
}

/// A bin card's picture: the placeholder fill with the folder glyph in `text.muted`.
pub(super) fn paint_bin_face<K: TextureKey>(ui: &Ui, picture: Rect, style: &BinStyle) {
    let tokens = Tokens::of(ui);
    ui.painter().rect_filled(picture, 0.0, Thumbnails::<K>::style(&tokens).placeholder);
    icons::paint(ui, picture, Icon::Folder, IconSize::Lg, style.bin_icon);
}

/// Draws the Icon view (see `MediaBin`'s doc): a `TileGrid` in `Cards { side: icon_size, aspect:
/// 16 / 9, caption: true }` over [`entries_of`], with the bins' folder face and drop outline,
/// hover scrub on items, In and Out bars (the grid's), and its events as `BinEvent`s.
///
/// - Enter and a double click are `Open` or `OpenBin { how }` (as the List view).
/// - A reorder gesture released over a bin card's middle is a drop into the bin (`Drop`, after a
///   `Reorder(Cancel)`); between cards in User Order it is `Reorder`; in a sorted order or a
///   search only drops into bins are offered. A release on the background is `Drop { into: None }`.
/// - Cmd/Ctrl + / − / 0 and a pinch are `Zoom(Set)` steps of the footer slider's range.
/// - The grid has no stars, flags or check boxes here (`RatingKeys::None`, `BadgeVisibility::Never`);
///   Space toggles the lead.
pub(super) fn icon_view<K, M>(ui: &mut Ui, input: IconInput<'_, K, M>) -> (Response, Vec<BinEvent<K>>)
where
    K: TextureKey + Debug,
    M: BinModel<K> + ?Sized,
{
    // filmcraft: crates/ui-egui/src/panels/project_views.rs:835 (`card`), :923 (`bin_card`), :821 (`hover_time`).
    let IconInput { id, model, state, selection, thumbnails, hover_scrub, zoom_range, default_size, labels } = input;
    let ctx = ui.ctx().clone();
    let tokens = Tokens::of(ui);
    let bin_style = BinStyle::of(&tokens);
    let (entries, user_order) = entries_of(model, state);
    let cards = CardList::new(model, entries);
    let size = state.settings.icon_size;

    let mut bin_rects: Vec<(K, Rect)> = Vec::new();
    let item_thumbs = thumbnails.map(|(thumbs, source)| (thumbs, ItemThumbs { model, inner: source }));
    let mut grid = TileGrid::new((id, "icon"), &cards)
        .layout(TileLayout::Cards { side: size, aspect: ASPECT, caption: true })
        .selection(&mut *selection)
        .hover_scrub(hover_scrub)
        .space(SpaceAction::Toggle)
        .rating_keys(RatingKeys::None)
        .badges(BadgeVisibility::Never)
        .label(labels.icon_view.as_str())
        .reorder(
            Reorder::new(Axis::Horizontal)
                .into_rows(|_, row| model.is_bin(row))
                .accepts(move |mv, _| if mv.position == DropPosition::Into { !inside_any(model, &mv.target, &mv.items) } else { user_order }),
        )
        .card_overlay(|ui, key, picture| {
            if model.is_bin(key) {
                paint_bin_face::<K>(ui, picture, &bin_style);
                bin_rects.push((key.clone(), picture));
            }
        });
    if let Some((thumbs, source)) = &item_thumbs {
        grid = grid.thumbnails(thumbs, source);
    }
    let out = grid.show(ui);
    let response = out.response;
    let modifiers = ctx.input(|i| i.modifiers);

    let zoom = TileZoom::Range { min: zoom_range.0, max: zoom_range.1, step: super::footer::ZOOM_STEP, default: default_size };
    let mut zoomed = size;
    let mut events: Vec<BinEvent<K>> = Vec::new();
    let mut dropped_in_bin = false;
    for event in out.events {
        match event {
            TileEvent::Selection(change) => events.push(BinEvent::Selection(change)),
            TileEvent::Activate(key) => events.push(open_event(model, key, modifiers)),
            TileEvent::Remove(keys) => events.push(BinEvent::Remove(keys)),
            TileEvent::Drag(drag) => events.push(BinEvent::Drag(drag)),
            TileEvent::Scrub { key, fraction } if !model.is_bin(&key) => events.push(BinEvent::Scrub { key, fraction }),
            TileEvent::ContextMenu(request) => events.push(BinEvent::ContextMenu(request)),
            TileEvent::Zoom(step) => {
                zoomed = zoom.step(zoomed, step);
                events.push(BinEvent::Zoom(EditEvent::set(f64::from(zoomed), modifiers)));
            }
            TileEvent::Reorder(EditEvent::Commit { id: gesture, value, .. }) if value.position == DropPosition::Into => {
                dropped_in_bin = true;
                if user_order {
                    events.push(BinEvent::Reorder(EditEvent::cancel(gesture, CancelReason::Dismissed)));
                }
                events.push(BinEvent::Drop { items: value.items, into: Some(value.target) });
            }
            TileEvent::Reorder(event) if user_order => events.push(BinEvent::Reorder(event)),
            _ => {}
        }
    }
    if !dropped_in_bin && let Some(payload) = response.dnd_release_payload::<ListDrag<K>>() {
        events.push(BinEvent::Drop { items: payload.items.clone(), into: None });
    }

    // The outline of the bin a drag would drop into.
    if let (Some(payload), Some(pointer)) = (DragAndDrop::payload::<ListDrag<K>>(&ctx), ctx.input(|i| i.pointer.latest_pos())) {
        let target = bin_rects.iter().find(|(key, rect)| rect.contains(pointer) && !inside_any(model, key, &payload.items));
        if let Some((_, rect)) = target {
            paint_drop_mark(ui, *rect, DropMark::Into);
        }
    }
    (response, events)
}
