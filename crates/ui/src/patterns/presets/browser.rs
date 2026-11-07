//! `PresetBrowser`: a library of named presets shown as thumbnails, with groups, a grid or a list, a thumbnail size,
//! search, rename and drag to reorder.
//!
//! The host keeps the library and performs every command; the browser shows the rows it is given, changes only its
//! `PresetState` and reports `PresetEvent`s. The grid view is a `TileGrid` of `Tile`s under group headers, the list
//! view a tree `List` of `ListRow`s; both read the same rows and the same state.

mod reorder;
#[cfg(feature = "testing")]
mod stories;

pub use reorder::PresetReorder;

use super::event::{PresetEvent, PresetOutput};
use super::footer::PresetFooter;
use super::model::{CellShape, PresetRow, ThumbInfo, children_end, has_captions};
use super::state::{DefaultOpen, PresetState, PresetView};
use super::words::PresetWords;
use crate::bars::{Bar, BarKind};
use crate::choice::Segmented;
use crate::contract::{
    Bind, BodyCx, CollectionEvent, Entry, LeadSource, ListDrag, OpenChange, Pattern, Props, Rename, ReorderMove, ReorderRule, SelectMode, Show as _,
    ShowCollection as _, ShowContainer as _, request_rename,
};
use crate::display::Divider;
use crate::layout_components::Stack;
use crate::list::{List, ListRow, RowInk, RowTest, ThumbSize};
use crate::menu::{Menu, MenuItem};
use crate::notices::{Notice, NoticeLayout};
use crate::primitives::ContentPainterCx;
use crate::runtime::env::{Env, Label};
use crate::runtime::memory::StageAccess;
use crate::text::TextField;
use crate::tile_grid::{CaptionPlace, GridEvent, Tile, TileGrid, TileGroup, TileZoom};
use craft_behavior::{DisplayOrder, EditEvent, GestureId, GesturePhase, IconId, ItemSource, Mixed, Selection, SelectionChange};
use craft_i18n::matcher::Matcher;
use craft_theme::variant::Tone;
use egui::{Rect, Ui};
use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// The commands of a preset's context menu. They are app commands: the browser reports a run row as the event of the
/// same name, and a host that binds a key to one handles it as it handles the event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, craft_ui_macros::CommandSet)]
#[command(prefix = "presetItem")]
#[non_exhaustive]
pub enum PresetItemCommands {
    /// Delete the preset.
    #[command(label = "Delete")]
    Delete,
    /// Delete the group and the presets in it.
    #[command(label = "Delete Group")]
    DeleteGroup,
    /// Rename the preset.
    #[command(label = "Rename…")]
    Rename,
    /// Rename the group.
    #[command(label = "Rename Group…")]
    RenameGroup,
}

/// The zoom from which list rows show their large thumbnail, and the zoom up to which they show the small one.
const LARGE_FROM: f64 = 1.5;
const SMALL_UNTIL: f64 = 0.75;

/// Paints a preset's thumbnail: the content painter, the rect and what the browser knows of the preset.
pub struct ThumbPainter<'a, K>(Box<dyn Fn(&ContentPainterCx, Rect, ThumbInfo<'_, K>) + 'a>);

impl<'a, K> ThumbPainter<'a, K> {
    /// The painter that runs `paint`.
    pub fn new(paint: impl Fn(&ContentPainterCx, Rect, ThumbInfo<'_, K>) + 'a) -> Self {
        ThumbPainter(Box::new(paint))
    }
}

/// Adds the host's buttons to the footer the browser makes.
pub struct FooterSetup<'a, K>(Box<dyn for<'f> Fn(PresetFooter<'f, K>) -> PresetFooter<'f, K> + 'a>);

impl<'a, K> FooterSetup<'a, K> {
    /// The setup that runs `setup`.
    pub fn new(setup: impl for<'f> Fn(PresetFooter<'f, K>) -> PresetFooter<'f, K> + 'a) -> Self {
        FooterSetup(Box::new(setup))
    }
}

/// A library of named presets shown as thumbnails.
///
/// - **Rows.** The host's flat display list: groups with their depth, then presets. The grid view shows each group as
///   a header over its presets; the list view shows the tree.
/// - **State.** `state` is the view, the thumbnail zoom, the open groups and the search text. Bind it with
///   `Bind::bound`: the browser writes a changed state through the binding.
/// - **Keys and pointer.** A click selects, a double click or Enter applies, F2 renames, Left and Right close and open
///   a group, and a drag reorders by the `reorder` rules. The host performs the delete, rename and move it is told of.
/// - **Search.** The toolbar's field keeps the presets whose name matches, with their groups, all open; Up, Down and
///   Enter go from the field to the presets.
/// - **Row menu.** Rename, Move to, Delete and the host's `extra_items` act on the row the menu was asked for in this pass, else on
///   `selected`.
///
/// ```ignore
/// let out = PresetBrowser::new("Swatches", Bind::bound(&mut state), &rows).selected(chosen.clone()).toolbar(true).show(ui);
/// for event in out.events { /* the host applies Select, Activate, Reorder, ... */ }
/// ```
#[must_use = "show the browser with `show`"]
#[derive(Pattern, Props)]
#[pattern(probe(PresetBrowser<'static, u32>))]
pub struct PresetBrowser<'a, K: Clone + Eq + Hash + Send + Sync + 'static> {
    label: Label,
    state: Bind<'a, PresetState<K>>,
    rows: &'a [PresetRow<K>],
    /// The selected preset, which the host keeps.
    #[prop]
    selected: Option<K>,
    /// The shape of a grid tile's picture.
    #[prop]
    cell_shape: CellShape,
    /// The zoom the thumbnails span and how it steps; the footer's slider uses the same.
    #[prop]
    thumb_range: TileZoom,
    /// Which groups are open before the user toggles any.
    #[prop]
    default_open: DefaultOpen,
    /// A row above the presets with the search field and the grid and list buttons.
    #[prop]
    toolbar: bool,
    /// Whether a secondary click or the context-menu key opens the row menu: Rename, Move to, Delete and the host's
    /// `extra_items`.
    #[prop]
    row_menu: bool,
    /// Fixes the view, whatever the state says; the toolbar then has no view buttons.
    #[prop]
    view_lock: Option<PresetView>,
    /// Which presets and groups may be renamed; F2 does nothing on another.
    #[prop(func = "Fn(&K) -> bool")]
    can_rename: Option<RowTest<'a, K>>,
    /// Which presets and groups the context menu may delete; its row is disabled for the others.
    #[prop(func = "Fn(&K) -> bool")]
    can_delete: Option<RowTest<'a, K>>,
    /// Which presets the row menu's Move to may move; the menu has no such row for the others.
    #[prop(func = "Fn(&K) -> bool")]
    can_move: Option<RowTest<'a, K>>,
    /// Paints a preset's thumbnail; without it the row's own `preview` paints.
    #[prop(func = "Fn(&ContentPainterCx, Rect, ThumbInfo<'_, K>)")]
    thumb: Option<ThumbPainter<'a, K>>,
    /// Presets (and groups) drag and move by these rules.
    #[prop]
    reorder: Option<PresetReorder<'a, K>>,
    extra_items: Vec<Entry<String>>,
    /// A footer under the presets. The closure receives it with the browser's zoom bound to its slider and the count
    /// of presets, and adds the host's buttons and options.
    #[prop(func = "for<'f> Fn(PresetFooter<'f, K>) -> PresetFooter<'f, K>")]
    footer: Option<FooterSetup<'a, K>>,
}

impl<'a, K: Clone + Eq + Hash + Send + Sync + 'static> PresetBrowser<'a, K> {
    /// A browser named `label` of `rows`, keeping its view settings in `state`: a grid of square thumbnails, no
    /// toolbar, with a context menu.
    pub fn new(label: impl Into<Label>, state: Bind<'a, PresetState<K>>, rows: &'a [PresetRow<K>]) -> Self {
        PresetBrowser {
            label: label.into(),
            state,
            rows,
            selected: None,
            cell_shape: CellShape::Square,
            thumb_range: TileZoom::Ladder,
            default_open: DefaultOpen::All,
            toolbar: false,
            row_menu: true,
            view_lock: None,
            can_rename: None,
            can_delete: None,
            can_move: None,
            thumb: None,
            reorder: None,
            extra_items: Vec::new(),
            footer: None,
        }
    }

    /// Rows added to the context menu after the standard ones; running one reports `PresetEvent::Menu` with the
    /// entry's value.
    #[must_use]
    pub fn extra_items(mut self, items: impl IntoIterator<Item = Entry<String>>) -> Self {
        self.extra_items = items.into_iter().collect();
        self
    }
}

/// The rows the browser shows, as a display order over `rows`.
struct Shown<'a, K> {
    rows: &'a [PresetRow<K>],
    at: Vec<usize>,
    open: Vec<bool>,
    tree: bool,
}

impl<K: Clone> DisplayOrder<K> for Shown<'_, K> {
    fn len(&self) -> usize {
        self.at.len()
    }

    fn key(&self, at: usize) -> K {
        self.rows[self.at[at]].key.clone()
    }

    fn depth(&self, at: usize) -> usize {
        if self.tree { self.rows[self.at[at]].depth } else { 0 }
    }

    fn has_children(&self, at: usize) -> bool {
        self.tree && self.rows[self.at[at]].is_group()
    }

    fn open(&self, at: usize) -> bool {
        self.open[at]
    }

    fn text(&self, at: usize) -> Option<std::borrow::Cow<'_, str>> {
        Some(std::borrow::Cow::Borrowed(self.rows[self.at[at]].label.as_str()))
    }
}

impl<K: Clone> ItemSource<K> for Shown<'_, K> {}

/// The positions of the rows to show: without a search, the rows outside closed groups; with one, the presets whose
/// name matches and their groups.
fn shown_rows<K: Clone + Eq + Hash>(
    rows: &[PresetRow<K>],
    state: &PresetState<K>,
    default_open: DefaultOpen,
    matcher: Option<&Matcher>,
) -> Vec<usize> {
    if let Some(matcher) = matcher {
        let mut keep = vec![false; rows.len()];
        let mut path: Vec<usize> = Vec::new();
        for (at, row) in rows.iter().enumerate() {
            path.truncate(row.depth);
            if !row.is_group() && matcher.score(&row.label).is_some() {
                keep[at] = true;
                for &ancestor in &path {
                    keep[ancestor] = true;
                }
            }
            path.push(at);
        }
        return (0..rows.len()).filter(|&at| keep[at]).collect();
    }
    let open = state.open_test(rows, default_open);
    let mut out = Vec::with_capacity(rows.len());
    let mut at = 0;
    while at < rows.len() {
        out.push(at);
        at = if rows[at].is_group() && !open(at) { children_end(rows, at) } else { at + 1 };
    }
    out
}

/// The grid's tiles and group headers from the shown rows: each header covers the presets that follow it inside its
/// group.
struct Plan {
    tiles: Vec<usize>,
    groups: Vec<TileGroup>,
    group_rows: Vec<usize>,
}

fn grid_plan<K>(rows: &[PresetRow<K>], shown: &[usize], open: impl Fn(usize) -> bool, count: impl Fn(usize) -> Option<String>) -> Plan {
    let mut plan = Plan { tiles: Vec::new(), groups: Vec::new(), group_rows: Vec::new() };
    let mut counting = false;
    for &at in shown {
        let row = &rows[at];
        if row.is_group() {
            let mut group = TileGroup::new(plan.tiles.len(), 0, row.label.clone()).open(open(at));
            if let Some(detail) = count(at) {
                group = group.detail(detail);
            }
            plan.groups.push(group);
            plan.group_rows.push(at);
            counting = true;
            continue;
        }
        plan.tiles.push(at);
        if counting
            && let (Some(group), Some(&head)) = (plan.groups.last_mut(), plan.group_rows.last())
            && row.depth > rows[head].depth
        {
            group.count += 1;
        } else {
            counting = false;
        }
    }
    plan
}

/// The position in the full row list that a reorder of the shown rows means: `mv.to` counts the rows that remain
/// shown once `mv.keys` are out; the result counts all rows that remain once the moved rows and the rows inside
/// moved groups are out, as the library's reorder takes it.
fn full_position<K: Eq>(rows: &[PresetRow<K>], shown: &[usize], mv: &ReorderMove<K>) -> usize {
    let mut gone = vec![false; rows.len()];
    for key in &mv.keys {
        if let Some(at) = rows.iter().position(|row| row.key == *key) {
            gone[at..children_end(rows, at)].fill(true);
        }
    }
    let anchor = shown.iter().copied().filter(|&at| !mv.keys.contains(&rows[at].key)).nth(mv.to);
    match anchor {
        Some(anchor) => (0..anchor).filter(|&at| !gone[at]).count(),
        None => gone.iter().filter(|gone| !**gone).count(),
    }
}

/// The rows stamp of a query: another query starts the lead over.
fn query_stamp(query: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    query.hash(&mut hasher);
    hasher.finish()
}

fn list_thumb(zoom: f64) -> ThumbSize {
    if zoom >= LARGE_FROM {
        ThumbSize::Large
    } else if zoom < SMALL_UNTIL {
        ThumbSize::Small
    } else {
        ThumbSize::Medium
    }
}

fn card_aspect(shape: CellShape) -> f32 {
    match shape {
        CellShape::Square => 1.0,
        CellShape::Ratio { wide, high } => f32::from(wide) / f32::from(high.max(1)),
    }
}

/// What a pass of either view reported that the browser acts on.
struct Pass<K> {
    seen: Vec<Seen<K>>,
    grid: Vec<GridEvent<K>>,
}

enum Seen<K> {
    Select(K),
    Activate(K),
    Open(OpenChange<K>),
    Menu(K),
    Pointer(K),
    Reorder(EditEvent<ReorderMove<K>>),
    Rename(EditEvent<Rename<K>>),
}

/// The events of a collection that the browser acts on, whatever its items report.
fn seen<K, E>(events: Vec<CollectionEvent<K, E>>) -> Vec<Seen<K>> {
    events
        .into_iter()
        .filter_map(|event| match event {
            CollectionEvent::Selection(SelectionChange::Set(keys)) => keys.into_iter().last().map(Seen::Select),
            CollectionEvent::Activate(key) => Some(Seen::Activate(key)),
            CollectionEvent::Open(change) => Some(Seen::Open(change)),
            CollectionEvent::ContextMenu(request) => Some(Seen::Menu(request.key)),
            CollectionEvent::LeadMoved { key, by: LeadSource::Pointer } => Some(Seen::Pointer(key)),
            CollectionEvent::Reorder(edit) => Some(Seen::Reorder(edit)),
            CollectionEvent::Rename(edit) => Some(Seen::Rename(edit)),
            _ => None,
        })
        .collect()
}

/// The browser's view settings across the passes of its body and footer: the state as read, the state as the
/// pass changes it, and the events in order. Its binding is written once, by the last region that runs.
struct Session<'a, K> {
    state: Bind<'a, PresetState<K>>,
    before: PresetState<K>,
    local: PresetState<K>,
    events: Vec<PresetEvent<K>>,
}

impl<K: Clone + Eq + Hash + Send + Sync + 'static> Session<'_, K> {
    /// Starts a pass: reads the state and clears the events of an earlier run of the body.
    fn begin(&mut self, cx: &mut impl StageAccess, thumb_range: &TileZoom) {
        self.before = match self.state.reborrow().read(cx) {
            Some(Mixed::Same(held)) => held,
            _ => PresetState::new(),
        };
        self.local = self.before.clone();
        self.local.zoom = self.local.zoom_in(thumb_range);
        self.events.clear();
    }

    /// Writes the state the pass changed through the binding.
    fn commit(&mut self, cx: &mut impl StageAccess, rows: &[PresetRow<K>], name: &str) {
        self.local.retain_groups(rows);
        if self.local != self.before {
            let event = EditEvent::set(GestureId::default(), Mixed::Same(self.before.clone()), self.local.clone(), Arc::from(name), None);
            let _ = self.state.reborrow().write(cx, &event);
        }
    }
}

impl<K: Clone + Eq + Hash + Send + Sync + 'static> PresetBrowser<'_, K> {
    /// Shows the browser across the available width and reports what the user did, in order.
    pub fn show(self, ui: &mut Ui) -> PresetOutput<K> {
        let PresetBrowser {
            label,
            state,
            rows,
            selected,
            cell_shape,
            thumb_range,
            default_open,
            toolbar,
            row_menu,
            view_lock,
            can_rename,
            can_delete,
            can_move,
            thumb,
            reorder,
            extra_items,
            footer,
        } = self;
        let env = Env::of(ui);
        let words = env.words::<PresetWords>();
        let list_id = ui.scope_id().with("browser-rows");
        let name = label.as_str().to_owned();
        let has_footer = footer.is_some();
        let session = RefCell::new(Session { state, before: PresetState::new(), local: PresetState::new(), events: Vec::new() });

        let body = |ui: &mut Ui, cx: &mut BodyCx<'_>| {
            session.borrow_mut().begin(cx, &thumb_range);
            let mut local = session.borrow().local.clone();
            let mut events: Vec<PresetEvent<K>> = Vec::new();

            if toolbar {
                let modes = [
                    Entry::new(PresetView::Grid, words.grid_view.clone()).icon(IconId::new("layout-grid")),
                    Entry::new(PresetView::List, words.list_view.clone()).icon(IconId::new("list")),
                ];
                let locked = view_lock.is_some();
                let _ = Bar::new(Label::Empty, BarKind::Tool).show(ui, |ui, _| {
                    let field = TextField::search(words.search.clone(), Bind::bound(&mut local.filter)).placeholder(words.search.clone());
                    let field = field.drives(list_id);
                    field.show(ui);
                    if !locked {
                        Segmented::new(words.view.clone(), Bind::bound(&mut local.view), &modes).show(ui);
                    }
                });
            }
            let view = view_lock.unwrap_or(local.view);

            let query = local.filter.trim().to_owned();
            let matcher = (!query.is_empty()).then(|| Matcher::new(&env.folder(), &query));
            let shown = shown_rows(rows, &local, default_open, matcher.as_ref());
            let open_flags: Vec<bool> = {
                let test = local.open_test(rows, default_open);
                (0..rows.len()).map(test).collect()
            };
            let counted = |at: usize| rows[at].count.map(|n| words.group_count.text(n as u64, &env.format().integer(n as u64)));
            let selection = selected.clone().map_or_else(|| Selection::new(Vec::new(), None, None), Selection::single);
            let stamp = query_stamp(&query);
            let zoom = local.zoom;

            let can_drag = |key: &K| {
                rows.iter()
                    .find(|row| row.key == *key)
                    .is_some_and(|row| !row.is_group() || reorder.as_ref().is_some_and(PresetReorder::moves_groups))
            };
            let accepts = |drag: &ListDrag<K>, over: &K, insert: usize| reorder.as_ref().is_some_and(|rules| rules.allows(rows, drag, over, insert));
            let rename_ok = |key: &K| can_rename.as_ref().is_none_or(|test| test.passes(key));
            let painter = thumb.as_ref();
            let paint = |row: &PresetRow<K>, selected: bool| {
                let key = row.key.clone();
                let preview = row.preview.clone();
                move |cx: &ContentPainterCx, rect: Rect, painter: Option<&ThumbPainter<'_, K>>| match (painter, &preview) {
                    (Some(painter), _) => (painter.0)(cx, rect, ThumbInfo::new(&key, selected, false)),
                    (None, Some(preview)) => (preview.paint)(cx, rect),
                    (None, None) => {}
                }
            };

            let mut menu_target: Option<K> = None;
            let mut hovered: Option<K> = None;
            let (response, displayed, pass) = if shown.is_empty() {
                let message = if query.is_empty() { words.empty.clone() } else { words.no_match.clone() };
                let response = Notice::new(Tone::Neutral, message).layout(NoticeLayout::Centred).show(ui).response;
                (response, Vec::new(), Pass { seen: Vec::new(), grid: Vec::new() })
            } else if view == PresetView::Grid {
                let plan = grid_plan(rows, &shown, |at| open_flags[at], counted);
                let tiles = Shown { rows, at: plan.tiles.clone(), open: vec![false; plan.tiles.len()], tree: false };
                let captions = if has_captions(rows) { CaptionPlace::Below } else { CaptionPlace::None };
                let mut grid = TileGrid::<K>::new(label.clone())
                    .id(list_id)
                    .select(SelectMode::Single)
                    .selection(Bind::given(selection))
                    .caption(captions)
                    .aspect(card_aspect(cell_shape))
                    .zoom(zoom as f32)
                    .groups(&plan.groups)
                    .collapsible()
                    .rename(true)
                    .rows_stamp(stamp)
                    .can_rename(&rename_ok)
                    .can_drag(&can_drag)
                    .accepts(&accepts);
                if reorder.is_some() {
                    grid = grid.reorder(ReorderRule::new());
                }
                let aspect = cell_shape.aspect();
                let out = grid.show_items(ui, &tiles, &mut |_, icx| {
                    let row = &rows[tiles.at[icx.at]];
                    let mut tile = Tile::new(row.label.clone());
                    if let Some(detail) = &row.caption {
                        tile = tile.detail(detail.clone());
                    }
                    if row.dimmed {
                        tile = tile.dimmed();
                    }
                    if let Some(tip) = &row.tooltip {
                        tile = tile.tooltip(tip.clone());
                    }
                    if let Some(name) = &row.accessible_name {
                        tile = tile.accessible_name(name.clone());
                    }
                    let selected = icx.row.selected;
                    let draw = paint(row, selected);
                    tile.thumb(row.preview.as_ref().map_or(aspect, |preview| preview.aspect), move |cx, rect| draw(cx, rect, painter))
                });
                (out.response.response.clone(), plan.tiles.clone(), Pass { seen: seen(out.response.events), grid: out.events })
            } else {
                let open: Vec<bool> = shown.iter().map(|&at| open_flags[at]).collect();
                let source = Shown { rows, at: shown.clone(), open, tree: true };
                let thumb_size = list_thumb(zoom);
                let mut list = List::<K>::new(label.clone())
                    .id(list_id)
                    .tree(true)
                    .select(SelectMode::Single)
                    .selection(Bind::given(selection))
                    .rename(true)
                    .rows_stamp(stamp)
                    .can_rename(&rename_ok)
                    .can_drag(&can_drag)
                    .accepts(&accepts);
                if reorder.is_some() {
                    list = list.reorder(ReorderRule::new().into_groups());
                }
                let aspect = cell_shape.aspect();
                let out = list.show_items(ui, &source, &mut |_, icx| {
                    let row = &rows[source.at[icx.at]];
                    if row.is_group() {
                        let mut line = ListRow::new(row.label.clone()).heading().depth(row.depth).group(icx.row.open);
                        if let Some(detail) = counted(source.at[icx.at]) {
                            line = line.detail(detail);
                        }
                        return line;
                    }
                    let mut line = ListRow::new(row.label.clone()).depth(row.depth);
                    if let Some(caption) = &row.caption {
                        line = line.detail(caption.clone());
                    }
                    if row.dimmed {
                        line = line.ink(RowInk::Dimmed);
                    }
                    if let Some(tip) = &row.tooltip {
                        line = line.tooltip(tip.clone());
                    }
                    if let Some(name) = &row.accessible_name {
                        line = line.accessible_name(name.clone());
                    }
                    let draw = paint(row, icx.row.selected);
                    line.thumb_aspect(thumb_size, row.preview.as_ref().map_or(aspect, |preview| preview.aspect), move |cx, rect| {
                        draw(cx, rect, painter)
                    })
                });
                (out.response.clone(), shown.clone(), Pass { seen: seen(out.events), grid: Vec::new() })
            };

            let mods = crate::runtime::pointer::frame(ui.ctx()).mods;
            let row_of = |key: &K| rows.iter().position(|row| row.key == *key);
            let toggle = |local: &mut PresetState<K>, events: &mut Vec<PresetEvent<K>>, at: usize, open: Option<bool>| {
                let open = open.unwrap_or_else(|| !local.is_open(rows, at, default_open));
                let change = OpenChange::new(rows[at].key.clone(), open);
                local.apply_open(rows, &change, default_open);
                events.push(PresetEvent::Open(change));
            };
            for event in pass.grid {
                match event {
                    GridEvent::GroupOpen { group, open } => {
                        if let Some(&at) = grid_group_row(&shown, rows, group) {
                            toggle(&mut local, &mut events, at, Some(open));
                        }
                    }
                    GridEvent::GroupPicked { group, .. } => {
                        if let Some(&at) = grid_group_row(&shown, rows, group) {
                            toggle(&mut local, &mut events, at, None);
                        }
                    }
                    GridEvent::GroupMenu { group, .. } => {
                        menu_target = grid_group_row(&shown, rows, group).map(|&at| rows[at].key.clone());
                    }
                    _ => {}
                }
            }
            for event in pass.seen {
                match event {
                    Seen::Select(key) => {
                        if row_of(&key).is_some_and(|at| !rows[at].is_group()) {
                            events.push(PresetEvent::Select { key, mods });
                        }
                    }
                    Seen::Activate(key) => match row_of(&key) {
                        Some(at) if rows[at].is_group() => toggle(&mut local, &mut events, at, None),
                        Some(_) => events.push(PresetEvent::Activate { key, mods }),
                        None => {}
                    },
                    Seen::Open(change) => {
                        local.apply_open(rows, &change, default_open);
                        events.push(PresetEvent::Open(change));
                    }
                    Seen::Menu(key) => menu_target = Some(key),
                    Seen::Pointer(key) => hovered = row_of(&key).filter(|&at| !rows[at].is_group()).map(|_| key),
                    Seen::Reorder(edit) => {
                        let moved = |mv: ReorderMove<K>| {
                            let to = full_position(rows, &displayed, &mv);
                            ReorderMove { to, ..mv }
                        };
                        events.push(PresetEvent::Reorder(edit.map(moved)));
                    }
                    Seen::Rename(edit) => {
                        let ended = matches!(edit.phase(), Some(GesturePhase::Commit) | None);
                        if let (true, Some(rename)) = (ended, edit.value()) {
                            let text = rename.name.trim();
                            let changed = row_of(&rename.key).is_some_and(|at| rows[at].label != text);
                            if changed && !text.is_empty() {
                                events.push(PresetEvent::Rename { key: rename.key.clone(), text: text.to_owned() });
                            }
                        }
                    }
                }
            }

            if row_menu && let Some(target) = menu_target.or_else(|| selected.clone()).filter(|key| row_of(key).is_some()) {
                let at = row_of(&target).unwrap_or_default();
                let group = rows[at].is_group();
                let deletable = can_delete.as_ref().is_none_or(|test| test.passes(&target));
                let renamable = rename_ok(&target) && (!group || view == PresetView::List);
                let movable = !group && can_move.as_ref().is_none_or(|test| test.passes(&target));
                let home = rows[..at].iter().rposition(|row| row.is_group() && row.depth < rows[at].depth);
                let destinations: Vec<usize> = (0..rows.len()).filter(|&to| rows[to].is_group() && Some(to) != home).collect();
                let mut ran: Vec<PresetEvent<K>> = Vec::new();
                let _ = Menu::context(words.menu.clone(), &response).show(ui, |ui, _| {
                    let (rename, delete) = if group {
                        (PresetItemCommands::RenameGroup, PresetItemCommands::DeleteGroup)
                    } else {
                        (PresetItemCommands::Rename, PresetItemCommands::Delete)
                    };
                    if MenuItem::command(rename).enabled(renamable).show(ui).ran {
                        request_rename(ui.ctx(), list_id, &target);
                    }
                    if movable && !destinations.is_empty() {
                        let _ = Menu::submenu_row(ui, words.move_to.clone(), |ui, _| {
                            for &to in &destinations {
                                if MenuItem::new(Label::from(rows[to].label.clone())).show(ui).ran {
                                    ran.push(PresetEvent::MoveTo { key: target.clone(), group: rows[to].key.clone() });
                                }
                            }
                        });
                    }
                    if MenuItem::command(delete).enabled(deletable).show(ui).ran {
                        ran.push(PresetEvent::Delete(target.clone()));
                    }
                    if !extra_items.is_empty() {
                        Divider::horizontal().show(ui);
                    }
                    for entry in &extra_items {
                        if entry.is_separator() {
                            Divider::horizontal().show(ui);
                        } else if let Some(value) = entry.value() {
                            let row = MenuItem::new(entry.label().clone()).enabled(entry.is_enabled());
                            if row.show(ui).ran {
                                ran.push(PresetEvent::Menu { target: target.clone(), command: value.clone() });
                            }
                        }
                    }
                });
                events.extend(ran);
            }

            let mut shared = session.borrow_mut();
            shared.local = local;
            shared.events = events;
            if !has_footer {
                shared.commit(cx, rows, &name);
            }
            (response, hovered)
        };

        let footer_region = |ui: &mut Ui, cx: &mut BodyCx<'_>| {
            let Some(setup) = footer.as_ref() else { return };
            let mut shared = session.borrow_mut();
            let mut zoom = shared.local.zoom;
            let count = rows.iter().filter(|row| !row.is_group()).count();
            let bar = (setup.0)(PresetFooter::new(Bind::bound(&mut zoom)).range(thumb_range).count(count));
            let events = bar.show(ui).events;
            shared.local.zoom = zoom;
            shared.events.extend(events.into_iter().map(PresetEvent::Footer));
            shared.commit(cx, rows, &name);
        };

        let stack = Stack::new().fill();
        let stack = if has_footer { stack.footer(footer_region) } else { stack };
        let shown = stack.show(ui, body);
        let (response, hovered) = shown.inner.unwrap_or_else(|| (shown.output.clone(), None));
        let events = std::mem::take(&mut session.borrow_mut().events);
        let drag = crate::runtime::drag::active::<ListDrag<K>>(ui.ctx()).filter(|drag| drag.list == list_id).map(|drag| (*drag).clone());
        PresetOutput::new(response, events, drag, hovered)
    }
}

/// The row of the header `group` of the grid's plan over the shown rows.
fn grid_group_row<'s, K>(shown: &'s [usize], rows: &[PresetRow<K>], group: usize) -> Option<&'s usize> {
    shown.iter().filter(|&&at| rows[at].is_group()).nth(group)
}
