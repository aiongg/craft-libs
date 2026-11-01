//! The List view: a tree `Table` (`Table::tree`) whose first column holds the bins.

use std::fmt::Debug;
use std::sync::Arc;

use craft_platform::open::natural_cmp;
use craft_theme::TypeRole;
use craft_ui::behavior::{
    Axis, DragEnd, DropPosition, ListDrag, ListSelection, Move, Reorder, ReorderGesture, RowInfo, Selection, SelectionChange, SelectionMode, StackKey,
};
use craft_ui::canvas::textures::{Priority, TextureKey, ThumbPaint, ThumbRequest};
use craft_ui::controls::{Badge, ControlSize, TextField};
use craft_ui::edit::EditEvent;
use craft_ui::icons::{self, Icon, IconSize};
use craft_ui::list::{
    CellContext, ContextMenuRequest, DropMark, ListRow, SortOrder, Table, TableEvent, TableLabels, TableSort, paint_drop_mark, paint_ghost,
};
use craft_ui::localized::{Labels as _, SharedLabels as _};
use craft_ui::theme::{ToEgui as _, Tokens, TokensOf as _, role_format};
use egui::os::OperatingSystem;
use egui::text::{CCursor, CCursorRange, LayoutJob};
use egui::text_edit::TextEditState;
use egui::{CornerRadius, DragAndDrop, Event, Id, IdSalt, Key, Label, Modifiers, PointerButton, Pos2, Rect, Response, Sense, Ui, vec2};

use super::{BinEntry, BinEvent, BinItem, BinLabels, BinModel, BinState, BinStyle, OpenHow, ThumbPair};
use crate::ui::grid::{TileLabels, TileStatus};

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
}

/// The rename editor open on one row.
#[derive(Clone, Debug)]
struct Rename<K> {
    key: K,
    draft: String,
    fresh: bool,
}

/// A click on the selected item's name, waiting out the double-click time.
#[derive(Clone, Debug)]
struct SlowClick<K> {
    key: K,
    since: f64,
}

/// What the pointer did in a cell, handled after the table (which holds the selection).
struct Hit<K> {
    key: K,
    kind: HitKind,
    modifiers: Modifiers,
    rect: Rect,
}

enum HitKind {
    Click { on_name: bool },
    Double,
    Menu(Option<Pos2>),
    DragStart,
}

/// The row a drag is over and where a drop on it would land.
struct RowDrop<K> {
    key: K,
    y: egui::Rangef,
    position: DropPosition,
    payload: Arc<ListDrag<K>>,
    valid: bool,
}

/// Draws the List view (see `MediaBin`'s doc and risk 7): a `Table` with `.tree(|key| name)` over
/// the rows in display order (a `DisplayOrder` of bins before items, sorted by the layout's sort,
/// open bins' children under them, with depth, `has_children` for bins and `open` from
/// `BinState::expanded`); the first cell's chip, thumbnail, icon and name after the table's
/// indent and triangle, the slow-click rename and the bin drop targets. `TableEvent::Open`
/// becomes `BinEvent::Expanded` (one per bin under the shown bin for `all`); the other table
/// events pass through as `BinEvent::Table`.
///
/// Every cell senses clicks and drags itself (the table's row senses only clicks), so a row can
/// start a drag; the selection rules, the double click and the context menu of those pointer
/// events are applied here with the same rules the table uses.
pub(super) fn list_view<K, M>(ui: &mut Ui, input: ListInput<'_, K, M>) -> (Response, Vec<BinEvent<K>>)
where
    K: TextureKey + Debug,
    M: BinModel<K> + ?Sized,
{
    // filmcraft: crates/ui-egui/src/panels/project_views.rs:497 (`list_view`), :687 (`list_bin`), :457 (`slow_click_rename`).
    let ListInput { id, model, state, selection, thumbnails, columns, labels } = input;
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
    let table_id = ui.make_persistent_id(IdSalt::new(salt));
    let focused = ui.memory(|m| m.has_focus(table_id));
    let tokens = Tokens::of(ui);
    let bin_style = BinStyle::of(&tokens);
    let row_style = ListRow::<K>::style(&tokens);
    let mac = matches!(ctx.os(), OperatingSystem::Mac | OperatingSystem::IOS);
    let now = ctx.input(|i| i.time);

    let rename_slot = table_id.with("rename");
    let slow_slot = table_id.with("slow");
    let mut rename: Option<Rename<K>> = ctx.data(|d| d.get_temp::<Option<Rename<K>>>(rename_slot)).flatten();
    let mut slow: Option<SlowClick<K>> = ctx.data(|d| d.get_temp::<Option<SlowClick<K>>>(slow_slot)).flatten();
    let mut gesture = ReorderGesture::<K>::load(ui, table_id);

    // Keys of the focused list that the table does not read.
    if focused && rename.is_none() {
        let (f2, remove, stack) = ctx.input(|i| {
            let stack = i.events.iter().find_map(|e| match e {
                Event::Key { key, pressed: true, modifiers, .. } => StackKey::from_key(*key, *modifiers, mac),
                _ => None,
            });
            (i.key_pressed(Key::F2), i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace), stack)
        });
        if f2 && let Some(lead) = selection.lead.clone().filter(|key| rows.iter().any(|row| row.key == *key)) {
            rename = Some(Rename { draft: item_of(model, &lead).name, key: lead, fresh: true });
        } else if remove && !selection.is_empty() {
            events.push(BinEvent::Remove(selection.items.clone()));
        } else if let (Some(stack), true) = (stack, user_order) {
            let modifiers = ctx.input(|i| i.modifiers);
            if let Some(mv) = Reorder::new(Axis::Vertical).key_move(rows.as_slice(), &selection.items, stack, modifiers) {
                gesture.set(mv);
            }
        }
    }

    let mut table_labels = TableLabels::of(ui);
    table_labels.empty = if searching { labels.no_matches.clone() } else { labels.empty.clone() };
    let row_height = Table::style(&tokens).row_height;
    let thumb_side = row_style.thumb_side(state.settings.thumbnails).min(row_height - 2.0 * row_style.thumb_pad.min(row_height / 4.0)).max(0.0);

    let mut cells = Cells {
        model,
        style: bin_style,
        icon_px: IconSize::Sm.points(&tokens),
        thumb_side,
        thumbnails: thumbnails.filter(|_| thumb_side > 0.0),
        rename: &mut rename,
        rename_label: labels.rename.as_str(),
        tile_labels: TileLabels::shared(ui),
        hits: Vec::new(),
        ends: Vec::new(),
        drop: None,
        user_order,
        pressed: false,
    };
    let columns_ref = columns;
    let out = Table::new(salt, labels.list_view.as_str(), columns_ref, &mut state.settings.columns, rows.as_slice())
        .selection(&mut *selection)
        .labels(table_labels)
        .tree(|key| item_of(model, key).name)
        .show(ui, |ui, key, column, cx| cells.cell(ui, key, column, cx));
    let Cells { hits, ends, drop, pressed, .. } = cells;
    let response = out.response;
    if pressed {
        ctx.memory_mut(|m| m.request_focus(table_id));
    }
    events.extend(ends);

    let modifiers = ctx.input(|i| i.modifiers);
    for event in out.events {
        match event {
            TableEvent::Selection(change) => events.push(BinEvent::Selection(change)),
            TableEvent::Activate(key) => events.push(open_event(model, key, modifiers)),
            TableEvent::ContextMenu(request) => events.push(BinEvent::ContextMenu(request)),
            TableEvent::Open(change) if change.all => {
                let mut bins = Vec::new();
                collect_bins(model, state.shown.as_ref(), 0, &mut bins);
                events.extend(bins.into_iter().map(|key| BinEvent::Expanded { key, open: change.open }));
            }
            TableEvent::Open(change) => events.push(BinEvent::Expanded { key: change.row, open: change.open }),
            other => events.push(BinEvent::Table(other)),
        }
    }

    let env = Env { model, rows: rows.as_slice(), table_id, user_order, now };
    apply_hits(&env, &ctx, hits, selection, &mut slow, &mut gesture, &mut events);
    settle_slow_click(&env, &ctx, selection, &mut slow, &mut rename);
    let (released, down, latest) = ctx.input(|i| (i.pointer.any_released(), i.pointer.any_down(), i.pointer.latest_pos()));
    let reorder_target = settle_drops(ui, drop, &response, released, &mut events);
    if gesture.is_active() {
        let has_payload = DragAndDrop::payload::<ListDrag<K>>(&ctx).is_some_and(|p| p.list == table_id);
        if released || !down {
            gesture.end(DragEnd::Released(reorder_target), modifiers);
        } else if !has_payload {
            gesture.end(DragEnd::PayloadCleared, modifiers);
        }
    }
    if let Some(event) = gesture.finish() {
        events.push(BinEvent::Reorder(event));
    }
    if let (Some(payload), Some(pointer)) = (DragAndDrop::payload::<ListDrag<K>>(&ctx).filter(|p| p.list == table_id), latest)
        && !released
    {
        paint_ghost(ui, table_id, &row_style, &payload.label, pointer, ui.available_width(), None);
    }

    ctx.data_mut(|d| {
        d.insert_temp(rename_slot, rename);
        d.insert_temp(slow_slot, slow);
    });
    if !events.is_empty() {
        ctx.request_repaint();
    }
    (response, events)
}

/// What the list's passes share.
struct Env<'a, K, M: ?Sized> {
    model: &'a M,
    rows: &'a [RowInfo<K>],
    table_id: Id,
    user_order: bool,
    now: f64,
}

/// Applies the pointer events of the cells with the selection rules the table uses.
fn apply_hits<K, M>(
    env: &Env<'_, K, M>,
    ctx: &egui::Context,
    hits: Vec<Hit<K>>,
    selection: &mut Selection<K>,
    slow: &mut Option<SlowClick<K>>,
    gesture: &mut ReorderGesture<K>,
    events: &mut Vec<BinEvent<K>>,
) where
    K: TextureKey + Debug,
    M: BinModel<K> + ?Sized,
{
    let Env { model, rows, table_id, user_order, now } = *env;
    let rules = ListSelection::new(SelectionMode::Multiple);
    for hit in hits {
        let Hit { key, kind, modifiers, rect } = hit;
        let Some(at) = rows.iter().position(|row| row.key == key) else { continue };
        match kind {
            HitKind::Click { on_name } => {
                let lone = selection.items.len() == 1 && selection.contains(&key);
                *slow = (lone && on_name && !modifiers.shift && !modifiers.command).then(|| SlowClick { key: key.clone(), since: now });
                if let Some(change) = rules.click(selection, rows, at, modifiers) {
                    apply(selection, change, events);
                }
            }
            HitKind::Double => {
                *slow = None;
                events.push(open_event(model, key, modifiers));
            }
            HitKind::Menu(pointer) => {
                *slow = None;
                let selected = selection.contains(&key);
                if !selected && let Some(change) = rules.click(selection, rows, at, Modifiers::NONE) {
                    apply(selection, change, events);
                }
                events.push(BinEvent::ContextMenu(ContextMenuRequest { row: Some(key), selected, anchor: rect, pointer, toggle: None }));
            }
            HitKind::DragStart => {
                *slow = None;
                if !selection.contains(&key) {
                    let pick = if modifiers.command { modifiers } else { Modifiers::NONE };
                    if let Some(change) = rules.click(selection, rows, at, pick) {
                        apply(selection, change, events);
                    }
                }
                let payload = ListDrag::new(table_id, rows, &selection.items, key.clone(), item_of(model, &key).name);
                if user_order {
                    gesture.begin();
                }
                events.push(BinEvent::Drag(payload.clone()));
                DragAndDrop::set_payload(ctx, payload);
            }
        }
    }
}

/// The slow click: the rename opens once the double-click time has passed without a second click.
fn settle_slow_click<K, M>(
    env: &Env<'_, K, M>,
    ctx: &egui::Context,
    selection: &Selection<K>,
    slow: &mut Option<SlowClick<K>>,
    rename: &mut Option<Rename<K>>,
) where
    K: TextureKey + Debug,
    M: BinModel<K> + ?Sized,
{
    let Some(pending) = slow.take() else { return };
    if !(selection.items.len() == 1 && selection.contains(&pending.key)) || rename.is_some() {
        return;
    }
    let due = pending.since + ctx.options(|o| o.input_options.max_double_click_delay);
    if env.now >= due {
        *rename = Some(Rename { draft: item_of(env.model, &pending.key).name, key: pending.key, fresh: true });
        ctx.request_repaint();
    } else {
        ctx.request_repaint_after_secs((due - env.now) as f32);
        *slow = Some(pending);
    }
}

/// Drops: the row under the pointer, else the view's background. Returns the move a release
/// between rows asks for.
fn settle_drops<K: TextureKey>(
    ui: &Ui,
    drop: Option<RowDrop<K>>,
    response: &Response,
    released: bool,
    events: &mut Vec<BinEvent<K>>,
) -> Option<Move<K>> {
    let mut reorder_target = None;
    if let Some(target) = drop.filter(|target| target.valid) {
        let line = Rect::from_x_y_ranges(response.rect.x_range(), target.y);
        match target.position {
            DropPosition::Into if released => events.push(BinEvent::Drop { items: target.payload.items.clone(), into: Some(target.key) }),
            DropPosition::Into => paint_drop_mark(ui, line, DropMark::Into),
            position if released => {
                reorder_target = Some(Move { items: target.payload.items.clone(), target: target.key, position, copy: false });
            }
            position => paint_drop_mark(ui, line, if position == DropPosition::Before { DropMark::Before } else { DropMark::After }),
        }
    } else if let Some(payload) = response.dnd_release_payload::<ListDrag<K>>() {
        events.push(BinEvent::Drop { items: payload.items.clone(), into: None });
    }
    reorder_target
}

/// Sets the selection a change gives and reports it.
fn apply<K: Clone>(selection: &mut Selection<K>, change: SelectionChange<K>, events: &mut Vec<BinEvent<K>>) {
    *selection = change.selection.clone();
    events.push(BinEvent::Selection(change));
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
        BinEvent::Open(key)
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
                let info = if model.count(&key) > 0 || open { RowInfo::group(key.clone(), open) } else { RowInfo::leaf(key.clone()) };
                rows.push(info.at_depth(depth));
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
    rename: &'a mut Option<Rename<K>>,
    rename_label: &'a str,
    tile_labels: Arc<TileLabels>,
    hits: Vec<Hit<K>>,
    ends: Vec<BinEvent<K>>,
    drop: Option<RowDrop<K>>,
    user_order: bool,
    pressed: bool,
}

impl<K, M> Cells<'_, K, M>
where
    K: TextureKey + Debug,
    M: BinModel<K> + ?Sized,
{
    fn cell(&mut self, ui: &mut Ui, key: &K, column: &String, cx: &CellContext) {
        let rect = ui.max_rect();
        // Registered first, so the cell's content is above it and takes its own input.
        let hit = ui.interact(rect, ui.scope_id().with("hit"), Sense::click_and_drag());
        self.track_drop(ui, key, &hit, rect);
        let name_rect = if column == NAME {
            self.name_cell(ui, key, cx)
        } else {
            let color = if cx.selected { cx.text } else { self.style.cell_text };
            let tokens = Tokens::of(ui);
            let format = role_format(&tokens, TypeRole::Body, color);
            ui.add(Label::new(LayoutJob::single_section(self.model.cell(key, column), format)).truncate().selectable(false));
            None
        };
        self.pressed |= hit.is_pointer_button_down_on();
        let modifiers = ui.input(|i| i.modifiers);
        let mut push = |kind| self.hits.push(Hit { key: key.clone(), kind, modifiers, rect });
        if hit.double_clicked() {
            push(HitKind::Double);
        } else if hit.clicked() {
            let on_name = name_rect.zip(hit.interact_pointer_pos()).is_some_and(|(name, pointer)| name.contains(pointer));
            push(HitKind::Click { on_name });
        }
        if hit.secondary_clicked() {
            push(HitKind::Menu(hit.interact_pointer_pos()));
        }
        if hit.drag_started_by(PointerButton::Primary) {
            push(HitKind::DragStart);
        }
    }

    /// Notes the row a drag is over: a bin takes the items into it; in User Order the edges of a
    /// row are before and after it.
    fn track_drop(&mut self, ui: &Ui, key: &K, hit: &Response, rect: Rect) {
        let Some(payload) = hit.dnd_hover_payload::<ListDrag<K>>() else { return };
        let Some(pointer) = ui.input(|i| i.pointer.latest_pos()) else { return };
        let is_bin = self.model.is_bin(key);
        let fraction = ((pointer.y - rect.top()) / rect.height().max(1.0)).clamp(0.0, 1.0);
        let edge = if is_bin { 0.25 } else { 0.5 };
        let position = if self.user_order && fraction < edge {
            DropPosition::Before
        } else if self.user_order && fraction >= 1.0 - edge {
            DropPosition::After
        } else if is_bin {
            DropPosition::Into
        } else {
            return;
        };
        let valid = if position == DropPosition::Into { !inside_any(self.model, key, &payload.items) } else { !payload.items.contains(key) };
        self.drop = Some(RowDrop { key: key.clone(), y: rect.y_range(), position, payload, valid });
    }

    /// The name column's cell: label chip, thumbnail, icon, name (or the rename field), badges and
    /// the status. Returns the name's rect.
    fn name_cell(&mut self, ui: &mut Ui, key: &K, cx: &CellContext) -> Option<Rect> {
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
        let color = if offline { self.style.offline } else { cx.text };
        let name_rect = if self.rename.as_ref().is_some_and(|r| r.key == *key) {
            Some(self.rename_field(ui, key, &item.name))
        } else {
            let format = role_format(&tokens, TypeRole::Body, color);
            let label = ui.add(Label::new(LayoutJob::single_section(item.name.clone(), format)).truncate().selectable(false));
            Some(label.rect)
        };
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
        name_rect
    }

    /// The rename editor: Enter and a click elsewhere commit, Esc cancels. Returns its rect.
    fn rename_field(&mut self, ui: &mut Ui, key: &K, name: &str) -> Rect {
        let Some(open) = self.rename.as_mut() else { return ui.max_rect() };
        let field = ui.scope_id().with("rename-field");
        if open.fresh {
            ui.memory_mut(|m| m.request_focus(field));
            let mut state = TextEditState::default();
            state.cursor.set_char_range(Some(CCursorRange::two(CCursor::new(0), CCursor::new(open.draft.chars().count()))));
            state.store(ui.ctx(), field);
        }
        let edit = TextField::new("rename", self.rename_label, &open.draft).size(ControlSize::Sm).id(field).width(ui.available_width()).show(ui);
        if let Some(
            EditEvent::Begin { value: Some(value), .. }
            | EditEvent::Update { value, .. }
            | EditEvent::Set { value, .. }
            | EditEvent::Commit { value, .. },
        ) = &edit.event
        {
            open.draft.clone_from(value);
        }
        let cancelled = edit.escaped || matches!(&edit.event, Some(EditEvent::Cancel { .. }));
        let committed = edit.submitted || (edit.response.lost_focus() && !open.fresh);
        open.fresh = false;
        if cancelled {
            *self.rename = None;
        } else if committed {
            let draft = std::mem::take(&mut open.draft);
            *self.rename = None;
            if !draft.is_empty() && draft != name {
                self.ends.push(BinEvent::Rename { key: key.clone(), name: draft });
            }
        }
        edit.response.rect
    }
}
