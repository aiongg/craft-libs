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
/// a group above its children; children of a closed group (`is_open` false) left out; under a
/// layer whose `effects_open` is true, its Smart Filters rows (header, then items) and its
/// effects rows ([`LayerTree::sub_rows`]) as leaf rows one level deeper.
///
/// With an [`active`](LayerFilter::active) `filter`, every group's contents are listed whether
/// open or not, rows the filter doesn't [`show`](LayerFilter::shows) are left out (groups too,
/// since no kind matches them), and the rest keep their depth. Sub-rows follow their layer.
///
/// Groups are rows with `has_children` (open as `is_open`, all open while filtered), so the list
/// draws their triangles and takes `Into` drops on them.
pub(super) fn display_rows<T: LayerTree>(tree: &T, filter: &LayerFilter) -> Vec<RowInfo<LayerRowKey<T::Id>>> {
    let filtered = filter.active();
    let mut rows = Vec::new();
    push_level(tree, filter, filtered, tree.children(None), 0, &mut rows);
    rows
}

/// [`display_rows`], kept in egui memory under `id` while [`LayerTree::revision`] and the filter
/// are what they were in the last pass; built again when either changes or the tree gives no
/// revision.
pub(super) fn shared_display_rows<T: LayerTree>(ctx: &Context, id: Id, tree: &T, filter: &LayerFilter) -> Arc<Vec<RowInfo<LayerRowKey<T::Id>>>> {
    let Some(revision) = tree.revision() else { return Arc::new(display_rows(tree, filter)) };
    let key = id.with("display-rows");
    let cached = ctx.data(|d| d.get_temp::<CachedRows<T::Id>>(key));
    if let Some(cached) = cached.filter(|c| c.revision == revision && c.filter == *filter) {
        return cached.rows;
    }
    let rows = Arc::new(display_rows(tree, filter));
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
    /// Whether a group row was shown, for the panel menu's Collapse All Groups.
    pub(super) any_group: bool,
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
///   column); an `Update` or `Set` with Alt on the eye → [`LayersEvent::ShowOnly`] instead.
/// - `Row(Renamed)` → [`LayersEvent::Rename`] with the trimmed text, dropped when empty or
///   unchanged; Tab and Shift+Tab continue on the next and previous renamable row (the list).
/// - `Activate` → [`LayersEvent::Activate`]: `Thumbnail(part)` when a `ThumbClicked` of the same
///   pass names a thumbnail, `Name` for a double click on the label of a non-renamable layer, else
///   `Row` (also Enter).
/// - `Row(LinkToggled)` → [`LayersEvent::MaskLink`] for the mask at that extra thumbnail.
/// - `Open` → [`LayersEvent::Open`] for layer rows.
/// - `ContextMenu` → `menu`: with `toggle: Some(0)` (the eye column,
///   [`ContextMenuRequest::toggle`](craft_ui::list::ContextMenuRequest::toggle)) the colour menu
///   for the selection or the row; any other `toggle` reports nothing; else the row menu with `part` from the thumbnail
///   rects in `cx.sink.thumbs` under the pointer, `on_selection` from the request.
/// - `Reorder` → [`drop_event`](super::drag::drop_event).
/// - `Row(ThumbDrag)` → [`start_mask_drag`](super::drag::start_mask_drag).
/// - **Keys** while the list has the focus, besides the list's own: Shift+→ and Shift+← on the
///   lead report [`LayersEvent::FxOpen`] (Alt: `all`) when the lead layer has effects or Smart
///   Filters; the menu keys open the row menu at the lead.
/// - **Reveal**: when the active layer differs from the last pass's (not on the panel's first
///   pass), [`reveal_opens`] reports `Open` events for its closed ancestors and the list reveals
///   its row; `reveal` does the same for a layer the app names.
/// - **Thumbnails**: after the list, [`ask_near_thumbnails`](super::row::ask_near_thumbnails).
/// - **Cursor**: while Cmd/Ctrl is held over a thumbnail that loads a selection, the pointing hand
///   with `CursorBadge::{Add, Subtract, Intersect}` for Shift, Alt, Shift+Alt.
pub(super) fn show_body<T: LayerTree>(
    ui: &mut Ui,
    cx: &PanelContext<'_, T>,
    rows: &[RowInfo<LayerRowKey<T::Id>>],
    reorder: Option<Reorder<'_, LayerRowKey<T::Id>>>,
    reveal: Option<T::Id>,
) -> BodyOutput<T::Id> {
    let tree = cx.tree;
    let salt = cx.id.with("list");
    let list_id = ui.make_persistent_id(salt);
    let mut selection = list_selection(&cx.ctx, cx.id, &cx.selection);
    let mut events: Vec<LayersEvent<T::Id>> = Vec::new();

    let reveal_key = reveal_target(cx, rows, reveal, &mut events);

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
    let out = list.show(ui, |at, key| build_row(cx, at, key));

    let mut menu = None;
    map_events(ui, cx, list_id, &out.events, &mut events, &mut menu);
    events.append(&mut cx.sink.borrow_mut().events);

    ask_near_thumbnails(cx, rows);
    thumb_load_cursor(ui, cx, &out.response);

    BodyOutput {
        events,
        drop_target: out.drop_target,
        menu,
        min_width: min_width(ui, cx),
        any_group: rows.iter().any(|row| row.has_children),
        response: out.response,
    }
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

/// The least width of the panel's rows: the [`ListRow::min_width`](craft_ui::list::ListRow::min_width)
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
    // The group's disclosure column is the list's own and not part of a row the host builds.
    row.min_width(ui) + cx.row_style.disclosure
}

/// The display order kept between passes.
#[derive(Clone)]
struct CachedRows<Id> {
    revision: u64,
    filter: LayerFilter,
    rows: Arc<Vec<RowInfo<LayerRowKey<Id>>>>,
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

/// A toggle gesture in progress: where it started, and the `Begin` held back until a value says
/// where that is.
#[derive(Clone)]
struct ToggleStart<Id> {
    id: craft_ui::edit::GestureId,
    start: Option<(LayerRowKey<Id>, LayerColumn)>,
    begin: Option<EditEvent<ToggleEdit<LayerRowKey<Id>>>>,
}

fn leaf<Id>(key: LayerRowKey<Id>, depth: usize) -> RowInfo<LayerRowKey<Id>> {
    let mut row = RowInfo::leaf(key);
    row.depth = depth;
    row
}

/// Appends the rows of `children` (the layers of one level, at `depth`) and, unfiltered, of the
/// open groups among them.
fn push_level<T: LayerTree>(
    tree: &T,
    filter: &LayerFilter,
    filtered: bool,
    children: Vec<T::Id>,
    depth: usize,
    rows: &mut Vec<RowInfo<LayerRowKey<T::Id>>>,
) {
    for id in children {
        let below = tree.children(Some(&id));
        let shown = !filtered || filter.shows(&tree.info(&id));
        let open = filtered || (!below.is_empty() && tree.is_open(&id));
        let at = rows.len();
        if shown {
            let mut row = leaf(LayerRowKey::Layer(id.clone()), depth);
            row.has_children = !below.is_empty();
            row.open = row.has_children && open;
            rows.push(row);
            push_sub_rows(tree, &id, depth + 1, rows);
        }
        let after_sub_rows = rows.len();
        if open {
            push_level(tree, filter, filtered, below, depth + 1, rows);
        }
        if shown && filtered {
            // A group with no match inside has nothing to open.
            rows[at].has_children = rows.len() > after_sub_rows;
            rows[at].open = rows[at].has_children;
        }
    }
}

/// The Smart Filters rows, then the effects rows, of a layer that shows them.
fn push_sub_rows<T: LayerTree>(tree: &T, id: &T::Id, depth: usize, rows: &mut Vec<RowInfo<LayerRowKey<T::Id>>>) {
    if !tree.effects_open(id) {
        return;
    }
    if let Some(sub) = tree.sub_rows(id, SubRowGroup::SmartFilters) {
        rows.push(leaf(LayerRowKey::Filters(id.clone(), None), depth));
        rows.extend((0..sub.items.len()).map(|i| leaf(LayerRowKey::Filters(id.clone(), Some(i)), depth)));
    }
    if let Some(sub) = tree.sub_rows(id, SubRowGroup::Effects) {
        rows.push(leaf(LayerRowKey::Effects(id.clone(), None), depth));
        rows.extend((0..sub.items.len()).map(|i| leaf(LayerRowKey::Effects(id.clone(), Some(i)), depth)));
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

/// The row the list scrolls to this pass. A new active layer (or the app's `reveal`) reports the
/// `Open` events for its closed ancestors; the row exists in the display order only once the app
/// has applied them, so the scroll waits for the next pass.
fn reveal_target<T: LayerTree>(
    cx: &PanelContext<'_, T>,
    rows: &[RowInfo<LayerRowKey<T::Id>>],
    named: Option<T::Id>,
    events: &mut Vec<LayersEvent<T::Id>>,
) -> Option<LayerRowKey<T::Id>> {
    let tree = cx.tree;
    let active = cx.selection.active.clone();
    let seen = cx.id.with("active-layer");
    let previous: Option<Option<T::Id>> = cx.ctx.data(|d| d.get_temp(seen));
    cx.ctx.data_mut(|d| d.insert_temp(seen, active.clone()));
    let in_rows = |id: &T::Id| rows.iter().any(|row| matches!(&row.key, LayerRowKey::Layer(k) if k == id));

    let waiting = cx.id.with("reveal-waiting");
    let carried: Option<T::Id> = cx.ctx.data(|d| d.get_temp::<Option<T::Id>>(waiting)).flatten();
    let mut reveal = carried.filter(|id| in_rows(id)).map(LayerRowKey::Layer);
    let mut next = None;

    let (target, opens) = match named {
        Some(id) => {
            let opens = closed_ancestor_opens(tree, &id);
            (Some(id), opens)
        }
        None => {
            let opens = reveal_opens(tree, active.as_ref(), previous.as_ref().map(Option::as_ref));
            (active.filter(|_| !opens.is_empty() || previous.as_ref().is_some_and(|p| *p != cx.selection.active)), opens)
        }
    };
    if let Some(id) = target {
        if opens.is_empty() {
            if in_rows(&id) {
                reveal = Some(LayerRowKey::Layer(id));
            }
        } else {
            next = Some(id);
            events.extend(opens.into_iter().map(LayersEvent::Open));
        }
    }
    cx.ctx.data_mut(|d| d.insert_temp(waiting, next));
    reveal
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
    let double_click = ui.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary));
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
                } else if double_click && row.is_layer() && !tree.info(row.layer()).renamable {
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
            ListEvent::Row(RowEvent::Toggle(edit)) => toggle_event(edit, &mut starts, events),
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
                let label = if part == LayerPart::VectorMask { &cx.labels.vector_mask } else { &cx.labels.layer_mask };
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
    match request.toggle {
        Some(0) => {
            let rows = if cx.selection.layers.contains(layer) { cx.selection.layers.clone() } else { vec![layer.clone()] };
            *menu = Some(MenuRequest::Eye { rows, current: cx.tree.info(layer).label, at });
        }
        Some(_) => {}
        None => {
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
}

/// Maps an eye or lock edit. A sweep's `Begin` waits until a value names the row it started on.
fn toggle_event<Id: Clone + PartialEq>(
    edit: &EditEvent<ToggleEdit<LayerRowKey<Id>>>,
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
            starts.push(ToggleStart { id: gesture, start: None, begin: None });
            starts.len() - 1
        });
        if starts[at].start.is_none() {
            starts[at].start = Some((value.row.clone(), column_of(value)));
            if let Some(begin) = starts[at].begin.take() {
                events.push(toggle(&value.row, column_of(value), &begin));
            }
        }
        let (row, column) = starts[at].start.clone().expect("the start is set above");
        match edit {
            EditEvent::Update { modifiers, .. } if modifiers.alt && column_of(value) == LayerColumn::Visibility => {
                events.push(LayersEvent::ShowOnly(value.row.clone()));
            }
            _ => events.push(toggle(&row, column, edit)),
        }
    } else if matches!(edit, EditEvent::Begin { .. }) {
        starts.push(ToggleStart { id: gesture, start: None, begin: Some(edit.clone()) });
    } else if let Some(at) = at {
        let start = starts[at].start.clone();
        if let Some((row, column)) = start {
            events.push(toggle(&row, column, edit));
        }
    }
    if matches!(edit, EditEvent::Commit { .. } | EditEvent::Cancel { .. }) {
        starts.retain(|s| s.id != gesture);
    }
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
