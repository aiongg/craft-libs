//! Display order and filtered order, `List` wiring, selection and thumbnail-click rules, rename,
//! activate, reveal, keys, `min_width`.

use std::sync::Arc;

use craft_ui::behavior::{Move, Reorder, RowInfo, Selection, SelectionPick};
use craft_ui::canvas::{SelectionOperation, selection_badge_beside_pointer};
use craft_ui::edit::EditEvent;
use craft_ui::list::{ContextMenuRequest, List, ListEvent, ListRow, OpenChange, RowEvent, ThumbTarget, ToggleColumn, ToggleEdit};
use craft_ui::menu::OpenAt;
use egui::{Context, CursorIcon, Id, Key, Modifiers, Response, Ui};

use super::drag::{drop_event, start_mask_drag};
use super::events::{ActivateOn, LayerColumn, LayerSelect, LayerToggle, MenuTarget};
use super::row::{ask_near_thumbnails, build_row, row_height};

use super::context::PanelContext;
use super::events::LayersEvent;
use super::menu::MenuRequest;
use crate::ui::filter::LayerFilter;
use crate::ui::tree::{LayerInfo, LayerPart, LayerRowKey, LayerSelection, LayerTree, SubRowGroup};

/// The panel's rows in display order: [`LayerTree::children`] from the top level, topmost first,
/// a group above its children; children of a closed group (`is_open` false and not in `opened`)
/// left out; under a layer whose `effects_open` is true, its Smart Filters rows (header, then
/// items) and its effects rows ([`LayerTree::sub_rows`]) as leaf rows one level deeper.
///
/// With an [`active`](LayerFilter::active) `filter`, every group's contents are listed whether
/// open or not, rows the filter doesn't [`show`](LayerFilter::shows) are left out (groups too,
/// since no kind matches them). A row's list depth is its number of shown ancestors, so the
/// list finds the right parent for Left, drops and the accessibility tree; its painted depth
/// ([`DisplayRows::paint_depths`]) stays the true one. Sub-rows follow their layer.
///
/// `opened` names groups the panel opens this pass (a reveal): they list their contents already.
/// Groups are rows with `has_children` (open as `is_open`, all open while filtered), so the list
/// draws their triangles and takes `Into` drops on them.
pub(super) fn display_rows<T: LayerTree>(tree: &T, filter: &LayerFilter, opened: &[T::Id]) -> DisplayRows<T::Id> {
    let filtered = filter.active();
    let mut out = DisplayRows { rows: Vec::new(), paint_depths: Vec::new() };
    let level = Level { shown: 0, true_depth: 0 };
    push_level(tree, filter, filtered, opened, tree.children(None), level, &mut out);
    out
}

/// What [`display_rows`] returns.
pub(super) struct DisplayRows<Id> {
    /// The rows, with the list's depth.
    pub(super) rows: Vec<RowInfo<LayerRowKey<Id>>>,
    /// Each row's depth as painted: its depth in the layer tree.
    pub(super) paint_depths: Vec<usize>,
}

/// [`display_rows`], kept in egui memory under `id` while [`LayerTree::revision`] and the filter
/// are what they were in the last pass; built again when either changes, when the filter is
/// active (it reads fields the revision doesn't cover), when the panel opens groups this pass
/// (`opened`) or when the tree gives no revision.
pub(super) fn shared_display_rows<T: LayerTree>(
    ctx: &Context,
    id: Id,
    tree: &T,
    filter: &LayerFilter,
    opened: &[T::Id],
) -> Arc<DisplayRows<T::Id>> {
    let revision = tree.revision().filter(|_| !filter.active() && opened.is_empty());
    let Some(revision) = revision else { return Arc::new(display_rows(tree, filter, opened)) };
    let key = id.with("display-rows");
    let cached = ctx.data(|d| d.get_temp::<CachedRows<T::Id>>(key));
    if let Some(cached) = cached.filter(|c| c.revision == revision && c.filter == *filter) {
        return cached.rows;
    }
    let rows = Arc::new(display_rows(tree, filter, opened));
    ctx.data_mut(|d| d.insert_temp(key, CachedRows { revision, filter: filter.clone(), rows: Arc::clone(&rows) }));
    rows
}

/// The list's selection for `selection`: the selected layers' rows in selection order, the lead
/// on the active layer's row. The anchor of a Shift range is kept in egui memory under `id` while
/// the app's selection is the one the list produced in the last pass, else it is the lead.
pub(super) fn list_selection<Id: Clone + PartialEq + Send + Sync + 'static>(
    ctx: &Context,
    id: egui::Id,
    selection: &LayerSelection<Id>,
) -> Selection<LayerRowKey<Id>> {
    let to_key = |layer: &Id| LayerRowKey::Layer(layer.clone());
    let lead = selection.active.as_ref().map(to_key);
    let anchor = ctx
        .data(|d| d.get_temp::<ProducedSelection<Id>>(id.with("anchor")))
        .filter(|produced| same_layers(&produced.layers, &selection.layers))
        .and_then(|produced| produced.anchor)
        .or_else(|| lead.clone());
    Selection { items: selection.layers.iter().map(to_key).collect(), lead, anchor }
}

/// What [`show_body`] returns.
pub(super) struct BodyOutput<Id> {
    /// The body's events in order, the row closures' (`cx.sink`) merged in where they happened.
    pub(super) events: Vec<LayersEvent<Id>>,
    /// While a drag is under way, where it would drop ([`ListOutput::drop_target`](craft_ui::list::ListOutput::drop_target)).
    pub(super) drop_target: Option<Move<LayerRowKey<Id>>>,
    /// A row menu or the eye's colour menu asked for this pass.
    pub(super) menu: Option<MenuRequest<Id>>,
    /// The list's response (its focus, its rect; the row menus open on it).
    pub(super) response: Response,
    /// The least width the rows need ([`min_width`]).
    pub(super) min_width: f32,
}

/// Draws the rows: a [`List`](craft_ui::list::List) in tree mode over `rows`, named
/// `LayersLabels::list`, Multiple selection from [`list_selection`], renamable where
/// `info.renamable` (`List::can_rename`), heights from [`row_height`](super::row::row_height),
/// each row from [`build_row`](super::row::build_row), with `reorder` when given, and maps what it
/// reports:
///
/// - `Selection` → [`LayersEvent::Select`] with the pick, the row's layer and the thumbnail the
///   click hit (`RowEvent::ThumbClicked` of the same pass: `First` is `Pixels`, `Extra(i)` the
///   `i`-th mask). A click with modifiers on a thumbnail that [`thumb_click_replaces_select`]
///   reports [`LayersEvent::ThumbClick`] instead, and the list's selection change of that click
///   is dropped. A click on the empty space is `Clear`. A right-click outside the selection
///   selects the row first (`Replace`).
/// - `Row(Toggle)` → [`LayersEvent::Toggle`] (`Visibility` for column 0, `Lock` for the lock
///   column); a gesture whose first value is on the eye with Alt held → [`LayersEvent::ShowOnly`]
///   once and nothing else.
/// - `Row(Renamed)` → [`LayersEvent::Rename`] with the trimmed text, dropped when empty or
///   unchanged; Tab and Shift+Tab continue on the next and previous renamable row (the list).
/// - `Activate` → [`LayersEvent::Activate`]: `Thumbnail(part)` when a `ThumbClicked` of the same
///   pass names a thumbnail, `Name` when the list reports a double click on the label
///   ([`RowEvent::LabelDoubleClicked`]) of a layer, else `Row` (also Enter).
/// - `Row(LinkToggled)` → [`LayersEvent::MaskLink`] for the mask at that extra thumbnail.
/// - `Open` → [`LayersEvent::Open`] for layer rows.
/// - `ContextMenu` → `menu`: with `toggle: Some(0)` (the eye column,
///   [`ContextMenuRequest::toggle`](craft_ui::list::ContextMenuRequest::toggle)) on a layer row the
///   colour menu for the selection or the row, `current` the colour the layers share; the eye of
///   an effects or Smart Filters row and the rest of any row open the row menu with `part` from
///   the thumbnail rects in `cx.sink.thumbs` under the pointer, `on_selection` from the request;
///   any other `toggle` reports nothing.
/// - `Reorder` → [`drop_event`](super::drag::drop_event).
/// - `Row(ThumbDrag)` → [`start_mask_drag`](super::drag::start_mask_drag).
/// - **Keys** while the list has the focus, besides the list's own: Shift+→ and Shift+← on the
///   lead report [`LayersEvent::FxOpen`] (Alt: `all`) when the lead layer has effects or Smart
///   Filters; the menu keys open the row menu at the lead.
/// - **Reveal**: `reveal` ([`reveal_plan`]) is the `Open` events for the closed ancestors of the
///   active layer when it differs from the last pass's (not on the panel's first pass) or of a
///   layer the app names, and the row to scroll to. The display order lists those groups as open
///   already, so the list reveals the row in the same pass.
/// - **Rename**: `rename` opens the rename field of that layer's row ([`List::start_rename`]).
/// - **Thumbnails**: after the list, [`ask_near_thumbnails`](super::row::ask_near_thumbnails).
/// - **Cursor**: while Cmd/Ctrl is held over a thumbnail that loads a selection, the pointing hand
///   with `CursorBadge::{Add, Subtract, Intersect}` for Shift, Alt, Shift+Alt.
pub(super) fn show_body<T: LayerTree>(
    ui: &mut Ui,
    cx: &PanelContext<'_, T>,
    display: &DisplayRows<T::Id>,
    reorder: Option<Reorder<'_, LayerRowKey<T::Id>>>,
    reveal: RevealPlan<T::Id>,
    rename: Option<T::Id>,
) -> BodyOutput<T::Id> {
    let tree = cx.tree;
    let rows = display.rows.as_slice();
    let salt = cx.id.with("list");
    let list_id = list_id(ui, cx.id);
    let mut selection = list_selection(&cx.ctx, cx.id, &cx.selection);
    let mut events: Vec<LayersEvent<T::Id>> = Vec::new();

    let reveal_key = reveal.target.filter(|id| rows.iter().any(|row| matches!(&row.key, LayerRowKey::Layer(k) if k == id))).map(LayerRowKey::Layer);
    events.extend(reveal.opens.into_iter().map(LayersEvent::Open));

    if let Some(event) = fx_key(ui, cx, list_id) {
        events.push(event);
    }

    let mut list = List::new(salt, cx.labels.list.clone(), rows)
        .tree(true)
        .selection(&mut selection)
        .heights(|at| row_height(cx, &rows[at], cx.options.thumbnail_size))
        .can_rename(|key| matches!(key, LayerRowKey::Layer(id) if tree.info(id).renamable));
    if let Some(reorder) = reorder {
        list = list.reorder(reorder);
    }
    if let Some(key) = reveal_key {
        list = list.reveal(key);
    }
    if let Some(layer) = rename {
        list = list.start_rename(LayerRowKey::Layer(layer));
    }
    let out = list.show(ui, |at, key| {
        let row = build_row(cx, at, key);
        // A filtered view lists rows at their shown depth and paints them at the true one.
        match display.paint_depths.get(at) {
            Some(&depth) if depth != rows[at].depth => row.paint_depth(depth),
            _ => row,
        }
    });

    let mut menu = None;
    map_events(ui, cx, list_id, &out.events, &mut events, &mut menu);
    events.append(&mut cx.sink.borrow_mut().events);

    ask_near_thumbnails(cx, rows);
    thumb_load_cursor(ui, cx, &out.response);

    BodyOutput { events, drop_target: out.drop_target, menu, min_width: min_width(ui, cx), response: out.response }
}

/// The id of the panel `id`'s list in `ui`.
pub(super) fn list_id(ui: &Ui, panel: Id) -> Id {
    List::id_in(ui, panel.with("list"))
}

/// Whether a click with `modifiers` on the thumbnail of `part` reports
/// [`LayersEvent::ThumbClick`] instead of selecting: Cmd/Ctrl on any thumbnail that loads a
/// selection (`info.thumb_loads_selection` for `Pixels`; masks always); Shift or Alt on a mask
/// (toggle, view); Alt alone on the pixel thumbnail (fit the layer on screen). Cmd/Ctrl on the
/// Background's thumbnail is a plain click (`Replace`).
pub(super) fn thumb_click_replaces_select(part: LayerPart, modifiers: Modifiers, info: &LayerInfo) -> bool {
    let on_mask = part != LayerPart::Pixels;
    if modifiers.command {
        return on_mask || info.thumb_loads_selection;
    }
    if on_mask { modifiers.shift || modifiers.alt } else { modifiers.alt && !modifiers.shift }
}

/// The `Open` events that reveal `active`: one per closed ancestor group, outermost first, when
/// `active` differs from `previous` and this is not the panel's first pass (`previous` is `None`
/// then). No event when every ancestor is open.
pub(super) fn reveal_opens<T: LayerTree>(tree: &T, active: Option<&T::Id>, previous: Option<Option<&T::Id>>) -> Vec<OpenChange<T::Id>> {
    let (Some(active), Some(previous)) = (active, previous) else { return Vec::new() };
    if previous == Some(active) {
        return Vec::new();
    }
    closed_ancestor_opens(tree, active)
}

/// The least width of the panel's rows: the [`ListRow::min_width_in_tree`](craft_ui::list::ListRow::min_width_in_tree)
/// of a group row one level deep with a layer thumbnail and one mask at the current thumbnail
/// size, the lock column and the app's trailing furniture when given. Thumbnails never shrink;
/// below it the name is cut.
pub(super) fn min_width<T: LayerTree>(ui: &Ui, cx: &PanelContext<'_, T>) -> f32 {
    let mut row = ListRow::new((), "")
        .toggle(ToggleColumn::eye("", true))
        .thumb(cx.options.thumbnail_size, |_, _| {})
        .extra_thumb(|_, _| {})
        .thumb_link(true, "");
    if cx.lock_column {
        row = row.toggle(ToggleColumn::lock("", false));
    }
    if let Some(trailing) = cx.trailing {
        row = row.trailing_item(trailing.width, |_| {});
    }
    row.min_width_in_tree(ui, true)
}

/// The display order kept between passes.
#[derive(Clone)]
struct CachedRows<Id> {
    revision: u64,
    filter: LayerFilter,
    rows: Arc<DisplayRows<Id>>,
}

/// The selection the list produced in the last pass, so a Shift range keeps its anchor while the
/// app's selection is that one.
#[derive(Clone)]
struct ProducedSelection<Id> {
    layers: Vec<Id>,
    anchor: Option<LayerRowKey<Id>>,
}

fn same_layers<Id: PartialEq>(a: &[Id], b: &[Id]) -> bool {
    a.len() == b.len() && a.iter().all(|id| b.contains(id))
}

/// A toggle gesture in progress: where it started, the `Begin` held back until a value says
/// where that is, and whether it turned out to be Show Only (its events are swallowed).
#[derive(Clone)]
pub(super) struct ToggleStart<Id> {
    id: craft_ui::edit::GestureId,
    start: Option<(LayerRowKey<Id>, LayerColumn)>,
    begin: Option<EditEvent<ToggleEdit<LayerRowKey<Id>>>>,
    show_only: bool,
}

fn leaf<Id>(key: LayerRowKey<Id>, depth: usize) -> RowInfo<LayerRowKey<Id>> {
    let mut row = RowInfo::leaf(key);
    row.depth = depth;
    row
}

/// The depths of the rows of one level: among the shown rows, and in the layer tree.
#[derive(Clone, Copy)]
struct Level {
    shown: usize,
    true_depth: usize,
}

impl<Id> DisplayRows<Id> {
    fn push(&mut self, row: RowInfo<LayerRowKey<Id>>, true_depth: usize) {
        self.rows.push(row);
        self.paint_depths.push(true_depth);
    }
}

/// Appends the rows of `children` (the layers of one level) and, unfiltered, of the open groups
/// among them.
fn push_level<T: LayerTree>(
    tree: &T,
    filter: &LayerFilter,
    filtered: bool,
    opened: &[T::Id],
    children: Vec<T::Id>,
    level: Level,
    out: &mut DisplayRows<T::Id>,
) {
    for id in children {
        let below = tree.children(Some(&id));
        let shown = !filtered || filter.shows(&tree.info(&id));
        let open = filtered || (!below.is_empty() && (tree.is_open(&id) || opened.contains(&id)));
        let at = out.rows.len();
        if shown {
            let mut row = leaf(LayerRowKey::Layer(id.clone()), level.shown);
            row.has_children = !below.is_empty();
            row.open = row.has_children && open;
            out.push(row, level.true_depth);
            push_sub_rows(tree, &id, Level { shown: level.shown + 1, true_depth: level.true_depth + 1 }, out);
        }
        let after_sub_rows = out.rows.len();
        if open {
            let inner = Level { shown: level.shown + usize::from(shown), true_depth: level.true_depth + 1 };
            push_level(tree, filter, filtered, opened, below, inner, out);
        }
        if shown && filtered {
            // A group with no match inside has nothing to open.
            out.rows[at].has_children = out.rows.len() > after_sub_rows;
            out.rows[at].open = out.rows[at].has_children;
        }
    }
}

/// The Smart Filters rows, then the effects rows, of a layer that shows them.
fn push_sub_rows<T: LayerTree>(tree: &T, id: &T::Id, level: Level, out: &mut DisplayRows<T::Id>) {
    if !tree.effects_open(id) {
        return;
    }
    let mut leaf_row = |key: LayerRowKey<T::Id>| out.push(leaf(key, level.shown), level.true_depth);
    if let Some(sub) = tree.sub_rows(id, SubRowGroup::SmartFilters) {
        leaf_row(LayerRowKey::Filters(id.clone(), None));
        (0..sub.items.len()).for_each(|i| leaf_row(LayerRowKey::Filters(id.clone(), Some(i))));
    }
    if let Some(sub) = tree.sub_rows(id, SubRowGroup::Effects) {
        leaf_row(LayerRowKey::Effects(id.clone(), None));
        (0..sub.items.len()).for_each(|i| leaf_row(LayerRowKey::Effects(id.clone(), Some(i))));
    }
}

/// `Open` events for the closed groups above `id`, outermost first.
fn closed_ancestor_opens<T: LayerTree>(tree: &T, id: &T::Id) -> Vec<OpenChange<T::Id>> {
    let mut opens = Vec::new();
    let mut current = tree.parent(id);
    while let Some(parent) = current {
        if !tree.is_open(&parent) {
            opens.push(OpenChange { row: parent.clone(), open: true, all: false });
        }
        current = tree.parent(&parent);
    }
    opens.reverse();
    opens
}

/// What this pass reveals: the layer the list scrolls to and the `Open` events for its closed
/// ancestors.
pub(super) struct RevealPlan<Id> {
    /// The layer to scroll into view, when it has a row.
    pub(super) target: Option<Id>,
    /// The `Open` events, outermost group first. The display order lists these groups as open
    /// already, so the row exists in this pass.
    pub(super) opens: Vec<OpenChange<Id>>,
}

impl<Id: Clone> RevealPlan<Id> {
    /// The groups the display order lists as open this pass.
    pub(super) fn opened(&self) -> Vec<Id> {
        self.opens.iter().map(|open| open.row.clone()).collect()
    }
}

/// The reveal of this pass. A new active layer (or the app's `named` layer) reveals its row and
/// opens its closed ancestors.
pub(super) fn reveal_plan<T: LayerTree>(cx: &PanelContext<'_, T>, named: Option<T::Id>) -> RevealPlan<T::Id> {
    let active = cx.selection.active.clone();
    let seen = cx.id.with("active-layer");
    let previous: Option<Option<T::Id>> = cx.ctx.data(|d| d.get_temp(seen));
    cx.ctx.data_mut(|d| d.insert_temp(seen, active.clone()));
    if let Some(id) = named {
        let opens = closed_ancestor_opens(cx.tree, &id);
        return RevealPlan { target: Some(id), opens };
    }
    let opens = reveal_opens(cx.tree, active.as_ref(), previous.as_ref().map(Option::as_ref));
    let changed = previous.as_ref().is_some_and(|p| *p != cx.selection.active);
    RevealPlan { target: active.filter(|_| changed), opens }
}

/// Shift+→ and Shift+← (Alt: every layer) on the active layer, when it has effects or Smart
/// Filters.
fn fx_key<T: LayerTree>(ui: &Ui, cx: &PanelContext<'_, T>, list_id: Id) -> Option<LayersEvent<T::Id>> {
    let layer = cx.selection.active.as_ref()?;
    if !ui.memory(|m| m.has_focus(list_id)) || !ui.input(|i| i.modifiers.shift && (i.key_pressed(Key::ArrowLeft) || i.key_pressed(Key::ArrowRight))) {
        return None;
    }
    let info = cx.tree.info(layer);
    if !(info.has_effects || info.has_smart_filters) {
        return None;
    }
    for all in [true, false] {
        let modifiers = if all { Modifiers::SHIFT | Modifiers::ALT } else { Modifiers::SHIFT };
        for (key, open) in [(Key::ArrowRight, true), (Key::ArrowLeft, false)] {
            if ui.input_mut(|i| i.consume_key(modifiers, key)) {
                return Some(LayersEvent::FxOpen { layer: layer.clone(), open, all });
            }
        }
    }
    None
}

/// The part a click on `which` of the row `key` names.
fn thumb_part<Id>(key: &LayerRowKey<Id>, info: &LayerInfo, which: ThumbTarget) -> LayerPart {
    match which {
        ThumbTarget::First if matches!(key, LayerRowKey::Filters(_, None)) => LayerPart::FilterMask,
        ThumbTarget::First => LayerPart::Pixels,
        ThumbTarget::Extra(i) => info.masks.get(i).map_or(LayerPart::Pixels, |mask| mask.part),
    }
}

fn thumb_click_of<Id>(events: &[ListEvent<LayerRowKey<Id>>], consumed: &[bool], row: &LayerRowKey<Id>) -> Option<(usize, ThumbTarget)>
where
    Id: PartialEq,
{
    events.iter().enumerate().find_map(|(i, event)| match event {
        ListEvent::Row(RowEvent::ThumbClicked { row: r, which }) if r == row && !consumed[i] => Some((i, *which)),
        _ => None,
    })
}

/// Turns what the list reported into the panel's events, in order.
fn map_events<T: LayerTree>(
    ui: &Ui,
    cx: &PanelContext<'_, T>,
    list_id: Id,
    list: &[ListEvent<LayerRowKey<T::Id>>],
    events: &mut Vec<LayersEvent<T::Id>>,
    menu: &mut Option<MenuRequest<T::Id>>,
) {
    let tree = cx.tree;
    let modifiers = ui.input(|i| i.modifiers);
    let mut consumed = vec![false; list.len()];
    let activated: Vec<&LayerRowKey<T::Id>> = list.iter().filter_map(|e| if let ListEvent::Activate(row) = e { Some(row) } else { None }).collect();
    let mut produced: Option<ProducedSelection<T::Id>> = None;
    let mut starts: Vec<ToggleStart<T::Id>> = cx.ctx.data(|d| d.get_temp(cx.id.with("toggles"))).unwrap_or_default();

    for (i, event) in list.iter().enumerate() {
        match event {
            ListEvent::Selection(change) => {
                let Some(row) = &change.row else {
                    events.push(LayersEvent::Select(LayerSelect { layer: None, pick: SelectionPick::Clear, part: None }));
                    produced = Some(ProducedSelection { layers: Vec::new(), anchor: None });
                    continue;
                };
                let layer = row.layer().clone();
                let mut part = None;
                if let Some((j, which)) = thumb_click_of(list, &consumed, row) {
                    consumed[j] = true;
                    let info = tree.info(&layer);
                    let hit = thumb_part(row, &info, which);
                    if thumb_click_replaces_select(hit, modifiers, &info) {
                        events.push(LayersEvent::ThumbClick { layer, part: hit, modifiers });
                        continue;
                    }
                    part = Some(hit);
                }
                events.push(LayersEvent::Select(LayerSelect { layer: Some(layer), pick: change.pick, part }));
                produced = Some(produced_selection(&change.selection));
            }
            ListEvent::Row(RowEvent::ThumbClicked { row, which }) if !consumed[i] && !activated.contains(&row) => {
                let layer = row.layer().clone();
                let info = tree.info(&layer);
                let part = thumb_part(row, &info, *which);
                if thumb_click_replaces_select(part, modifiers, &info) {
                    events.push(LayersEvent::ThumbClick { layer, part, modifiers });
                } else {
                    events.push(LayersEvent::Select(LayerSelect { layer: Some(layer), pick: SelectionPick::Replace, part: Some(part) }));
                }
            }
            ListEvent::Activate(row) => {
                let on = if let Some(which) = list.iter().find_map(|e| match e {
                    ListEvent::Row(RowEvent::ThumbClicked { row: r, which }) if r == row => Some(*which),
                    _ => None,
                }) {
                    ActivateOn::Thumbnail(thumb_part(row, &tree.info(row.layer()), which))
                } else if row.is_layer()
                    && list.iter().any(|e| matches!(e, ListEvent::Row(RowEvent::LabelDoubleClicked { row: r }) if r == row))
                {
                    ActivateOn::Name
                } else {
                    ActivateOn::Row
                };
                events.push(LayersEvent::Activate { row: row.clone(), on });
            }
            ListEvent::Open(change) => {
                if let LayerRowKey::Layer(id) = &change.row {
                    events.push(LayersEvent::Open(OpenChange { row: id.clone(), open: change.open, all: change.all }));
                }
            }
            ListEvent::ContextMenu(request) => context_menu(cx, request, events, menu, &mut produced),
            ListEvent::Reorder(edit) => events.extend(drop_event(edit.clone())),
            ListEvent::Row(RowEvent::Toggle(edit)) => toggle_event(edit, modifiers, &mut starts, events),
            ListEvent::Row(RowEvent::Renamed { row: LayerRowKey::Layer(id), text }) => {
                let text = text.trim();
                if !text.is_empty() && text != tree.info(id).name {
                    events.push(LayersEvent::Rename { layer: id.clone(), text: text.to_owned() });
                }
            }
            ListEvent::Row(RowEvent::LinkToggled { row, thumb, linked }) => {
                let info = tree.info(row.layer());
                if let Some(mask) = info.masks.get(*thumb) {
                    events.push(LayersEvent::MaskLink { layer: row.layer().clone(), part: mask.part, linked: *linked });
                }
            }
            ListEvent::Row(RowEvent::ThumbDrag { row, which }) => {
                let info = tree.info(row.layer());
                let part = thumb_part(row, &info, *which);
                let label = if part == LayerPart::VectorMask { &cx.labels.vector_mask_title } else { &cx.labels.layer_mask_title };
                start_mask_drag(ui, list_id, row.layer().clone(), part, label);
            }
            _ => {}
        }
    }
    if let Some(produced) = produced {
        cx.ctx.data_mut(|d| d.insert_temp(cx.id.with("anchor"), produced));
    }
    cx.ctx.data_mut(|d| d.insert_temp(cx.id.with("toggles"), starts));
}

fn produced_selection<Id: Clone + PartialEq>(selection: &Selection<LayerRowKey<Id>>) -> ProducedSelection<Id> {
    let mut layers: Vec<Id> = Vec::new();
    for key in &selection.items {
        if !layers.contains(key.layer()) {
            layers.push(key.layer().clone());
        }
    }
    ProducedSelection { layers, anchor: selection.anchor.as_ref().map(|key| LayerRowKey::Layer(key.layer().clone())) }
}

/// A right-click or the menu key: the eye's colour menu, or the row menu (after selecting a row
/// outside the selection).
fn context_menu<T: LayerTree>(
    cx: &PanelContext<'_, T>,
    request: &ContextMenuRequest<LayerRowKey<T::Id>>,
    events: &mut Vec<LayersEvent<T::Id>>,
    menu: &mut Option<MenuRequest<T::Id>>,
    produced: &mut Option<ProducedSelection<T::Id>>,
) {
    let Some(row) = &request.row else { return };
    let layer = row.layer();
    let at = request.pointer.map_or(OpenAt::Rect(request.anchor), OpenAt::Pointer);
    // A sub-row's eye is the row's own: its right-click opens the row menu like the rest of the row.
    let eye = request.toggle == Some(0) && matches!(row, LayerRowKey::Layer(_));
    if eye {
        let rows = if cx.selection.layers.contains(layer) { cx.selection.layers.clone() } else { vec![layer.clone()] };
        let mut labels = rows.iter().map(|id| cx.tree.info(id).label);
        let first = labels.next().flatten();
        let current = if labels.all(|label| label == first) { first } else { None };
        *menu = Some(MenuRequest::Eye { rows, current, at });
    } else if request.toggle.is_none() || matches!(row, LayerRowKey::Effects(..) | LayerRowKey::Filters(..)) {
        let part = request
            .pointer
            .and_then(|p| cx.sink.borrow().thumbs.iter().find(|(key, _, rect)| key == row && rect.contains(p)).map(|(_, part, _)| *part));
        if !request.selected {
            events.push(LayersEvent::Select(LayerSelect { layer: Some(layer.clone()), pick: SelectionPick::Replace, part }));
            *produced = Some(ProducedSelection { layers: vec![layer.clone()], anchor: Some(LayerRowKey::Layer(layer.clone())) });
        }
        let on_selection = request.selected && cx.selection.layers.len() > 1;
        *menu = Some(MenuRequest::Row { target: MenuTarget { row: row.clone(), part, on_selection }, at });
    }
}

/// The modifiers an edit carries; `None` for the phases that carry none.
fn edit_modifiers<T>(edit: &EditEvent<T>) -> Option<Modifiers> {
    match edit {
        EditEvent::Update { modifiers, .. } | EditEvent::Commit { modifiers, .. } | EditEvent::Set { modifiers, .. } => Some(*modifiers),
        _ => None,
    }
}

/// Maps an eye or lock edit. A sweep's `Begin` waits until a value names the row it started on.
/// A gesture whose first value is on an eye with Alt held is Show Only: it reports
/// [`LayersEvent::ShowOnly`] once and nothing else, however the gesture goes on.
fn toggle_event<Id: Clone + PartialEq>(
    edit: &EditEvent<ToggleEdit<LayerRowKey<Id>>>,
    held: Modifiers,
    starts: &mut Vec<ToggleStart<Id>>,
    events: &mut Vec<LayersEvent<Id>>,
) {
    let column_of = |value: &ToggleEdit<LayerRowKey<Id>>| if value.column == 0 { LayerColumn::Visibility } else { LayerColumn::Lock };
    let toggle = |row: &LayerRowKey<Id>, column, edit: &EditEvent<ToggleEdit<LayerRowKey<Id>>>| {
        LayersEvent::Toggle(LayerToggle { row: row.clone(), column, edit: edit.clone() })
    };
    let Some(gesture) = edit.id() else {
        if let EditEvent::Set { value, modifiers, .. } = edit {
            let column = column_of(value);
            events.push(if modifiers.alt && column == LayerColumn::Visibility {
                LayersEvent::ShowOnly(value.row.clone())
            } else {
                toggle(&value.row, column, edit)
            });
        }
        return;
    };
    let at = starts.iter().position(|s| s.id == gesture);
    if let Some(value) = edit.value() {
        let at = at.unwrap_or_else(|| {
            starts.push(ToggleStart { id: gesture, start: None, begin: None, show_only: false });
            starts.len() - 1
        });
        if starts[at].start.is_none() {
            let column = column_of(value);
            starts[at].start = Some((value.row.clone(), column));
            let alt = edit_modifiers(edit).unwrap_or(held).alt;
            if alt && column == LayerColumn::Visibility {
                starts[at].show_only = true;
                starts[at].begin = None;
                events.push(LayersEvent::ShowOnly(value.row.clone()));
            } else if let Some(begin) = starts[at].begin.take() {
                events.push(toggle(&value.row, column, &begin));
            }
        }
        if !starts[at].show_only {
            let (row, column) = starts[at].start.clone().expect("the start is set above");
            events.push(toggle(&row, column, edit));
        }
    } else if matches!(edit, EditEvent::Begin { .. }) {
        starts.push(ToggleStart { id: gesture, start: None, begin: Some(edit.clone()), show_only: false });
    } else if let Some(at) = at {
        let start = starts[at].start.clone().filter(|_| !starts[at].show_only);
        if let Some((row, column)) = start {
            events.push(toggle(&row, column, edit));
        }
    }
    if matches!(edit, EditEvent::Commit { .. } | EditEvent::Cancel { .. }) {
        starts.retain(|s| s.id != gesture);
    }
}

#[cfg(test)]
pub(super) fn toggle_event_for_tests<Id: Clone + PartialEq>(
    edit: &EditEvent<ToggleEdit<LayerRowKey<Id>>>,
    starts: &mut Vec<ToggleStart<Id>>,
    events: &mut Vec<LayersEvent<Id>>,
) {
    toggle_event(edit, Modifiers::NONE, starts, events);
}

/// While Cmd/Ctrl is held over a thumbnail that loads a selection: the pointing hand with the
/// badge of the operation the other modifiers ask for.
fn thumb_load_cursor<T: LayerTree>(ui: &Ui, cx: &PanelContext<'_, T>, list: &Response) {
    let modifiers = ui.input(|i| i.modifiers);
    let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) else { return };
    if !modifiers.command || !list.rect.contains(pointer) {
        return;
    }
    let over = cx.sink.borrow().thumbs.iter().find(|(_, _, rect)| rect.contains(pointer)).map(|(key, part, _)| (key.clone(), *part));
    let Some((key, part)) = over else { return };
    if part == LayerPart::Pixels && !cx.tree.info(key.layer()).thumb_loads_selection {
        return;
    }
    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    selection_badge_beside_pointer(ui.ctx(), pointer, SelectionOperation::from_modifiers(modifiers).badge());
}
