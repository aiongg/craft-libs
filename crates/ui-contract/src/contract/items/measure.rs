//! The largest item of a collection: the widest row of a content-sized collection, the cell of a collection that
//! sizes its items from their measure. Measured, recorded, kept while the items' inputs hold, and read back.

use super::rows;
use super::{ItemsHooks, key_hash};
use crate::contract::Ui;
use crate::contract::collection::RowState;
use crate::contract::run::Measuring;
use crate::contract::{Collection, Internal, ItemCx, Props, Show as _};
use crate::primitives::Id;
use crate::runtime::access::SetPosition;
use crate::runtime::env::ArenaId;
use crate::runtime::env::{Env, ItemFacts, ScrollOwner};
use crate::runtime::memory::{MemoryStage, sub};
use crate::style::{EnvKey, Flags, SurfaceKind};
use crate::style::{StyleCx, UiTokens};
use craft_behavior::layout_memory::measure_tolerance;
use craft_behavior::{ItemLayout as _, Proposal, ScrollPos, SelectionModel as _};
use craft_ui_rt::Pos2;
use craft_ui_rt::Rect;
use craft_ui_rt::Vec2;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

impl<'s, 'r, C: Collection> ItemsHooks<'s, 'r, C> {
    /// Whether the collection takes its width from its widest row: `Sizing::Content`. A row's width holds its
    /// column cells at the widths the header resolves for them.
    pub(super) fn content_sized(&self, comp: &C) -> bool {
        comp.props(Internal::new()).sizing.content_width()
    }

    /// The padding the collection's props add on every side of its rows (`ShapeHooks::pad`).
    pub(super) fn props_pad(comp: &C, tokens: &UiTokens) -> f32 {
        crate::contract::pad_of(tokens, comp.props(Internal::new()).pad)
    }

    /// The room the header and rows lay out in: `room` less the padding the props add on both sides. Step 5 adds the
    /// padding back and step 7 insets the content rect by it.
    pub(super) fn inner_room(comp: &C, tokens: &UiTokens, room: Vec2) -> Vec2 {
        (room - Vec2::splat(2.0 * Self::props_pad(comp, tokens))).max(Vec2::ZERO)
    }

    /// The width a collection that fills the width takes where it is not given one, in a measuring run under a body
    /// sized from its content: the header's least width, else its largest row's, at least the width it asks for
    /// (`Collection::ideal_width`), up to `room`. The column it sits in holds it at that width and shows it across
    /// the width the column settles on.
    pub(super) fn width_from_content(&mut self, comp: &C, at: (&mut Ui, &Env), id: Id, room: Vec2, item: Option<Vec2>, scx: &StyleCx<'_>) -> f32 {
        let rows = if self.header.has_columns() { self.header.least_width() } else { self.largest_in(comp, at, id, room).x };
        rows.max(comp.ideal_width(item, self.source.len(), scx)).min(room.x)
    }

    /// Measures the largest row now when this run's width comes from the rows, the collection `measures_items`, or a
    /// column takes its widest cell's width; the result is kept in [`RowSizes::largest`] for `items_extent`.
    pub(super) fn record_largest(&mut self, comp: &C, at: &Measuring<'_, '_>, id: Id) {
        let room = Self::inner_room(comp, &at.env.tokens, at.room);
        if self.source.is_empty() || (room.x.is_finite() && !self.content_sized(comp) && !comp.measures_items() && !self.header.has_content_columns())
        {
            return;
        }
        self.header.measure(at.scx);
        at.with_ui(|ui| self.largest_in(comp, (ui, at.place), id, room));
    }

    /// The largest row in the room `room`: the kept measure while the room and the items' inputs hold, else the rows
    /// built and measured now and kept. The widest cell of each `Content` column, or its header label when wider, is
    /// kept with it and handed to the header. The caller measures the header first. `env` is the env `ui`'s children
    /// are placed under.
    pub(super) fn largest_in(&mut self, comp: &C, (ui, env): (&mut Ui, &Env), id: Id, room: Vec2) -> Vec2 {
        let inputs = self.items_inputs(comp, env, room);
        if let Some(kept) = &self.sizes.largest
            && inputs.is_some()
            && kept.inputs == inputs
        {
            self.header.read_content(&kept.content);
            return kept.took;
        }
        #[cfg(feature = "testing")]
        counts::count(&counts::WIDEST, id);
        let (largest, run, cells) = self.largest_row_in(comp, (&mut *ui, env), id);
        let columns: Vec<(usize, craft_behavior::ColumnKey)> = self.header.content_columns().map(|(at, key)| (at, key.clone())).collect();
        let mut content = Vec::with_capacity(columns.len());
        for (at, column) in columns {
            let widest = cells.iter().find(|(held, _)| *held == column).map_or(0.0, |(_, width)| *width);
            let wide = widest.max(self.header.label_width(ui, &env.view(), at));
            content.push((column, wide));
        }
        self.header.read_content(&content);
        self.sizes.largest = Some(Largest { inputs, took: largest, run, content });
        largest
    }

    /// What the items' measure reads besides the rows' content: the room, the item count, the env and what `dress`
    /// reads of the collection. `None` when the source declares no content version (rows stamp or revision): the rows
    /// may change unseen, so every run that needs the largest row measures them.
    fn items_inputs(&self, comp: &C, env: &Env, room: Vec2) -> Option<ItemsInputs> {
        let content = (comp.items().rows_stamp, self.source.revision());
        (content.0.is_some() || content.1.is_some()).then(|| ItemsInputs {
            room,
            count: self.source.len(),
            content,
            dress: comp.dress_inputs(),
            env: env.key,
        })
    }

    /// The length of the rows side by side at their ideal widths, as the kept measure of the largest row in the room
    /// `room` found them (`largest_in`); 0 before one.
    pub(super) fn run_in(&self, room: Vec2) -> f32 {
        self.sizes.largest.as_ref().filter(|kept| kept.inputs.as_ref().is_none_or(|inputs| inputs.room == room)).map_or(0.0, |kept| kept.run)
    }

    fn largest_row_in(&mut self, comp: &C, (ui, env): (&mut Ui, &Env), id: Id) -> (Vec2, f32, Vec<(craft_behavior::ColumnKey, f32)>) {
        let mut memory = MemoryStage::default();
        let cells: Vec<(craft_behavior::ColumnKey, Rect)> =
            self.header.columns(0.0).into_iter().map(|(key, width)| (key, Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 0.0)))).collect();
        let mut largest = Vec2::ZERO;
        let mut run = 0.0;
        let mut widest: Vec<(craft_behavior::ColumnKey, f32)> = Vec::new();
        for at in 0..self.source.len() {
            if self.source.section_stop(at) {
                continue;
            }
            let row = RowMeasure::new(at, Rect::ZERO, cells.clone());
            let (size, cells) = self.measure_row(comp, &mut *ui, (id, env), &mut memory, row, |measure, _, _| measure(Proposal::Ideal));
            largest = largest.max(size);
            run += size.x;
            for (column, width) in cells {
                match widest.iter_mut().find(|(held, _)| *held == column) {
                    Some((_, held)) => *held = held.max(width),
                    None => widest.push((column, width)),
                }
            }
        }
        (largest, run, widest)
    }

    /// Builds the item of `row.at` as the collection `id` builds it, with the row state `row.state` and flags
    /// `row.flags`, under `installed`, and hands `measure` a measure of it: its size at a proposal and the widths its
    /// column cells reported (a kept row that answers from its size cache reports the cells it reported when it last
    /// ran), whether the item declared a detail body, which the measure leaves out, and whether it is as wide as it
    /// measures (`rows::fits`). The item is built once however many proposals `measure` asks. Item memory goes to
    /// `memory`.
    pub(super) fn measure_row<R>(
        &mut self,
        comp: &C,
        ui: &mut Ui,
        (id, installed): (Id, &Env),
        memory: &mut MemoryStage,
        row: RowMeasure,
        measure: impl FnOnce(&mut dyn FnMut(Proposal) -> (Vec2, Vec<(craft_behavior::ColumnKey, f32)>), bool, bool) -> R,
    ) -> R {
        let RowMeasure { at, rect, cells, state, flags, tree } = row;
        let len = self.source.len();
        let key = self.source.key(at);
        let hash = key_hash(&key);
        let item_id = sub(id, ("key", hash));
        let position = self.row_position(at, cells.len(), tree);
        let columns: Vec<craft_behavior::ColumnKey> = cells.iter().map(|(key, _)| key.clone()).collect();
        let column_count = cells.len();
        let mut icx = ItemCx {
            key: &key,
            at,
            rect,
            row: state.clone(),
            depth: self.source.depth(at),
            set: position,
            row_index: at,
            row_count: len,
            column_count,
            env: installed.view(),
            id: sub(item_id, "item"),
            cells,
            grid: None,
            tip: None,
            detail: None,
            openers: Vec::new(),
            item: item_id,
            memory,
        };
        #[cfg(feature = "testing")]
        counts::count(&counts::ROW_BUILDS, item_id);
        let item = (self.item)(&key, &mut icx);
        let detail = icx.detail.is_some();
        let mut item = Props::id(comp.dress(item, &state), item_id);
        let fits = rows::fits(&item);
        let facts = ItemFacts {
            item: item_id,
            flags,
            position,
            columns,
            toggles: self.header.toggle_columns().map(|(key, _)| key).collect(),
            toggle_keys: Vec::new(),
            closed_toggles: Vec::new(),
            aligns: self.header.aligns(),
            pressed: None,
            menu_open: false,
        };
        let env = installed.derive_local(|scoped| {
            scoped.item = Some(item_id);
            scoped.row = Some(rows::row_group(item_id));
            scoped.item_facts = Some(std::sync::Arc::new(facts));
        });
        let sizes = &mut self.sizes;
        Env::run_in(ui, env, |ui| {
            measure(
                &mut |proposal| {
                    let (size, reported) = collect_cells(|| item.measure_in(ui, proposal));
                    let cells = match reported {
                        Some(cells) => {
                            sizes.cells.insert(hash, cells.clone());
                            cells
                        }
                        None => sizes.cells.get(&hash).cloned().unwrap_or_default(),
                    };
                    (size, cells)
                },
                detail,
                fits,
            )
        })
    }

    /// Sets in `layout` what the collection kept of its rows' measures (heights, auto widths, the widths of rows as
    /// wide as they measured), by key: the rows this run does not measure, out of view, are placed by them.
    pub(super) fn restore_sizes(&self, layout: &mut C::ItemLayout) {
        let sizes = &self.sizes;
        if sizes.heights.is_empty() && sizes.widths.is_empty() && sizes.fits.is_empty() {
            return;
        }
        let keys: Vec<u64> = (0..self.source.len()).map(|at| key_hash(&self.source.key(at))).collect();
        layout.measured(&|at| sizes.heights.get(&keys[at]).map(|size| size.y));
        if layout.auto_widths() {
            for (at, key) in keys.iter().enumerate() {
                if let Some(&(ideal, min)) = sizes.widths.get(key) {
                    layout.set_width(at, ideal, min);
                }
            }
        }
        for (at, key) in keys.iter().enumerate() {
            if let Some(&fit) = sizes.fits.get(key) {
                layout.set_fit(at, Some(fit));
            }
        }
    }

    /// Measures the auto rows in the view the collection takes that this run has not measured, before step 5 sizes
    /// the collection from its layout: their ideal and minimum widths in a layout of auto widths, then their heights at
    /// their widths. Each measure is kept ([`RowSizes`]) and set in `layout`, so this run places every row in view by
    /// its measure, and later runs take it without measuring again. The view is the one step 5 sizes: `width` wide, as
    /// high as the rows (capped by `max_rows`), and no higher than `room` less the header when the rows scroll in a
    /// bounded room, at the offset step 4 read. It repeats while measured rows bring other unmeasured rows into view;
    /// each row is measured at most once.
    pub(super) fn measure_ahead(&mut self, comp: &C, at: &Measuring<'_, '_>, id: Id, layout: &mut C::ItemLayout, (width, room): (f32, Vec2)) {
        if self.source.is_empty() {
            return;
        }
        let tolerance = measure_tolerance(at.with_ui(|ui| ui.pixels_per_point()));
        let props = comp.items();
        let (row, max_rows, striped) = (props.row_height.of(at.scx), props.max_rows, props.striped);
        let tree = matches!(layout.nav(), craft_behavior::NavShape::Tree);
        let known = self.known(comp, true);
        let env = Self::rows_env(comp, at.env, id, at.resolved.size, comp.props(Internal::new()).surface);
        let widths = layout.auto_widths();
        let mut memory = MemoryStage::default();
        // How many times each row was measured here: its widths once, its height again only at a width the later rows'
        // widths moved it to.
        let mut done: Vec<(usize, u8)> = Vec::new();
        loop {
            let content = layout.content_size().y as f32;
            let mut high = max_rows.map_or(content, |rows| content.min(super::hooks::rows_height(&*layout, self.source.len(), rows, row)));
            if self.scrolls && room.y.is_finite() {
                high = high.min((room.y - self.header.height).max(0.0));
            }
            let mut in_view = Vec::new();
            layout.visible(ScrollPos { x: self.offset_x, y: self.offset.max(0.0) }, Vec2::new(width, high), &mut in_view);
            in_view.sort_unstable();
            // The rows of the view this run has not measured: (row, its widths, its height).
            let todo: Vec<(usize, bool, bool)> = in_view
                .into_iter()
                .filter(|&at| layout.is_auto(at) && !self.source.section_stop(at) && done.iter().all(|&(held, times)| held != at || times < 2))
                .filter_map(|at| {
                    let key = key_hash(&self.source.key(at));
                    let wide = widths && !self.sizes.fresh.contains(&(key, Measure::Widths));
                    let row_width = layout.rect(at, ScrollPos::default()).width();
                    let high = !self.sizes.fresh.contains(&(key, Measure::Height))
                        || self.sizes.heights.get(&key).is_none_or(|size| (size.x - row_width).abs() > tolerance);
                    (wide || high).then_some((at, wide, high))
                })
                .collect();
            if todo.is_empty() {
                return;
            }
            let mut stripe: Option<(usize, usize)> = None;
            for (row_at, wide, high) in todo {
                match done.iter_mut().find(|(held, _)| *held == row_at) {
                    Some((_, times)) => *times += 1,
                    None => done.push((row_at, 1)),
                }
                let odd = striped && {
                    let run = self.stripe_run(stripe, row_at);
                    stripe = Some((row_at, run));
                    run.is_multiple_of(2) && !self.source.is_section(row_at)
                };
                let (state, flags) = self.known_row(row_at, &known, odd);
                let mut cells = Vec::new();
                layout.cells(row_at, ScrollPos::default(), &mut cells);
                let row_measure = RowMeasure { at: row_at, rect: layout.rect(row_at, ScrollPos::default()), cells, state, flags, tree };
                let key = key_hash(&self.source.key(row_at));
                let row_width = layout.rect(row_at, ScrollPos::default()).width();
                let (measured_widths, height, detail) = at.with_ui(|ui| {
                    self.measure_row(comp, ui, (id, &env), &mut memory, row_measure, |measure, detail, fits| {
                        // A row of auto widths takes its ideal width unless the fit over every row's widths shrinks it: its
                        // height is measured there, and again only where the fit settles on another width. A row with a
                        // detail body is as high as its leaf and its body, which the shown row records. A row sized to its
                        // content is as wide as it measures there.
                        let widths = wide.then(|| Vec2::new(measure(Proposal::Ideal).0.x, measure(Proposal::Min).0.x));
                        let row_width = widths.map_or(row_width, |widths| widths.x);
                        let height = (!detail && (high || wide)).then(|| {
                            let size = measure(Proposal::Width(row_width)).0;
                            (Vec2::new(row_width, size.y), fits.then_some(size.x))
                        });
                        (widths, height, detail)
                    })
                });
                if detail && let Some((_, times)) = done.iter_mut().find(|(held, _)| *held == row_at) {
                    // Its height stays unmeasured this run: once is enough.
                    *times = 2;
                }
                if let Some(widths) = measured_widths {
                    self.sizes.widths.insert(key, (widths.x, widths.y));
                    self.sizes.fresh.insert((key, Measure::Widths));
                    layout.set_width(row_at, widths.x, widths.y);
                }
                if let Some((height, fit)) = height {
                    #[cfg(feature = "testing")]
                    counts::count(&counts::ROW_MEASURES, sub(id, ("row_size", key)));
                    self.sizes.heights.insert(key, height);
                    self.sizes.fresh.insert((key, Measure::Height));
                    layout.set_height(row_at, height.y);
                    if self.sizes.keep_fit(key, fit) {
                        layout.set_fit(row_at, fit);
                    }
                }
            }
        }
    }

    /// The place of the row `at` in the collection's set, among `columns` columns, with its level in a tree.
    pub(super) fn row_position(&self, at: usize, columns: usize, tree: bool) -> SetPosition {
        let len = self.source.len();
        let position = SetPosition::new(at - self.source.section_stops_before(at)).of(len - self.source.section_stops_before(len)).columns(columns);
        if tree { position.level(self.source.depth(at) + 1) } else { position }
    }

    /// The env the rows lay out under, from the collection's env `env`: the collection's arena, the child size (else
    /// the collection's `size`), the child glyph role, and the surface the rows sit on (`own`, else the one the
    /// collection sits on). The rows lay out their own content: no line or first line of the body around reaches them.
    /// A collection that scrolls its rows is their scroll owner: a reveal in a row comes to it. The content step adds
    /// the app values its body gives (the row-hook contract on `ItemCx`).
    pub(super) fn rows_env(comp: &C, env: &Env, id: Id, size: craft_theme::ControlSize, own: Option<SurfaceKind>) -> Env {
        let (child_size, child_glyphs) = (comp.props(Internal::new()).child_size, comp.props(Internal::new()).child_glyphs);
        let owns = super::hooks::scrolls_rows(comp.items(), env);
        env.derive(|scoped| {
            if owns {
                scoped.scroll_owner = Some(ScrollOwner { id, view: false });
            }
            scoped.arena = Some(ArenaId::new(id));
            scoped.size = child_size.unwrap_or(size);
            if let Some(role) = child_glyphs {
                scoped.glyphs = role;
            }
            scoped.surface = SurfaceKind::under(own, scoped.surface);
            scoped.row_line = None;
            scoped.first_line = None;
        })
    }

    /// The state of the row `at` its measure reads, from `known` (the row-hook contract on `ItemCx`): selected, lead,
    /// renaming, current, open, striped (`striped`, which the caller runs along the rows), and its place among
    /// sub-rows and the last row; with the `SELECTED`, `EXPANDED` and `CURRENT` flags. The content step adds the
    /// pointer, gesture and focus state.
    pub(super) fn known_row(&self, at: usize, known: &Known<C::Key>, striped: bool) -> (RowState, Flags) {
        let key = self.source.key(at);
        let len = self.source.len();
        let state = RowState {
            selected: known.placed.holds_row(at),
            lead: known.lead.as_ref() == Some(&key),
            renaming: known.renaming.as_ref() == Some(&key),
            striped,
            current: known.current.as_ref() == Some(&key),
            open: self.source.has_children(at) && self.source.open(at),
            sub_row: self.source.sub_row(at),
            continued: at + 1 < len && self.source.sub_row(at + 1).is_some(),
            last: at + 1 == len,
            ..RowState::default()
        };
        let mut flags = Flags::empty();
        flags.set(Flags::SELECTED, state.selected);
        flags.set(Flags::EXPANDED, state.open);
        flags.set(Flags::CURRENT, state.current);
        (state, flags)
    }

    /// The facts `known_row` reads: the selection, lead and renaming row as step 4 read them (`at_read`), or as the
    /// pass leaves them.
    pub(super) fn known(&self, comp: &C, at_read: bool) -> Known<C::Key> {
        let current = comp.items().current.as_deref().and_then(|key| key.downcast_ref::<C::Key>()).cloned();
        if at_read {
            Known {
                placed: self.selection_at_read.placed(&*self.source),
                lead: self.lead_at_read.clone(),
                renaming: self.renaming_at_read.clone(),
                current,
            }
        } else {
            Known { placed: self.selection.placed(&*self.source), lead: self.lead.key.clone(), renaming: self.renaming.key.clone(), current }
        }
    }
}

/// What the state of a row known before the pass senses input comes from (`ItemsHooks::known_row`).
pub(super) struct Known<K> {
    pub(super) placed: craft_behavior::PlacedSelection,
    pub(super) lead: Option<K>,
    pub(super) renaming: Option<K>,
    pub(super) current: Option<K>,
}

/// A row to build and measure (`measure_row`): its position, rect and column cells, its state and flags, and whether
/// the collection is a tree.
pub(super) struct RowMeasure {
    pub(super) at: usize,
    pub(super) rect: Rect,
    pub(super) cells: Vec<(craft_behavior::ColumnKey, Rect)>,
    pub(super) state: RowState,
    pub(super) flags: Flags,
    pub(super) tree: bool,
}

impl RowMeasure {
    /// The row `at` at `rect` with `cells`, in the default state.
    pub(super) fn new(at: usize, rect: Rect, cells: Vec<(craft_behavior::ColumnKey, Rect)>) -> Self {
        RowMeasure { at, rect, cells, state: RowState::default(), flags: Flags::empty(), tree: false }
    }
}

/// The inputs a kept measure of the items serves under.
#[derive(Clone, PartialEq)]
struct ItemsInputs {
    room: Vec2,
    count: usize,
    /// The rows stamp and the source's revision.
    content: (Option<u64>, Option<u64>),
    dress: u64,
    env: EnvKey,
}

/// What a row's measure found.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Measure {
    Widths,
    Height,
}

/// The largest row's measure and what it serves under (`largest_in`).
#[derive(Clone)]
struct Largest {
    /// `None` when the source declares no content version: the measure serves no later run.
    inputs: Option<ItemsInputs>,
    took: Vec2,
    /// The rows side by side at their ideal widths.
    run: f32,
    /// The widest cell of each `Content` column, or its header label when wider.
    content: Vec<(craft_behavior::ColumnKey, f32)>,
}

/// What a collection measured of its rows, kept in its node's session memory from run to run (the read step takes
/// it, the settle step keeps it): the one record of auto row sizes, so a row out of the band keeps its place.
#[derive(Clone, Default)]
pub(super) struct RowSizes {
    /// Auto rows by key hash: the width a row was measured at and its height there.
    heights: HashMap<u64, Vec2>,
    /// Auto-width items by key hash: their ideal and minimum widths.
    widths: HashMap<u64, (f32, f32)>,
    /// The width of each row as wide as it measured, by key hash.
    fits: HashMap<u64, f32>,
    /// The widths each row's column cells reported at its last run, by key hash.
    cells: HashMap<u64, Vec<(craft_behavior::ColumnKey, f32)>>,
    largest: Option<Largest>,
    /// What this run measured, by key hash: a row is measured once per run.
    fresh: HashSet<(u64, Measure)>,
}

impl RowSizes {
    /// The record the collection `id` kept in `stage`, ready for a new run.
    pub(super) fn read(stage: &mut MemoryStage, id: Id) -> RowSizes {
        let mut sizes = stage.session::<RowSizes>(sub(id, "row_sizes")).clone();
        sizes.fresh.clear();
        sizes
    }

    /// Keeps the record for the next run of the collection `id`.
    pub(super) fn keep(self, stage: &mut MemoryStage, id: Id) {
        *stage.session::<RowSizes>(sub(id, "row_sizes")) = self;
    }

    /// Keeps that the row `key` is `fit` wide, the width it measured, or no longer as wide as it measured. Returns
    /// whether that differs from what was kept, which this run's layout took: only then does the caller set it there.
    fn keep_fit(&mut self, key: u64, fit: Option<f32>) -> bool {
        if self.fits.get(&key).copied() == fit {
            return false;
        }
        match fit {
            Some(fit) => self.fits.insert(key, fit),
            None => self.fits.remove(&key),
        };
        true
    }

    /// Keeps the height `size.y` an auto row measured at width `size.x` while shown (its leaf and detail body),
    /// so the runs that place it out of the band place it so.
    pub(super) fn keep_shown_height(&mut self, key: u64, size: Vec2) {
        self.heights.insert(key, size);
        self.fresh.insert((key, Measure::Height));
    }

    /// Keeps the auto-width row `key`'s ideal and minimum widths measured while shown.
    pub(super) fn keep_shown_widths(&mut self, key: u64, widths: Vec2) {
        self.widths.insert(key, (widths.x, widths.y));
        self.fresh.insert((key, Measure::Widths));
    }

    /// Keeps whether the row `key` is as wide as it measured; `true` when that moved.
    pub(super) fn keep_shown_fit(&mut self, key: u64, fit: Option<f32>) -> bool {
        self.keep_fit(key, fit)
    }

    /// The largest row's size the last measure found, while one is kept.
    pub(super) fn largest(&self) -> Option<Vec2> {
        self.largest.as_ref().map(|kept| kept.took)
    }

    /// The cells the row `key` reported at its last measure.
    pub(super) fn cells_of(&self, key: u64) -> Option<&[(craft_behavior::ColumnKey, f32)]> {
        self.cells.get(&key).map(Vec::as_slice)
    }

    /// Keeps the cells the row `key` reported while shown.
    pub(super) fn keep_cells(&mut self, key: u64, cells: Vec<(craft_behavior::ColumnKey, f32)>) {
        self.cells.insert(key, cells);
    }
}

thread_local! {
    /// The open cell collectors, innermost last: a row measure collects the widths its cells report.
    static CELLS: RefCell<Vec<Vec<(craft_behavior::ColumnKey, f32)>>> = const { RefCell::new(Vec::new()) };
}

/// Runs `measure`, collecting the widths its cells report (`report_cell_width`); `None` when none reported, as when
/// the measured item answered from its size cache.
pub(super) fn collect_cells<R>(measure: impl FnOnce() -> R) -> (R, Option<Vec<(craft_behavior::ColumnKey, f32)>>) {
    CELLS.with(|cells| cells.borrow_mut().push(Vec::new()));
    struct Pop;
    impl Drop for Pop {
        fn drop(&mut self) {
            CELLS.with(|cells| cells.borrow_mut().pop());
        }
    }
    let pop = Pop;
    let out = measure();
    let cells = CELLS.with(|cells| std::mem::take(cells.borrow_mut().last_mut().expect("the collector this call pushed")));
    drop(pop);
    (out, (!cells.is_empty()).then_some(cells))
}

/// Reports that a cell of column `column` takes `width` in the row being measured (`MeasureCx::cell_width`): the
/// collection measuring the row takes its column's widest cell from it. Nothing outside a row measure.
pub(crate) fn report_cell_width(_ui: &mut Ui, column: &craft_behavior::ColumnKey, width: f32) {
    CELLS.with(|cells| {
        if let Some(open) = cells.borrow_mut().last_mut() {
            match open.iter_mut().find(|(held, _)| held == column) {
                Some((_, held)) => *held = held.max(width),
                None => open.push((column.clone(), width)),
            }
        }
    });
}

/// Count trials of the items' measure (`testing`): what a run built and measured of the rows, by id, taken by the
/// trial that reads it.
#[cfg(feature = "testing")]
pub(crate) mod counts {
    use crate::primitives::Id;
    use std::cell::RefCell;
    use std::thread::LocalKey;

    type Log = RefCell<Vec<(Id, u32)>>;

    thread_local! {
        pub(super) static WIDEST: Log = const { RefCell::new(Vec::new()) };
        pub(super) static ROW_BUILDS: Log = const { RefCell::new(Vec::new()) };
        pub(super) static ROW_MEASURES: Log = const { RefCell::new(Vec::new()) };
    }

    pub(super) fn count(log: &'static LocalKey<Log>, id: Id) {
        log.with(|log| {
            let mut log = log.borrow_mut();
            match log.iter_mut().find(|(held, _)| *held == id) {
                Some((_, n)) => *n += 1,
                None => log.push((id, 1)),
            }
        });
    }

    /// The collections that measured their largest row since the last take, with how many times.
    #[doc(hidden)]
    pub fn take_widest_runs() -> Vec<(Id, u32)> {
        WIDEST.with(|log| std::mem::take(&mut *log.borrow_mut()))
    }

    /// The rows built to be measured since the last take, by item id, with how many times.
    #[doc(hidden)]
    pub fn take_row_builds() -> Vec<(Id, u32)> {
        ROW_BUILDS.with(|log| std::mem::take(&mut *log.borrow_mut()))
    }

    /// The auto rows whose height was measured since the last take, with how many times.
    #[doc(hidden)]
    pub fn take_row_measures() -> Vec<(Id, u32)> {
        ROW_MEASURES.with(|log| std::mem::take(&mut *log.borrow_mut()))
    }
}
