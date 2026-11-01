//! The List view: a tree `Table` (`Table::tree`) whose first column holds the bins.

use std::fmt::Debug;
use std::sync::Arc;

use craft_platform::open::natural_cmp;
use craft_theme::TypeRole;
use craft_ui::behavior::{Axis, DropPosition, ListDrag, Reorder, RowInfo, Selection};
use craft_ui::canvas::textures::{Priority, TextureKey, ThumbPaint, ThumbRequest};
use craft_ui::controls::Badge;
use craft_ui::edit::{CancelReason, EditEvent};
use craft_ui::icons::{self, Icon, IconSize};
use craft_ui::list::{CellContext, DropMark, ListRow, OpenChange, SortOrder, Table, TableEvent, TableLabels, TableSort, paint_drop_mark};
use craft_ui::localized::{Labels as _, SharedLabels as _};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, role_format};
use egui::text::LayoutJob;
use egui::{CornerRadius, DragAndDrop, Event, Id, Key, Label, Modifiers, Rangef, Rect, Response, Sense, Ui, vec2};

use super::{BinEntry, BinEvent, BinItem, BinLabels, BinModel, BinState, BinStyle, MediaBin, OpenHow, ThumbPair};
use crate::ui::grid::{TileLabels, TileStatus, TileZoom, ZoomStep};

/// The key of the name column, the first of the host's columns.
const NAME: &str = "name";

/// Bins deeper than this are not drawn or searched, so a model that contains itself ends.
const MAX_DEPTH: usize = 64;

/// What the List view reads and edits.
pub(super) struct ListInput<'a, K: TextureKey, M: ?Sized> {
    pub id: Id,
    pub model: &'a M,
    pub state: &'a mut BinState<K>,
    pub selection: &'a mut Selection<K>,
    pub thumbnails: Option<ThumbPair<'a, K>>,
    pub columns: &'a [craft_ui::list::TableColumn<String>],
    pub labels: &'a BinLabels,
    pub zoom_range: (f32, f32),
    pub default_size: f32,
    pub rename: Option<K>,
}

/// The bin a foreign drag is over: its key and the rows' vertical range.
struct ForeignRow<K> {
    key: K,
    y: Rangef,
}

/// Draws the List view (see `MediaBin`'s doc): a `Table` with `.tree(|key| name)` over the rows
/// in display order (bins before items, sorted by the layout's sort, open bins' children under
/// them, with depth, a group for every bin and `open` from `BinState::expanded`); the first
/// cell's chip, thumbnail, icon and name after the table's indent and triangle. The table owns
/// the selection, the double click, the context menu, the row drag (`Table::reorder`) and the
/// rename (`Table::renamable`); this view maps its events to `BinEvent`s and takes the drops the
/// table leaves: a drag from another list over a bin row, and any release on the background.
/// `TableEvent::Open` becomes `BinEvent::Open` (one per bin under the shown bin for `all`); the
/// other table events pass through as `BinEvent::Table`.
pub(super) fn list_view<K, M>(ui: &mut Ui, input: ListInput<'_, K, M>) -> (Response, Vec<BinEvent<K>>)
where
    K: TextureKey + Debug,
    M: BinModel<K> + ?Sized,
{
    // filmcraft: crates/ui-egui/src/panels/project_views.rs:497 (`list_view`), :687 (`list_bin`), :457 (`slow_click_rename`).
    let ListInput { id, model, state, selection, thumbnails, columns, labels, zoom_range, default_size, rename } = input;
    let ctx = ui.ctx().clone();
    let mut events: Vec<BinEvent<K>> = Vec::new();

    let repaired = state.settings.columns.clone().repaired(columns);
    if repaired != state.settings.columns {
        state.settings.columns = repaired;
    }
    let searching = !state.search.trim().is_empty();
    let sort = state.settings.columns.sort.clone();
    let user_order = sort.is_none() && !searching;
    let rows = build_rows(model, state, sort.as_ref());

    let salt = (id, "list");
    let table_id = Table::id_in(ui, salt);
    let focused = ui.memory(|m| m.has_focus(table_id));
    let tokens = Tokens::of(ui);
    let bin_style = MediaBin::<'_, K, M>::style(&tokens);
    let row_style = ListRow::<K>::style(&tokens);
    let modifiers = ctx.input(|i| i.modifiers);

    // Keys of the focused list that the table does not read.
    if focused {
        let (remove, zoom) = ctx.input(|i| {
            let zoom = i.events.iter().find_map(|e| match e {
                Event::Key { key, pressed: true, modifiers, .. } if modifiers.command && !modifiers.alt => match key {
                    Key::Equals | Key::Plus => Some(ZoomStep::In),
                    Key::Minus => Some(ZoomStep::Out),
                    Key::Num0 => Some(ZoomStep::Reset),
                    _ => None,
                },
                _ => None,
            });
            (i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace), zoom)
        });
        if remove && !selection.is_empty() {
            events.push(BinEvent::Remove(selection.items.clone()));
        }
        if let Some(step) = zoom {
            let by = TileZoom::Range { min: zoom_range.0, max: zoom_range.1, step: super::footer::ZOOM_STEP, default: default_size };
            events.push(BinEvent::Zoom(EditEvent::set(f64::from(by.step(state.settings.icon_size, step)), modifiers)));
        }
    }

    let mut table_labels = TableLabels::of(ui);
    table_labels.empty = if searching { labels.no_matches.clone() } else { labels.empty.clone() };
    let thumb = state.settings.thumbnails;
    let thumb_side = row_style.thumb_side(thumb);

    let mut cells = Cells {
        model,
        style: bin_style,
        icon_px: IconSize::Sm.points(&tokens),
        thumb_side,
        thumbnails: thumbnails.filter(|_| thumb_side > 0.0),
        tile_labels: TileLabels::shared(ui),
        table_id,
        foreign: None,
    };
    let reorder = Reorder::new(Axis::Vertical)
        .accepts(|mv, _| if mv.position == DropPosition::Into { !inside_any(model, &mv.target, &mv.items) } else { user_order });
    let name_column = NAME.to_owned();
    let mut table = Table::new(salt, labels.list_view.as_str(), columns, &mut state.settings.columns, rows.as_slice())
        .selection(&mut *selection)
        .labels(table_labels)
        .tree(|key| item_of(model, key).name)
        .reorder(reorder)
        .renamable(name_column)
        .thumbnails(thumb);
    if let Some(key) = rename {
        table = table.start_rename(key);
    }
    let out = table.show(ui, |ui, key, column, cx| cells.cell(ui, key, column, cx));
    let foreign = cells.foreign;
    let response = out.response;

    let mut dropped = false;
    for event in out.events {
        match event {
            TableEvent::Selection(change) => events.push(BinEvent::Selection(change)),
            TableEvent::Activate(key) => events.push(open_event(model, key, modifiers)),
            TableEvent::ContextMenu(request) => events.push(BinEvent::ContextMenu(request)),
            TableEvent::Renamed { row, text } => {
                if !text.is_empty() {
                    events.push(BinEvent::Rename { key: row, text });
                }
            }
            TableEvent::RenameEnded { .. } => {}
            TableEvent::Open(change) if change.all => {
                let mut bins = Vec::new();
                collect_bins(model, state.shown.as_ref(), 0, &mut bins);
                events.extend(bins.into_iter().map(|row| BinEvent::Open(OpenChange { row, open: change.open, all: false })));
            }
            TableEvent::Open(change) => events.push(BinEvent::Open(change)),
            TableEvent::Reorder(event @ EditEvent::Begin { .. }) => {
                if let Some(payload) = DragAndDrop::payload::<ListDrag<K>>(&ctx).filter(|payload| payload.list == table_id) {
                    events.push(BinEvent::Drag(ListDrag::clone(&payload)));
                }
                if user_order {
                    events.push(BinEvent::Reorder(event));
                }
            }
            // A drop into a bin is a `Drop`; the gesture ends without a move.
            TableEvent::Reorder(EditEvent::Commit { id: gesture, value, .. }) if value.position == DropPosition::Into => {
                dropped = true;
                if user_order {
                    events.push(BinEvent::Reorder(EditEvent::cancel(gesture, CancelReason::Dismissed)));
                }
                if let Some(items) = drop_items(model, Some(&value.target), &value.items) {
                    events.push(BinEvent::Drop { items, into: Some(value.target) });
                }
            }
            TableEvent::Reorder(event) if user_order => events.push(BinEvent::Reorder(event)),
            TableEvent::Reorder(_) => {}
            other => events.push(BinEvent::Table(other)),
        }
    }

    // What the table leaves: a drag from another list over a bin row, else the background.
    let released = ctx.input(|i| i.pointer.any_released());
    let pointer = ctx.input(|i| i.pointer.latest_pos()).filter(|pointer| response.rect.contains(*pointer));
    let over_bin = foreign.filter(|row| pointer.is_some_and(|pointer| row.y.contains(pointer.y)));
    if !dropped && let Some(row) = over_bin {
        let payload = DragAndDrop::payload::<ListDrag<K>>(&ctx).filter(|payload| payload.list != table_id);
        if let Some(payload) = payload.filter(|payload| !inside_any(model, &row.key, &payload.items)) {
            if released {
                DragAndDrop::clear_payload(&ctx);
                dropped = true;
                if let Some(items) = drop_items(model, Some(&row.key), &payload.items) {
                    events.push(BinEvent::Drop { items, into: Some(row.key) });
                }
            } else {
                paint_drop_mark(ui, Rect::from_x_y_ranges(response.rect.x_range(), row.y), DropMark::Into);
            }
        }
    }
    if !dropped
        && let Some(payload) = response.dnd_release_payload::<ListDrag<K>>()
        && let Some(items) = drop_items(model, state.shown.as_ref(), &payload.items)
    {
        events.push(BinEvent::Drop { items, into: None });
    }

    if !events.is_empty() {
        ctx.request_repaint();
    }
    (response, events)
}

/// The `items` a drop into the bin `into` (`None`: the root, or the shown bin's background when
/// that is `None`) moves: those the model holds elsewhere. `None` when there are none or `into`
/// is, or lies inside, one of `items`: no empty undo step and no bin inside itself (FilmCraft's
/// `accept_view_drop`). The background of the shown bin passes the shown bin as `into`.
pub(super) fn drop_items<K: Clone + PartialEq, M: BinModel<K> + ?Sized>(model: &M, into: Option<&K>, items: &[K]) -> Option<Vec<K>> {
    if into.is_some_and(|bin| inside_any(model, bin, items)) {
        return None;
    }
    let moved: Vec<K> = items.iter().filter(|key| model.parent(key).as_ref() != into).cloned().collect();
    (!moved.is_empty()).then_some(moved)
}

/// Whether `key` is `shown` or lies inside it; every key is inside the root.
pub(super) fn within<K: Clone + PartialEq, M: BinModel<K> + ?Sized>(model: &M, key: &K, shown: Option<&K>) -> bool {
    let Some(shown) = shown else { return true };
    let mut at = model.parent(key);
    for _ in 0..=MAX_DEPTH {
        let Some(parent) = at else { return false };
        if parent == *shown {
            return true;
        }
        at = model.parent(&parent);
    }
    false
}

/// What an Enter or a double click on `key` opens, and how.
pub(super) fn open_event<K, M: BinModel<K> + ?Sized>(model: &M, key: K, modifiers: Modifiers) -> BinEvent<K> {
    if model.is_bin(&key) {
        let how = if modifiers.command {
            OpenHow::NewWindow
        } else if modifiers.shift || modifiers.alt {
            OpenHow::NewTab
        } else {
            OpenHow::InPlace
        };
        BinEvent::OpenBin { key, how }
    } else {
        BinEvent::Activate(key)
    }
}

pub(super) fn item_of<K, M: BinModel<K> + ?Sized>(model: &M, key: &K) -> BinItem {
    let mut item = BinItem::default();
    model.item(key, &mut item);
    item
}

/// The rows in display order: the shown bin's children, bins before items, each open bin's
/// children under it; or, while searching, the matching items of every sub-bin.
fn build_rows<K, M>(model: &M, state: &BinState<K>, sort: Option<&TableSort<String>>) -> Vec<RowInfo<K>>
where
    K: Clone + Eq + std::hash::Hash,
    M: BinModel<K> + ?Sized,
{
    let mut rows = Vec::new();
    if state.search.trim().is_empty() {
        push_children(model, state, state.shown.as_ref(), sort, 0, &mut rows);
    } else {
        let mut found = Vec::new();
        super::search::collect_matches(model, state.shown.as_ref(), &state.search, 0, &mut found);
        let entries = sort_entries(model, sort, &mut found.into_iter().map(BinEntry::Item).collect::<Vec<_>>());
        for entry in entries {
            let (BinEntry::Item(key) | BinEntry::Bin(key)) = entry;
            rows.push(RowInfo::leaf(key));
        }
    }
    rows
}

fn push_children<K, M>(model: &M, state: &BinState<K>, parent: Option<&K>, sort: Option<&TableSort<String>>, depth: usize, rows: &mut Vec<RowInfo<K>>)
where
    K: Clone + Eq + std::hash::Hash,
    M: BinModel<K> + ?Sized,
{
    let mut children = Vec::new();
    model.children(parent, &mut children);
    for entry in sort_entries(model, sort, &mut children) {
        match entry {
            BinEntry::Bin(key) => {
                let open = state.expanded.contains(&key);
                // Every bin is a group, so an empty one has its triangle and takes drops into it.
                rows.push(RowInfo::group(key.clone(), open).at_depth(depth));
                if open && depth < MAX_DEPTH {
                    push_children(model, state, Some(&key), sort, depth + 1, rows);
                }
            }
            BinEntry::Item(key) => rows.push(RowInfo::leaf(key).at_depth(depth)),
        }
    }
}

/// `entries` with bins before items, each kind in the model's order or sorted by `sort`.
pub(super) fn sort_entries<K: Clone, M: BinModel<K> + ?Sized>(
    model: &M,
    sort: Option<&TableSort<String>>,
    entries: &mut Vec<BinEntry<K>>,
) -> Vec<BinEntry<K>> {
    let (mut bins, mut items): (Vec<_>, Vec<_>) = std::mem::take(entries).into_iter().partition(|entry| matches!(entry, BinEntry::Bin(_)));
    if let Some(sort) = sort {
        let text = |entry: &BinEntry<K>| {
            let (BinEntry::Bin(key) | BinEntry::Item(key)) = entry;
            if sort.column == NAME { item_of(model, key).name } else { model.cell(key, &sort.column) }
        };
        for group in [&mut bins, &mut items] {
            let mut keyed: Vec<(String, BinEntry<K>)> = group.drain(..).map(|entry| (text(&entry), entry)).collect();
            keyed.sort_by(|a, b| {
                let order = natural_cmp(&a.0, &b.0);
                if sort.order == SortOrder::Descending { order.reverse() } else { order }
            });
            group.extend(keyed.into_iter().map(|(_, entry)| entry));
        }
    }
    bins.append(&mut items);
    bins
}

/// Every bin under `parent`, sub-bins included.
fn collect_bins<K: Clone, M: BinModel<K> + ?Sized>(model: &M, parent: Option<&K>, depth: usize, out: &mut Vec<K>) {
    if depth > MAX_DEPTH {
        return;
    }
    let mut children = Vec::new();
    model.children(parent, &mut children);
    for child in children {
        if let BinEntry::Bin(key) = child {
            out.push(key.clone());
            collect_bins(model, Some(&key), depth + 1, out);
        }
    }
}

/// Whether `bin` is one of `items` or inside one of them.
pub(super) fn inside_any<K: Clone + PartialEq, M: BinModel<K> + ?Sized>(model: &M, bin: &K, items: &[K]) -> bool {
    let mut at = Some(bin.clone());
    for _ in 0..=MAX_DEPTH {
        let Some(key) = at else { return false };
        if items.contains(&key) {
            return true;
        }
        at = model.parent(&key);
    }
    false
}

/// The cell closure's state.
struct Cells<'a, K: TextureKey, M: ?Sized> {
    model: &'a M,
    style: BinStyle,
    icon_px: f32,
    thumb_side: f32,
    thumbnails: Option<ThumbPair<'a, K>>,
    tile_labels: Arc<TileLabels>,
    table_id: Id,
    foreign: Option<ForeignRow<K>>,
}

impl<K, M> Cells<'_, K, M>
where
    K: TextureKey + Debug,
    M: BinModel<K> + ?Sized,
{
    fn cell(&mut self, ui: &mut Ui, key: &K, column: &String, cx: &CellContext) {
        self.note_foreign_drag(ui, key);
        if column == NAME {
            self.name_cell(ui, key, cx);
        } else {
            let mut cx = *cx;
            if !cx.selected {
                cx.text = self.style.cell_text;
            }
            cx.label(ui, self.model.cell(key, column));
        }
    }

    /// Notes the bin row a drag from another list is over: the table resolves only its own
    /// drags, so a bin takes the others here.
    fn note_foreign_drag(&mut self, ui: &Ui, key: &K) {
        if self.foreign.is_some() || !self.model.is_bin(key) {
            return;
        }
        let foreign = DragAndDrop::payload::<ListDrag<K>>(ui.ctx()).is_some_and(|payload| payload.list != self.table_id);
        let Some(pointer) = ui.input(|i| i.pointer.latest_pos()).filter(|_| foreign) else { return };
        let y = ui.max_rect().y_range();
        if y.contains(pointer.y) && ui.clip_rect().y_range().contains(pointer.y) {
            self.foreign = Some(ForeignRow { key: key.clone(), y });
        }
    }

    /// The name column's cell: label chip, thumbnail, icon, name (the table's rename field while
    /// the row renames), badges and the status.
    fn name_cell(&mut self, ui: &mut Ui, key: &K, cx: &CellContext) {
        let item = item_of(self.model, key);
        let is_bin = self.model.is_bin(key);
        let offline = item.status != TileStatus::Normal;
        let tokens = Tokens::of(ui);
        if let Some(color) = item.label_color {
            let (rect, _) = ui.allocate_exact_size(vec2(self.style.chip, self.style.chip), Sense::hover());
            ui.painter().rect_filled(rect, CornerRadius::from(self.style.chip_radius), color.to_egui());
        }
        if !is_bin && let Some((thumbs, source)) = self.thumbnails {
            let (rect, _) = ui.allocate_exact_size(vec2(self.thumb_side, self.thumb_side), Sense::hover());
            if ui.is_rect_visible(rect) {
                let request = ThumbRequest::for_cell(self.thumb_side, ui.pixels_per_point());
                let state = thumbs.state(ui.ctx(), source, key, &request, Priority::VISIBLE);
                thumbs.paint(ui.painter(), rect, &state, &ThumbPaint::default().checker(false));
            }
        }
        let glyph = if is_bin { Some(Icon::Folder) } else { item.icon };
        if let Some(icon) = glyph {
            let (rect, _) = ui.allocate_exact_size(vec2(self.icon_px, self.icon_px), Sense::hover());
            let color = if is_bin {
                self.style.bin_icon
            } else if offline {
                self.style.offline
            } else {
                self.style.cell_text
            };
            icons::paint(ui, rect, icon, IconSize::Sm, color);
        }
        let mut name = *cx;
        if offline {
            name.text = self.style.offline;
        }
        name.name(ui, item.name.as_str());
        if cx.renaming {
            return;
        }
        for badge in &item.badges {
            match &badge.kind {
                super::super::grid::BadgeKind::Text(text) => {
                    ui.add(Badge::new(text.as_str()).tone(badge.tone)).on_hover_text(&badge.tip);
                }
                super::super::grid::BadgeKind::Glyph(icon) => {
                    let (rect, response) = ui.allocate_exact_size(vec2(self.icon_px, self.icon_px), Sense::hover());
                    icons::paint(ui, rect, *icon, IconSize::Sm, self.style.cell_text);
                    response.on_hover_text(&badge.tip);
                }
            }
        }
        if offline {
            let text = match &item.status {
                TileStatus::Offline(text) => text.clone(),
                TileStatus::Unreadable => self.tile_labels.unreadable.clone(),
                _ => self.tile_labels.failed.clone(),
            };
            let (rect, _) = ui.allocate_exact_size(vec2(self.icon_px, self.icon_px), Sense::hover());
            icons::paint(ui, rect, if item.status == TileStatus::Failed { Icon::Warning } else { Icon::Unlink }, IconSize::Sm, self.style.offline);
            let format = role_format(&tokens, TypeRole::Caption, self.style.offline);
            ui.add(Label::new(LayoutJob::single_section(text, format)).truncate().selectable(false));
        }
    }
}
