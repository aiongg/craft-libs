//! The rows: the display order over the tree, the `List` wiring, and the mapping of the list's
//! events to the panel's.

use craft_behavior::{ColumnKey, ColumnWidth, DisplayOrder, EditEvent, GestureId, IconId, ItemSource, Mixed, Selection, SelectionChange};
use craft_keymap::Mods;
use craft_ui::contract::{
    Bind, CollectionEvent, ColumnKind, ColumnSpec, ListDrag, OpenChange, Props as _, ReorderRule, ShowCollection as _, SweepApply, SweepStep,
    ToggleColumn, request_rename,
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

/// The readers of the eye and lock sweeps, kept across passes while the pointer is held.
#[derive(Clone)]
pub(super) struct ToggleSweeps {
    eye: SweepApply,
    lock: SweepApply,
}

impl Default for ToggleSweeps {
    /// The eye shows only the pressed layer on Alt; the lock column ignores Alt.
    fn default() -> Self {
        ToggleSweeps { eye: SweepApply::show_only(), lock: SweepApply::new() }
    }
}

impl ToggleSweeps {
    /// The sweep reader of `column`.
    fn of(&mut self, column: LayerColumn) -> &mut SweepApply {
        if column == LayerColumn::Visibility { &mut self.eye } else { &mut self.lock }
    }
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

    let mut columns = vec![
        ColumnSpec::new(EYE, Label::Empty, ColumnWidth::Controls(1.0)).kind(ColumnKind::Toggle(ToggleColumn::sweepable())).icon(IconId::new("eye")),
    ];
    if cx.lock_column {
        columns.push(
            ColumnSpec::new(LOCK, Label::Empty, ColumnWidth::Controls(1.0))
                .kind(ColumnKind::Toggle(ToggleColumn::sweepable()))
                .icon(IconId::new("lock")),
        );
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
            CollectionEvent::Toggle(edit) => toggle_events(edit, &mut session.sweeps, &mut toggle_label, events),
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
                        let unlock = EditEvent::set(Mixed::Same(true), false, Arc::from(lock_name.as_str()), None);
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

/// The cells of an eye or lock sweep as events. The cells of one sweep are `Update`s of the gesture of its first, opened by a
/// `Begin` and closed by one `Commit` on release; the cells of one pass share one event, in the order crossed. An Escape reports the cells
/// as one `Cancel` of that gesture with the state each had before. A plain press is a `Set`. Alt on the first press of
/// an eye sweep shows only that layer and drops the rest of the sweep.
fn toggle_events<Id: Clone + PartialEq>(
    edit: EditEvent<craft_ui::contract::ToggleEdit<LayerRowKey<Id>>>,
    sweeps: &mut ToggleSweeps,
    label: &mut Arc<str>,
    events: &mut Vec<LayersEvent<Id>>,
) {
    if let EditEvent::Begin { label: named, .. } = &edit {
        *label = named.clone();
    }
    let column = match &edit {
        EditEvent::Update { value: cell, .. } | EditEvent::Set { after: cell, .. } | EditEvent::Commit { after: cell, .. } => column_of(&cell.column),
        EditEvent::Begin { before: Mixed::Same(cell), .. } | EditEvent::Cancel { before: Some(Mixed::Same(cell)), .. } => column_of(&cell.column),
        _ => return,
    };
    // The sweep ends with its first Commit or Cancel of a pass; the cells after it still belong to its gesture.
    let running = sweeps.of(column).gesture();
    let ending = matches!(edit, EditEvent::Commit { .. } | EditEvent::Cancel { .. });
    let Some(step) = sweeps.of(column).feed(&edit) else {
        // The first cell of a sweep opens its gesture; the cells after it join that one.
        if let EditEvent::Begin { before: Mixed::Same(before), .. } = &edit
            && running.is_none()
            && let Some(gesture) = sweeps.of(column).gesture()
        {
            let begin = EditEvent::begin(gesture, Mixed::Same(before.on), label.clone(), None);
            push_cell(events, Some(gesture), before.key.clone(), column, begin);
        }
        if let EditEvent::Commit { before: Mixed::Same(before), after, .. } = &edit
            && let Some(gesture) = running
        {
            let commit = EditEvent::commit(gesture, Mixed::Same(before.on), after.on);
            push_cell(events, Some(gesture), after.key.clone(), column, commit);
        }
        return;
    };
    let gesture = if ending { running.or_else(|| closing_gesture(events, column)) } else { sweeps.of(column).gesture() };
    match step {
        // Only the eye column shows one layer; the lock column is plain and never gives `ShowOnly`.
        SweepStep::ShowOnly(cell) => events.push(LayersEvent::ShowOnly(cell.key)),
        SweepStep::Flip(cell) => {
            let edit = match gesture {
                Some(id) => EditEvent::update(id, cell.on),
                None => EditEvent::set(Mixed::Same(!cell.on), cell.on, label.clone(), None),
            };
            push_cell(events, gesture, cell.key, column, edit);
        }
        SweepStep::Restore(cell) => {
            let EditEvent::Cancel { reason, .. } = edit else { return };
            let Some(gesture) = gesture else { return };
            let edit = EditEvent::cancel(gesture, reason, Some(Mixed::Same(cell.on)));
            push_cell(events, Some(gesture), cell.key, column, edit);
        }
        // The rows an Alt sweep turned off have no event to take them back.
        SweepStep::UndoShowOnly(_) => {}
        _ => {}
    }
}

/// The gesture of the sweep of `column` that this pass already closed, for the cells of its release or cancel that come
/// after the first.
fn closing_gesture<Id>(events: &[LayersEvent<Id>], column: LayerColumn) -> Option<GestureId> {
    events.iter().rev().find_map(|event| match event {
        LayersEvent::Toggle(toggle) if toggle.column == column => toggle.edit.id(),
        _ => None,
    })
}

/// Adds `key` to the last toggle event when it is the same kind of the same gesture, and starts a new event otherwise.
/// Events with no gesture (a plain press) never merge.
fn push_cell<Id: Clone + PartialEq>(
    events: &mut Vec<LayersEvent<Id>>,
    gesture: Option<GestureId>,
    key: LayerRowKey<Id>,
    column: LayerColumn,
    edit: EditEvent<bool>,
) {
    if let Some(LayersEvent::Toggle(last)) = events.last_mut()
        && gesture.is_some()
        && last.column == column
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
