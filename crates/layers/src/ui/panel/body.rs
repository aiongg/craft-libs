//! The rows: the display order over the tree, the `List` wiring, and the mapping of the list's
//! events to the panel's.

use craft_behavior::{ColumnKey, ColumnWidth, DisplayOrder, EditEvent, GestureId, ItemSource, Mixed, Selection, SelectionChange};
use craft_keymap::Mods;
use craft_ui::contract::{
    Bind, CollectionEvent, ColumnKind, ColumnSpec, ListDrag, OpenChange, Props as _, ReorderRule, ShowCollection as _, ToggleColumn, request_rename,
};
use craft_ui::list::{List, RowEvent, ThumbTarget};
use craft_ui::runtime::env::Label;
use egui::Ui;
use std::borrow::Cow;
use std::sync::Arc;

use super::Session;
use super::context::PanelContext;
use super::drag::{LayersDrag, accepts_drop, drop_event};
use super::events::{ActivateOn, LayerColumn, LayerSelect, LayerToggle, LayersEvent, MenuTarget};
use super::menu::OpenMenu;
use super::row::{EYE, FX_PART, LOCK, LOCK_PART, NAME, build_row};
use crate::ui::filter::LayerFilter;
use crate::ui::tree::{LayerInfo, LayerPart, LayerRowKey, LayerTree, SubRowGroup};

/// One displayed row.
pub(super) struct DisplayRow<Id> {
    pub(super) key: LayerRowKey<Id>,
    depth: usize,
    /// The depth the row paints at: a filtered view lists rows at their shown depth and paints them at
    /// the true one.
    pub(super) paint_depth: usize,
    has_children: bool,
    open: bool,
}

/// The rows of the tree as the list shows them: layers, and under each layer whose effects are open
/// the Smart Filter and effect rows.
pub(super) struct LayerRows<'t, T: LayerTree> {
    tree: &'t T,
    pub(super) rows: Vec<DisplayRow<T::Id>>,
    revision: Option<u64>,
}

impl<'t, T: LayerTree> LayerRows<'t, T> {
    /// The rows for `filter`, with the groups in `opened` shown open besides the tree's own.
    pub(super) fn new(tree: &'t T, filter: &LayerFilter, opened: &[T::Id]) -> Self {
        let filtered = filter.active();
        let mut rows = Vec::new();
        push_level(tree, filter, filtered, opened, tree.children(None), Level { shown: 0, true_depth: 0 }, &mut rows);
        let revision = tree.revision().filter(|_| !filtered && opened.is_empty());
        LayerRows { tree, rows, revision }
    }

    pub(super) fn keys(&self) -> Vec<LayerRowKey<T::Id>> {
        self.rows.iter().map(|row| row.key.clone()).collect()
    }
}

impl<T: LayerTree> DisplayOrder<LayerRowKey<T::Id>> for LayerRows<'_, T> {
    fn len(&self) -> usize {
        self.rows.len()
    }

    fn key(&self, at: usize) -> LayerRowKey<T::Id> {
        self.rows[at].key.clone()
    }

    fn depth(&self, at: usize) -> usize {
        self.rows[at].depth
    }

    fn has_children(&self, at: usize) -> bool {
        self.rows[at].has_children
    }

    fn open(&self, at: usize) -> bool {
        self.rows[at].open
    }

    fn text(&self, at: usize) -> Option<Cow<'_, str>> {
        match &self.rows[at].key {
            LayerRowKey::Layer(id) | LayerRowKey::Mask(id, _) => Some(Cow::Owned(self.tree.info(id).name)),
            LayerRowKey::Effects(id, index) => sub_label(self.tree, id, SubRowGroup::Effects, *index),
            LayerRowKey::Filters(id, index) => sub_label(self.tree, id, SubRowGroup::SmartFilters, *index),
        }
    }

    fn revision(&self) -> Option<u64> {
        self.revision
    }
}

impl<T: LayerTree> ItemSource<LayerRowKey<T::Id>> for LayerRows<'_, T> {
    fn toggled(&self, at: usize, column: &ColumnKey) -> bool {
        match &self.rows[at].key {
            LayerRowKey::Layer(id) if *column == EYE => self.tree.info(id).visible,
            LayerRowKey::Layer(id) if *column == LOCK => self.tree.info(id).locks.any(),
            LayerRowKey::Effects(id, index) if *column == EYE => sub_on(self.tree, id, SubRowGroup::Effects, *index),
            LayerRowKey::Filters(id, index) if *column == EYE => sub_on(self.tree, id, SubRowGroup::SmartFilters, *index),
            _ => false,
        }
    }
}

fn sub_label<T: LayerTree>(tree: &T, id: &T::Id, group: SubRowGroup, index: Option<usize>) -> Option<Cow<'static, str>> {
    let rows = tree.sub_rows(id, group)?;
    let row = match index {
        None => rows.header,
        Some(at) => rows.items.into_iter().nth(at)?,
    };
    Some(Cow::Owned(row.label))
}

fn sub_on<T: LayerTree>(tree: &T, id: &T::Id, group: SubRowGroup, index: Option<usize>) -> bool {
    let Some(rows) = tree.sub_rows(id, group) else { return false };
    match index {
        None => rows.header.on,
        Some(at) => rows.items.get(at).is_some_and(|row| row.on),
    }
}

#[derive(Clone, Copy)]
struct Level {
    shown: usize,
    true_depth: usize,
}

fn leaf<Id>(key: LayerRowKey<Id>, level: Level) -> DisplayRow<Id> {
    DisplayRow { key, depth: level.shown, paint_depth: level.true_depth, has_children: false, open: false }
}

fn push_level<T: LayerTree>(
    tree: &T,
    filter: &LayerFilter,
    filtered: bool,
    opened: &[T::Id],
    children: Vec<T::Id>,
    level: Level,
    out: &mut Vec<DisplayRow<T::Id>>,
) {
    for id in children {
        let below = tree.children(Some(&id));
        let shown = !filtered || filter.shows(&tree.info(&id));
        let open = filtered || (!below.is_empty() && (tree.is_open(&id) || opened.contains(&id)));
        let at = out.len();
        if shown {
            let mut row = leaf(LayerRowKey::Layer(id.clone()), level);
            row.has_children = !below.is_empty();
            row.open = row.has_children && open;
            out.push(row);
            push_sub_rows(tree, &id, Level { shown: level.shown + 1, true_depth: level.true_depth + 1 }, out);
        }
        let after_sub_rows = out.len();
        if open {
            let inner = Level { shown: level.shown + usize::from(shown), true_depth: level.true_depth + 1 };
            push_level(tree, filter, filtered, opened, below, inner, out);
        }
        if shown && filtered {
            // A group with no match inside has nothing to open.
            out[at].has_children = out.len() > after_sub_rows;
            out[at].open = out[at].has_children;
        }
    }
}

fn push_sub_rows<T: LayerTree>(tree: &T, id: &T::Id, level: Level, out: &mut Vec<DisplayRow<T::Id>>) {
    if !tree.effects_open(id) {
        return;
    }
    if let Some(sub) = tree.sub_rows(id, SubRowGroup::SmartFilters) {
        out.push(leaf(LayerRowKey::Filters(id.clone(), None), level));
        (0..sub.items.len()).for_each(|i| out.push(leaf(LayerRowKey::Filters(id.clone(), Some(i)), level)));
    }
    if let Some(sub) = tree.sub_rows(id, SubRowGroup::Effects) {
        out.push(leaf(LayerRowKey::Effects(id.clone(), None), level));
        (0..sub.items.len()).for_each(|i| out.push(leaf(LayerRowKey::Effects(id.clone(), Some(i)), level)));
    }
}

/// The closed ancestors of `id`, outermost first, as the opens that reveal it.
fn closed_ancestor_opens<T: LayerTree>(tree: &T, id: &T::Id) -> Vec<OpenChange<T::Id>> {
    let mut opens = Vec::new();
    let mut current = tree.parent(id);
    while let Some(parent) = current {
        if !tree.is_open(&parent) {
            opens.push(OpenChange::new(parent.clone(), true));
        }
        current = tree.parent(&parent);
    }
    opens.reverse();
    opens
}

/// What the pass reveals: the layer to scroll to and the groups to open on the way.
pub(super) struct RevealPlan<Id> {
    pub(super) target: Option<Id>,
    pub(super) opens: Vec<OpenChange<Id>>,
}

impl<Id: Clone> RevealPlan<Id> {
    pub(super) fn opened(&self) -> Vec<Id> {
        self.opens.iter().map(|open| open.key.clone()).collect()
    }
}

/// The reveal of the layer the app named, or of the active layer when it changed since the last pass.
pub(super) fn reveal_plan<T: LayerTree>(tree: &T, active: Option<&T::Id>, named: Option<T::Id>, session: &mut Session<T::Id>) -> RevealPlan<T::Id> {
    let previous = session.seen_active.replace(active.cloned());
    if let Some(id) = named {
        let opens = closed_ancestor_opens(tree, &id);
        return RevealPlan { target: Some(id), opens };
    }
    let changed = previous.as_ref().is_some_and(|previous| previous.as_ref() != active);
    let opens = match (active, changed) {
        (Some(active), true) => closed_ancestor_opens(tree, active),
        _ => Vec::new(),
    };
    RevealPlan { target: active.cloned().filter(|_| changed), opens }
}

/// The tree's selection as the list holds it: the selected layers, the active one as the lead, and the
/// anchor the last pick left while the selection is still what that pick produced.
fn list_selection<T: LayerTree>(cx: &PanelContext<'_, T>, session: &Session<T::Id>) -> Selection<LayerRowKey<T::Id>> {
    let items: Vec<LayerRowKey<T::Id>> = cx.selection.layers.iter().cloned().map(LayerRowKey::Layer).collect();
    let lead = cx.selection.active.clone().map(LayerRowKey::Layer);
    let same = session.produced.len() == cx.selection.layers.len() && session.produced.iter().all(|id| cx.selection.layers.contains(id));
    let anchor = session.anchor.clone().filter(|_| same).or_else(|| lead.clone());
    Selection::new(items, lead, anchor)
}

/// Whether a thumbnail click acts on its own and does not select the row.
pub(super) fn thumb_click_replaces_select(part: LayerPart, mods: Mods, info: &LayerInfo) -> bool {
    let on_mask = part != LayerPart::Pixels;
    if mods.cmd {
        return on_mask || info.thumb_loads_selection;
    }
    if on_mask { mods.shift || mods.alt } else { mods.alt && !mods.shift }
}

fn thumb_part<Id>(key: &LayerRowKey<Id>, info: &LayerInfo, which: ThumbTarget) -> LayerPart {
    match which {
        ThumbTarget::First if matches!(key, LayerRowKey::Filters(_, None)) => LayerPart::FilterMask,
        ThumbTarget::First => LayerPart::Pixels,
        ThumbTarget::Extra(i) => info.masks.get(i).map_or(LayerPart::Pixels, |mask| mask.part),
        _ => LayerPart::Pixels,
    }
}

/// The sweep down an eye or lock column that is running: the gesture it reports under, kept across passes while the
/// pointer is held.
#[derive(Clone)]
pub(super) struct ToggleSweep {
    gesture: GestureId,
    column: LayerColumn,
    show_only: bool,
}

/// What the rows' pass returns.
pub(super) struct BodyOutput<Id> {
    pub(super) events: Vec<LayersEvent<Id>>,
    pub(super) response: egui::Response,
}

/// Shows the list over `rows` and maps what it reports.
pub(super) fn show_body<T: LayerTree>(
    ui: &mut Ui,
    cx: &PanelContext<'_, T>,
    rows: &LayerRows<'_, T>,
    session: &mut Session<T::Id>,
    reveal: &RevealPlan<T::Id>,
    filtered: bool,
    lock_name: &Label,
) -> BodyOutput<T::Id> {
    let tree = cx.tree;
    let order = rows.keys();
    let mut selection = list_selection(cx, session);
    let reveal_key =
        reveal.target.clone().filter(|id| order.iter().any(|key| matches!(key, LayerRowKey::Layer(k) if k == id))).map(LayerRowKey::Layer);

    let mut columns = vec![ColumnSpec::new(EYE, Label::Empty, ColumnWidth::Controls(1.0)).kind(ColumnKind::Toggle(ToggleColumn::sweepable()))];
    if cx.lock_column {
        columns.push(ColumnSpec::new(LOCK, Label::Empty, ColumnWidth::Controls(1.0)).kind(ColumnKind::Toggle(ToggleColumn::sweepable())));
    }
    columns.push(ColumnSpec::new(NAME, Label::Empty, ColumnWidth::fill(1.0)));
    let parent_of = |id: &T::Id| tree.parent(id);
    let ctx = ui.ctx().clone();

    let mut list = List::new(cx.words.list.clone())
        .id(cx.list)
        .tree(true)
        .reorder(ReorderRule::new().into_groups().copy_with_alt())
        .allow_empty(true)
        .rename(true)
        .rename_column(NAME)
        .columns(columns)
        .selection(Bind::bound(&mut selection))
        .can_rename(|key| matches!(key, LayerRowKey::Layer(id) if tree.info(id).renamable))
        .accepts(|drag: &ListDrag<LayerRowKey<T::Id>>, _over, insert| {
            accepts_drop(tree, drag, &order, insert, craft_ui::runtime::pointer::frame(&ctx).mods)
        })
        .drag_payload(|drag: &ListDrag<LayerRowKey<T::Id>>| Some(LayersDrag::of_rows(drag)));
    if let Some(key) = reveal_key {
        list = list.reveal(key);
    }
    let out = list.show_items(ui, rows, &mut |key, item| {
        let row = build_row(cx, key).cells(item.cells()).state(item.row);
        match rows.rows.get(item.at) {
            Some(shown) if shown.has_children => row.depth(shown.paint_depth).group(shown.open),
            Some(shown) => row.depth(shown.paint_depth),
            None => row,
        }
    });

    let mut events: Vec<LayersEvent<T::Id>> = reveal.opens.iter().cloned().map(LayersEvent::Open).collect();
    let selection_changed = out.events.iter().any(|event| matches!(event, CollectionEvent::Selection(_)));
    let list_events = out.events;
    map_events(cx, &order, &parent_of, filtered, lock_name, session, list_events, &mut events);
    if !ui.input(|input| input.pointer.primary_down()) {
        session.sweep = None;
    }
    if selection_changed {
        let mut layers: Vec<T::Id> = Vec::new();
        for key in &selection.items {
            if !layers.contains(key.layer()) {
                layers.push(key.layer().clone());
            }
        }
        session.anchor = selection.anchor.as_ref().map(|key| LayerRowKey::Layer(key.layer().clone()));
        session.produced = layers;
    }
    BodyOutput { events, response: out.response }
}

#[allow(clippy::too_many_arguments)]
fn map_events<T: LayerTree>(
    cx: &PanelContext<'_, T>,
    order: &[LayerRowKey<T::Id>],
    parent_of: &dyn Fn(&T::Id) -> Option<T::Id>,
    filtered: bool,
    lock_name: &Label,
    session: &mut Session<T::Id>,
    list: Vec<CollectionEvent<LayerRowKey<T::Id>, RowEvent>>,
    events: &mut Vec<LayersEvent<T::Id>>,
) {
    let tree = cx.tree;
    let clicks: Vec<(LayerRowKey<T::Id>, ThumbTarget, Mods)> = list
        .iter()
        .filter_map(|event| match event {
            CollectionEvent::Item(key, RowEvent::ThumbClicked { target, mods, .. }) => Some((key.clone(), *target, *mods)),
            _ => None,
        })
        .collect();
    let mut used = vec![false; clicks.len()];
    let activated: Vec<LayerRowKey<T::Id>> = list
        .iter()
        .filter_map(|event| match event {
            CollectionEvent::Activate(key) => Some(key.clone()),
            _ => None,
        })
        .collect();

    let mut toggle_label: Arc<str> = Arc::from("");
    for event in list {
        match event {
            CollectionEvent::Selection(change) => {
                let (row, mapped) = match change {
                    SelectionChange::Set(keys) => (keys.last().cloned(), SelectionChange::Set(layers_of(&keys))),
                    SelectionChange::Toggle(key) => (Some(key.clone()), SelectionChange::Toggle(key.layer().clone())),
                    SelectionChange::Clear => (None, SelectionChange::Clear),
                    _ => continue,
                };
                let mut part = None;
                if let Some(row) = &row
                    && let Some(at) = clicks.iter().enumerate().position(|(i, click)| click.0 == *row && !used[i])
                {
                    used[at] = true;
                    let info = tree.info(row.layer());
                    let hit = thumb_part(row, &info, clicks[at].1);
                    if thumb_click_replaces_select(hit, clicks[at].2, &info) {
                        events.push(LayersEvent::ThumbClick { layer: row.layer().clone(), part: hit, mods: clicks[at].2 });
                        continue;
                    }
                    part = Some(hit);
                }
                events.push(LayersEvent::Select(LayerSelect::new(mapped, part)));
            }
            CollectionEvent::Activate(row) => {
                let on = clicks
                    .iter()
                    .find(|click| click.0 == row)
                    .map_or(ActivateOn::Row, |click| ActivateOn::Thumbnail(thumb_part(&row, &tree.info(row.layer()), click.1)));
                events.push(LayersEvent::Activate { row, on });
            }
            CollectionEvent::Open(change) => {
                if let LayerRowKey::Layer(id) = &change.key {
                    let mut open = OpenChange::new(id.clone(), change.open);
                    if change.all {
                        open = open.all();
                    }
                    events.push(LayersEvent::Open(open));
                }
            }
            CollectionEvent::ContextMenu(request) => {
                let layer = request.key.layer().clone();
                let eye = request.column.as_ref() == Some(&EYE) && matches!(request.key, LayerRowKey::Layer(_));
                let plain = request.column.is_none() || matches!(request.key, LayerRowKey::Effects(..) | LayerRowKey::Filters(..));
                let selected = cx.selection.layers.contains(&layer);
                if eye {
                    let rows = if selected { cx.selection.layers.clone() } else { vec![layer] };
                    let mut labels = rows.iter().map(|id| tree.info(id).label);
                    let first = labels.next().flatten();
                    let current = if labels.all(|label| label == first) { first } else { None };
                    session.menu = Some(OpenMenu::Eye { rows, current });
                } else if plain {
                    if !selected {
                        events.push(LayersEvent::Select(LayerSelect::new(SelectionChange::Set(vec![layer.clone()]), None)));
                        session.produced = vec![layer.clone()];
                        session.anchor = Some(LayerRowKey::Layer(layer));
                    }
                    let on_selection = selected && cx.selection.layers.len() > 1;
                    session.menu = Some(OpenMenu::Row(MenuTarget { row: request.key.clone(), part: None, on_selection }));
                }
            }
            CollectionEvent::Reorder(edit) => events.extend(drop_event(edit, order, parent_of, filtered)),
            CollectionEvent::Toggle(edit) => toggle_events(edit, &mut session.sweep, &mut toggle_label, events),
            CollectionEvent::Rename(edit) => {
                if let EditEvent::Commit { after, .. } | EditEvent::Set { after, .. } = edit
                    && let LayerRowKey::Layer(id) = &after.key
                {
                    let text = after.name.trim();
                    if !text.is_empty() && text != tree.info(id).name {
                        events.push(LayersEvent::Rename { layer: id.clone(), text: text.to_owned() });
                    }
                }
            }
            CollectionEvent::Item(LayerRowKey::Layer(id), RowEvent::Open { open, all, .. }) => {
                let change = OpenChange::new(id, open);
                events.push(LayersEvent::Open(if all { change.all() } else { change }));
            }
            CollectionEvent::Item(row, RowEvent::LinkToggled { thumb, linked, .. }) => {
                if let Some(mask) = tree.info(row.layer()).masks.get(thumb) {
                    events.push(LayersEvent::MaskLink { layer: row.layer().clone(), part: mask.part, linked });
                }
            }
            CollectionEvent::Item(row, RowEvent::Trailing { part, mods, .. }) => {
                let layer = row.layer().clone();
                match part {
                    part if part == craft_ui::list::TrailingPart::new(LOCK_PART) => {
                        let unlock = EditEvent::set(GestureId::default(), Mixed::Same(true), false, Arc::from(lock_name.as_str()), None);
                        events.push(LayersEvent::Toggle(LayerToggle::new(vec![row], LayerColumn::Lock, unlock)));
                    }
                    part if part == craft_ui::list::TrailingPart::new(FX_PART) => {
                        events.push(LayersEvent::FxOpen { open: !tree.effects_open(&layer), layer, all: mods.alt });
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    // A thumbnail click that selected nothing new still acts.
    for (i, (row, target, mods)) in clicks.iter().enumerate() {
        if used[i] || activated.contains(row) {
            continue;
        }
        let info = tree.info(row.layer());
        let part = thumb_part(row, &info, *target);
        events.push(if thumb_click_replaces_select(part, *mods, &info) {
            LayersEvent::ThumbClick { layer: row.layer().clone(), part, mods: *mods }
        } else {
            LayersEvent::Select(LayerSelect::new(SelectionChange::Set(vec![row.layer().clone()]), Some(part)))
        });
    }
}

fn layers_of<Id: Clone + PartialEq>(keys: &[LayerRowKey<Id>]) -> Vec<Id> {
    let mut layers: Vec<Id> = Vec::new();
    for key in keys {
        if !layers.contains(key.layer()) {
            layers.push(key.layer().clone());
        }
    }
    layers
}

fn column_of(column: &ColumnKey) -> LayerColumn {
    if *column == EYE { LayerColumn::Visibility } else { LayerColumn::Lock }
}

/// The cells of an eye or lock sweep as events. The list reports each cell a sweep crosses as a gesture of its own,
/// opened by a `Begin` (which names the gesture in `label`), given its value by an `Update` and ended by a `Commit` on
/// release or a `Cancel` on Escape. The cells of one sweep share the gesture of its first, and the cells of one pass
/// share one event, in the order crossed; an Escape reports each cell with the state it had before. Alt on the first
/// press of an eye sweep shows only that layer and drops the rest of the sweep.
fn toggle_events<Id: Clone + PartialEq>(
    edit: EditEvent<craft_ui::contract::ToggleEdit<LayerRowKey<Id>>>,
    sweep: &mut Option<ToggleSweep>,
    label: &mut Arc<str>,
    events: &mut Vec<LayersEvent<Id>>,
) {
    match edit {
        EditEvent::Begin { label: named, .. } => *label = named,
        EditEvent::Update { id, value, .. } => {
            let column = column_of(&value.column);
            if sweep.as_ref().is_none_or(|running| running.column != column) {
                let show_only = value.mods.alt && column == LayerColumn::Visibility;
                *sweep = Some(ToggleSweep { gesture: id, column, show_only });
                if show_only {
                    events.push(LayersEvent::ShowOnly(value.key));
                    return;
                }
            }
            let Some(running) = sweep.as_ref().filter(|running| !running.show_only) else { return };
            let edit = EditEvent::set(running.gesture, Mixed::Same(!value.on), value.on, label.clone(), None);
            push_cell(events, running.gesture, value.key, column, edit);
        }
        EditEvent::Cancel { reason, before: Some(Mixed::Same(before)), .. } => {
            let column = column_of(&before.column);
            let Some(running) = sweep.as_ref().filter(|running| !running.show_only && running.column == column) else { return };
            let edit = EditEvent::cancel(running.gesture, reason, Some(Mixed::Same(before.on)));
            push_cell(events, running.gesture, before.key, column, edit);
        }
        _ => {}
    }
}

/// Adds `key` to the last toggle event when it belongs to the gesture `gesture` and has the same kind, and starts a
/// new event otherwise.
fn push_cell<Id: Clone + PartialEq>(
    events: &mut Vec<LayersEvent<Id>>,
    gesture: GestureId,
    key: LayerRowKey<Id>,
    column: LayerColumn,
    edit: EditEvent<bool>,
) {
    if let Some(LayersEvent::Toggle(last)) = events.last_mut()
        && last.edit.id() == gesture
        && std::mem::discriminant(&last.edit) == std::mem::discriminant(&edit)
    {
        last.rows.push(key);
        return;
    }
    events.push(LayersEvent::Toggle(LayerToggle::new(vec![key], column, edit)));
}

/// Opens the rename field of `layer`'s row in the list `list`.
pub(super) fn start_rename<K: Clone + Eq + std::hash::Hash + Send + Sync + 'static>(ui: &Ui, list: egui::Id, layer: K) {
    request_rename(ui.ctx(), list, &LayerRowKey::Layer(layer));
}
